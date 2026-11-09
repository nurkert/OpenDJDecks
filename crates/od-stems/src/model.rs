//! The separation model: identity, what is on disk, and the download.
//!
//! Nothing here runs by itself: [`ModelStore::fetch`] only runs when the
//! user asked for the model (the dialog and Preferences in `od-ui`, or
//! [`crate::StemWorkerOptions::fetch_model`]).

use std::path::PathBuf;
#[cfg(feature = "separation")]
use std::path::Path;
#[cfg(feature = "separation")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "separation")]
use crate::Error;

/// A downloadable model file.
#[derive(Clone, Copy, Debug)]
pub struct ModelInfo {
    /// File name under `data_dir()/models/`.
    pub file_name: &'static str,
    pub url: &'static str,
    /// Lower-case hex SHA-256 of the file.
    pub sha256: &'static str,
    pub size: u64,
    pub license: &'static str,
}

/// HT-Demucs v4 (`htdemucs`, 4 sources), ONNX export with fp16-stored
/// weights, from <https://huggingface.co/StemSplitio/htdemucs-onnx>
/// (revision `d54ed9eb60e258ea82131c6ee14578628816456a`). Weights and code
/// © Meta Platforms, Inc., MIT License (facebookresearch/demucs); ONNX
/// export by StemSplit, MIT License.
pub const MODEL: ModelInfo = ModelInfo {
    file_name: "htdemucs_fp16weights.onnx",
    url: "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/d54ed9eb60e258ea82131c6ee14578628816456a/htdemucs_fp16weights.onnx",
    sha256: "d05c269d0178d2a72ad484b10b11dd370193fc923201c3b27a99f848745db70a",
    size: 165_612_636,
    license: "MIT (Demucs © Meta Platforms, Inc.; ONNX export © StemSplit)",
};

/// Free space kept on top of the model when checking the disk up front.
#[cfg(feature = "separation")]
const SPARE_BYTES: u64 = 16 * 1024 * 1024;

/// `od_library::data_dir()/models`.
pub fn models_dir() -> PathBuf {
    od_library::data_dir().join("models")
}

/// Where the model lives once fetched.
pub fn model_path() -> PathBuf {
    models_dir().join(MODEL.file_name)
}

/// Whether the model file is present with the expected size (use
/// [`ModelStore::status`] to tell a checked model from an unchecked one).
pub fn model_available() -> bool {
    model_status().usable()
}

/// What is on disk for the default model (cheap: nothing is hashed).
pub fn model_status() -> ModelStatus {
    ModelStore::open_default().status()
}

/// Whether this build can separate at all (Intel Macs have no ONNX Runtime
/// binaries; Lite builds leave it out).
pub fn separation_supported() -> bool {
    cfg!(onnx)
}

/// What is on disk for a model. Only metadata is read, so it is cheap
/// enough for the UI thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelStatus {
    /// No model, no partial download.
    Missing,
    /// A download that has not finished; resumes on the next fetch.
    Partial { bytes: u64 },
    /// Right size, but nothing records that its checksum was verified
    /// (a file from an older version, or one placed by hand).
    Unverified,
    /// Complete, and its SHA-256 was verified.
    Installed,
    /// A model file of the wrong size or with the wrong checksum.
    Damaged { bytes: u64 },
}

impl ModelStatus {
    /// Whether separation may start (an unverified model is checked on the
    /// worker before it is loaded).
    pub fn usable(self) -> bool {
        matches!(self, Self::Installed | Self::Unverified)
    }
}

/// Why a download did not finish, in the terms the user is told about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FetchFailure {
    #[error("could not reach the download server")]
    NoNetwork,
    #[error("the download server answered with status {0}")]
    Server(u16),
    #[error("not enough free disk space ({needed} bytes needed)")]
    DiskFull { needed: u64 },
    #[error("the downloaded file failed its checksum")]
    Checksum,
    #[error("the connection was lost before the download finished")]
    Interrupted,
    #[error("the model could not be written to disk")]
    Storage,
}

/// The model file, its partial download and its checks. Tests point it at
/// a temporary folder and a mock HTTP layer.
pub struct ModelStore {
    dir: PathBuf,
    info: ModelInfo,
    #[cfg(feature = "separation")]
    http: Option<Box<dyn od_update::Http>>,
    #[cfg(feature = "separation")]
    free_space: Option<FreeSpace>,
}

/// Free bytes on the volume of a path; `None` when that cannot be told.
#[cfg(feature = "separation")]
pub type FreeSpace = Box<dyn Fn(&Path) -> Option<u64> + Send + Sync>;

impl ModelStore {
    pub fn new(dir: impl Into<PathBuf>, info: ModelInfo) -> Self {
        Self {
            dir: dir.into(),
            info,
            #[cfg(feature = "separation")]
            http: None,
            #[cfg(feature = "separation")]
            free_space: None,
        }
    }

    /// The model of this app in `data_dir()/models`.
    pub fn open_default() -> Self {
        Self::new(models_dir(), MODEL)
    }

    #[cfg(feature = "separation")]
    pub fn with_http(mut self, http: Box<dyn od_update::Http>) -> Self {
        self.http = Some(http);
        self
    }

    #[cfg(feature = "separation")]
    pub fn with_free_space(mut self, free_space: FreeSpace) -> Self {
        self.free_space = Some(free_space);
        self
    }

    pub fn info(&self) -> ModelInfo {
        self.info
    }

    pub fn model_path(&self) -> PathBuf {
        self.dir.join(self.info.file_name)
    }

    fn part_path(&self) -> PathBuf {
        self.dir.join(format!("{}.part", self.info.file_name))
    }

    fn stamp_path(&self) -> PathBuf {
        self.dir.join(format!("{}.verified", self.info.file_name))
    }

    /// What is on disk; cheap (see [`ModelStatus`]).
    pub fn status(&self) -> ModelStatus {
        let len = |p: PathBuf| std::fs::metadata(p).ok().map(|m| m.len());
        match len(self.model_path()) {
            Some(n) if n == self.info.size => {
                if self.stamp_matches() {
                    ModelStatus::Installed
                } else {
                    ModelStatus::Unverified
                }
            }
            Some(bytes) => ModelStatus::Damaged { bytes },
            None => match len(self.part_path()) {
                Some(bytes) if bytes > 0 => ModelStatus::Partial { bytes },
                _ => ModelStatus::Missing,
            },
        }
    }

    /// Bytes the model and its leftovers take on disk.
    pub fn size_on_disk(&self) -> u64 {
        self.owned_files()
            .into_iter()
            .filter_map(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .sum()
    }

    /// The files in the model folder that belong to the model: the model,
    /// its partial download and check mark, and stale copies from earlier
    /// versions (`*.onnx`, `*.part`).
    fn owned_files(&self) -> Vec<PathBuf> {
        let Ok(dir) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        dir.flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                name.starts_with(self.info.file_name)
                    || name.ends_with(".onnx")
                    || name.ends_with(".part")
            })
            .collect()
    }

    /// The model file and every partial, damaged or stale leftover. Stems
    /// that were already separated are kept elsewhere and stay.
    pub fn remove_all(&self) -> std::io::Result<()> {
        let mut first = None;
        for f in self.owned_files() {
            if let Err(e) = std::fs::remove_file(&f) {
                first.get_or_insert(e);
            }
        }
        first.map_or(Ok(()), Err)
    }

    /// `(length, modification time)` of the model file, as the check mark
    /// records them.
    fn fingerprint(&self) -> Option<String> {
        let m = std::fs::metadata(self.model_path()).ok()?;
        let at = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
        Some(format!("{} {} {}", self.info.sha256, m.len(), at.as_nanos()))
    }

    fn stamp_matches(&self) -> bool {
        let stamp = std::fs::read_to_string(self.stamp_path()).unwrap_or_default();
        !stamp.is_empty() && self.fingerprint().as_deref() == Some(stamp.trim())
    }

    #[cfg(feature = "separation")]
    fn write_stamp(&self) {
        if let Some(print) = self.fingerprint()
            && let Err(e) = std::fs::write(self.stamp_path(), print)
        {
            // Costs one more hash on the next start, nothing else.
            tracing::warn!("could not record the model check: {e}");
        }
    }

    #[cfg(feature = "separation")]
    fn hash_file(path: &Path) -> std::io::Result<String> {
        use sha2::{Digest, Sha256};
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hex(&hasher.finalize()))
    }

    /// Checks the model file against its SHA-256 (slow: run it in the
    /// background) and records the result for the next start.
    #[cfg(feature = "separation")]
    pub fn verify(&self) -> Result<ModelStatus, Error> {
        let path = self.model_path();
        let bytes = match std::fs::metadata(&path) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(self.status()),
            Err(e) => return Err(e.into()),
        };
        if bytes == self.info.size && Self::hash_file(&path)? == self.info.sha256 {
            self.write_stamp();
            return Ok(ModelStatus::Installed);
        }
        Ok(ModelStatus::Damaged { bytes })
    }

    /// Downloads the model unless it is already there and verified. Resumes
    /// a partial download (`<file>.part`); `progress` receives (bytes done,
    /// bytes total). A file that fails its checksum is deleted. Free space
    /// is checked before anything is requested.
    #[cfg(feature = "separation")]
    pub fn fetch(
        &self,
        progress: &mut dyn FnMut(u64, u64),
        cancel: &AtomicBool,
    ) -> Result<PathBuf, Error> {
        use sha2::{Digest, Sha256};
        use std::io::{Read, Write};

        let path = self.model_path();
        let size = self.info.size;
        // Complete but unchecked: hash it instead of downloading it again.
        if self.status() == ModelStatus::Unverified && self.verify()?.usable() {
            progress(size, size);
            return Ok(path);
        }
        match self.status() {
            ModelStatus::Installed => {
                progress(size, size);
                return Ok(path);
            }
            // A damaged file is replaced, never used.
            ModelStatus::Unverified | ModelStatus::Damaged { .. } => {
                std::fs::remove_file(&path).map_err(storage(size))?;
            }
            ModelStatus::Missing | ModelStatus::Partial { .. } => {}
        }
        std::fs::create_dir_all(&self.dir).map_err(storage(size))?;
        let tmp = self.part_path();
        let mut done = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
        if done > size {
            std::fs::remove_file(&tmp).map_err(storage(size))?;
            done = 0;
        }
        // Finished earlier but not moved into place (the app closed in between).
        if done == size {
            if Self::hash_file(&tmp).map_err(storage(size))? == self.info.sha256 {
                return self.install_part(&tmp, &path);
            }
            std::fs::remove_file(&tmp).map_err(storage(size))?;
            done = 0;
        }
        let needed = size - done + SPARE_BYTES;
        if self.free_bytes().is_some_and(|f| f < needed) {
            return Err(Error::Fetch(FetchFailure::DiskFull { needed }));
        }
        // Resume: what is already there goes into the checksum first.
        let mut hasher = Sha256::new();
        if done > 0 {
            let mut f = std::fs::File::open(&tmp).map_err(storage(size))?;
            let mut buf = vec![0u8; 1 << 20];
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err(Error::Cancelled);
                }
                let n = f.read(&mut buf).map_err(storage(size))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
        }
        let default_http;
        let http: &dyn od_update::Http = match &self.http {
            Some(h) => h.as_ref(),
            None => {
                default_http = od_update::UreqHttp::new(env!("CARGO_PKG_VERSION"));
                &default_http
            }
        };
        let unreachable = |_| Error::Fetch(FetchFailure::NoNetwork);
        let mut resp = http
            .get(self.info.url, (done > 0).then_some(done))
            .map_err(unreachable)?;
        // The partial file no longer matches what the server has.
        if resp.status == 416 && done > 0 {
            std::fs::remove_file(&tmp).map_err(storage(size))?;
            done = 0;
            hasher = Sha256::new();
            resp = http.get(self.info.url, None).map_err(unreachable)?;
        }
        let append = match resp.status {
            206 if done > 0 => true,
            200 => {
                hasher = Sha256::new();
                done = 0;
                false
            }
            s => return Err(Error::Fetch(FetchFailure::Server(s))),
        };
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(&tmp)
            .map_err(storage(size - done))?;
        let mut file = std::io::BufWriter::new(file);
        let mut reader = resp.body;
        let mut buf = vec![0u8; 1 << 16];
        progress(done, size);
        loop {
            if cancel.load(Ordering::Relaxed) {
                // Keep the .part file: the next call resumes.
                file.flush().map_err(storage(size - done))?;
                return Err(Error::Cancelled);
            }
            let n = match reader.read(&mut buf) {
                Ok(n) => n,
                Err(_) => {
                    file.flush().map_err(storage(size - done))?;
                    return Err(Error::Fetch(FetchFailure::Interrupted));
                }
            };
            if n == 0 {
                break;
            }
            if done + n as u64 > size {
                // More than the model can be: not this file.
                drop(file);
                let _ = std::fs::remove_file(&tmp);
                return Err(Error::Fetch(FetchFailure::Checksum));
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n]).map_err(storage(size - done))?;
            done += n as u64;
            progress(done, size);
        }
        file.into_inner()
            .map_err(|e| e.into_error())
            .and_then(|f| f.sync_all())
            .map_err(storage(size - done))?;
        if done < size {
            // The partial file stays: the next call resumes.
            return Err(Error::Fetch(FetchFailure::Interrupted));
        }
        if hex(&hasher.finalize()) != self.info.sha256 {
            let _ = std::fs::remove_file(&tmp);
            return Err(Error::Fetch(FetchFailure::Checksum));
        }
        self.install_part(&tmp, &path)
    }

    #[cfg(feature = "separation")]
    fn install_part(&self, tmp: &Path, path: &Path) -> Result<PathBuf, Error> {
        std::fs::rename(tmp, path).map_err(storage(self.info.size))?;
        self.write_stamp();
        Ok(path.to_path_buf())
    }

    #[cfg(feature = "separation")]
    fn free_bytes(&self) -> Option<u64> {
        match &self.free_space {
            Some(f) => f(&self.dir),
            None => od_devices::volumes::space(&self.dir).map(|(_, free)| free),
        }
    }
}

/// A file error while writing the model: a full disk is its own case.
#[cfg(feature = "separation")]
fn storage(needed: u64) -> impl Fn(std::io::Error) -> Error {
    move |e| {
        if e.kind() == std::io::ErrorKind::StorageFull {
            Error::Fetch(FetchFailure::DiskFull { needed })
        } else {
            tracing::warn!("writing the model failed: {e}");
            Error::Fetch(FetchFailure::Storage)
        }
    }
}

#[cfg(feature = "separation")]
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Downloads the default model into [`models_dir`] unless already present,
/// verifying its SHA-256; see [`ModelStore::fetch`]. Alternatively place the
/// file there manually ([`MODEL`] has URL and hash).
#[cfg(feature = "separation")]
pub fn fetch_model(
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<PathBuf, Error> {
    ModelStore::open_default().fetch(progress, cancel)
}
