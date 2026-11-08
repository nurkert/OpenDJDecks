//! Discovery and ejection of mounted removable volumes.
//!
//! Polling based: on macOS mounts under `/Volumes`, on Linux the mount
//! table filtered to removable/USB block devices, on Windows the drive
//! letters whose drive is removable or sits on a USB / SD bus.

use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    pub mount_point: PathBuf,
    pub name: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    /// Block device (Linux), BSD name (macOS) or drive (`E:`, Windows), used
    /// for ejecting.
    pub device_node: Option<String>,
}

/// Total and free bytes of the file system containing `path`.
#[cfg(unix)]
pub fn space(path: &Path) -> Option<(u64, u64)> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` is a valid NUL-terminated path and `st` a valid out pointer.
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let frsize = st.f_frsize as u64;
    Some((st.f_blocks as u64 * frsize, st.f_bavail as u64 * frsize))
}

/// Total and free bytes of the file system containing `path`.
#[cfg(windows)]
pub fn space(path: &Path) -> Option<(u64, u64)> {
    windows::space(path)
}

/// Mounted removable volumes.
pub fn list() -> Vec<Volume> {
    if let Some(dir) = od_core::env::var_os("VOLUMES_DIR") {
        // Test hook: every directory below is treated as a mounted stick.
        return list_dir_as_volumes(Path::new(&dir));
    }
    #[cfg(windows)]
    {
        windows::list()
    }
    #[cfg(not(windows))]
    {
        if cfg!(target_os = "macos") {
            list_macos()
        } else {
            list_linux()
        }
    }
}

fn list_dir_as_volumes(dir: &Path) -> Vec<Volume> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut out: Vec<Volume> = read
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let p = e.path();
            let (total, free) = space(&p).unwrap_or((0, 0));
            Volume {
                name: e.file_name().to_string_lossy().into_owned(),
                mount_point: p,
                fs_type: "test".into(),
                total_bytes: total,
                free_bytes: free,
                device_node: None,
            }
        })
        .collect();
    out.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    out
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn list_macos() -> Vec<Volume> {
    let Ok(read) = std::fs::read_dir("/Volumes") else {
        return vec![];
    };
    let mut out = Vec::new();
    for e in read.flatten() {
        let p = e.path();
        // The boot volume appears as a symlink to "/".
        if std::fs::symlink_metadata(&p)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(true)
        {
            continue;
        }
        let Some(info) = macos_disk_info(&p) else {
            continue;
        };
        if !info.external {
            continue;
        }
        let (total, free) = space(&p).unwrap_or((0, 0));
        out.push(Volume {
            name: e.file_name().to_string_lossy().into_owned(),
            mount_point: p,
            fs_type: info.fs_type,
            total_bytes: total,
            free_bytes: free,
            device_node: info.device,
        });
    }
    out.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    out
}

struct MacDiskInfo {
    external: bool,
    fs_type: String,
    device: Option<String>,
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn macos_disk_info(mount: &Path) -> Option<MacDiskInfo> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (bool, String, Option<String>)>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some((external, fs, dev)) = cache.lock().ok()?.get(mount).cloned() {
        return Some(MacDiskInfo {
            external,
            fs_type: fs,
            device: dev,
        });
    }
    let out = std::process::Command::new("diskutil")
        .arg("info")
        .arg(mount)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |name: &str| {
        text.lines().find_map(|l| {
            l.trim()
                .strip_prefix(name)
                .map(|v| v.trim().trim_start_matches(':').trim().to_string())
        })
    };
    let location = field("Device Location").unwrap_or_default();
    let removable = field("Removable Media").unwrap_or_default();
    let protocol = field("Protocol").unwrap_or_default();
    let read_only = field("Volume Read-Only")
        .or_else(|| field("Media Read-Only"))
        .unwrap_or_default();
    // Disk images, network volumes and read-only media are never sticks.
    let excluded = protocol.eq_ignore_ascii_case("disk image")
        || protocol.eq_ignore_ascii_case("virtual interface")
        || read_only.to_ascii_lowercase().starts_with("yes")
        || mount.join("Backups.backupdb").exists()
        || mount.join(".timemachine").exists();
    let external = !excluded
        && (location.eq_ignore_ascii_case("external")
            || removable.eq_ignore_ascii_case("removable")
            || protocol.eq_ignore_ascii_case("usb")
            || protocol.eq_ignore_ascii_case("secure digital"));
    let fs_type = field("Type (Bundle)").unwrap_or_default();
    let device = field("Device Identifier");
    cache.lock().ok()?.insert(
        mount.to_path_buf(),
        (external, fs_type.clone(), device.clone()),
    );
    Some(MacDiskInfo {
        external,
        fs_type,
        device,
    })
}

/// Unescapes the octal escapes used in /proc/mounts (`\040` = space).
#[cfg_attr(windows, allow(dead_code))]
fn unescape_mount(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 4], 8) {
                out.push(v);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg_attr(windows, allow(dead_code))]
fn linux_is_removable(device: &str) -> bool {
    let name = Path::new(device)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let sys = Path::new("/sys/class/block").join(&name);
    let Ok(real) = std::fs::canonicalize(&sys) else {
        return false;
    };
    if real.to_string_lossy().contains("/usb") {
        return true;
    }
    // Partition: check the parent disk's removable flag.
    let disk = real.parent().map(|p| p.join("removable"));
    let own = real.join("removable");
    [Some(own), disk]
        .into_iter()
        .flatten()
        .any(|p| std::fs::read_to_string(p).is_ok_and(|v| v.trim() == "1"))
}

#[cfg_attr(windows, allow(dead_code))]
fn list_linux() -> Vec<Volume> {
    let Ok(mounts) = std::fs::read_to_string("/proc/self/mounts") else {
        return vec![];
    };
    let mut out = Vec::new();
    for line in mounts.lines() {
        let mut parts = line.split_whitespace();
        let (Some(dev), Some(mp), Some(fs)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if !dev.starts_with("/dev/") {
            continue;
        }
        let mp = unescape_mount(mp);
        if !(mp.starts_with("/media/") || mp.starts_with("/run/media/") || mp.starts_with("/mnt/"))
        {
            continue;
        }
        if !linux_is_removable(dev) {
            continue;
        }
        let p = PathBuf::from(&mp);
        let (total, free) = space(&p).unwrap_or((0, 0));
        out.push(Volume {
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| mp.clone()),
            mount_point: p,
            fs_type: fs.to_string(),
            total_bytes: total,
            free_bytes: free,
            device_node: Some(dev.to_string()),
        });
    }
    out.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    out
}

#[derive(Debug, thiserror::Error)]
#[error("eject failed: {0}")]
pub struct EjectError(pub String);

/// Unmounts and ejects a volume.
pub fn eject(volume: &Volume) -> Result<(), EjectError> {
    if od_core::env::var_os("VOLUMES_DIR").is_some() {
        return Ok(());
    }
    #[cfg(windows)]
    {
        windows::eject(volume).map_err(EjectError)
    }
    #[cfg(not(windows))]
    {
        let run = |cmd: &mut Command| -> Result<(), EjectError> {
            let out = cmd.output().map_err(|e| EjectError(e.to_string()))?;
            if out.status.success() {
                Ok(())
            } else {
                Err(EjectError(
                    String::from_utf8_lossy(&out.stderr).trim().to_string(),
                ))
            }
        };
        if cfg!(target_os = "macos") {
            run(Command::new("diskutil")
                .arg("eject")
                .arg(&volume.mount_point))
        } else if let Some(dev) = &volume.device_node {
            run(Command::new("udisksctl").args(["unmount", "-b", dev]))
                .or_else(|_| run(Command::new("umount").arg(&volume.mount_point)))?;
            // Powering off is best effort (not all devices support it).
            let _ = run(Command::new("udisksctl").args(["power-off", "-b", dev]));
            Ok(())
        } else {
            run(Command::new("umount").arg(&volume.mount_point))
        }
    }
}

/// Windows: drive letters, `GetDriveTypeW` and the storage bus type.
#[cfg(windows)]
mod windows {
    use super::Volume;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        BusTypeMmc, BusTypeSd, BusTypeUsb, CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FlushFileBuffers, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives,
        GetVolumeInformationW, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_STORAGE_EJECT_MEDIA,
        IOCTL_STORAGE_MEDIA_REMOVAL, IOCTL_STORAGE_QUERY_PROPERTY, PREVENT_MEDIA_REMOVAL,
        PropertyStandardQuery, STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
        StorageDeviceProperty,
    };
    use windows_sys::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE};

    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        std::ffi::OsString::from_wide(&buf[..end])
            .to_string_lossy()
            .into_owned()
    }

    pub fn space(path: &Path) -> Option<(u64, u64)> {
        let p = wide(path.as_os_str());
        let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
        // SAFETY: `p` is NUL-terminated, the out pointers are valid.
        let ok = unsafe { GetDiskFreeSpaceExW(p.as_ptr(), &mut avail, &mut total, &mut free) };
        (ok != 0).then_some((total, avail))
    }

    /// An open `\\.\X:` volume handle, closed on drop.
    struct Device(HANDLE);

    impl Device {
        fn open(letter: char, access: u32) -> Option<Self> {
            let p = wide(std::ffi::OsStr::new(&format!("\\\\.\\{letter}:")));
            // SAFETY: valid NUL-terminated path; no security attributes or template.
            let h = unsafe {
                CreateFileW(
                    p.as_ptr(),
                    access,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            (h != INVALID_HANDLE_VALUE && !h.is_null()).then_some(Device(h))
        }

        fn ioctl(&self, code: u32, input: &[u8], output: &mut [u8]) -> bool {
            let mut returned = 0u32;
            // SAFETY: the buffers are valid for their lengths; synchronous call.
            unsafe {
                DeviceIoControl(
                    self.0,
                    code,
                    if input.is_empty() {
                        std::ptr::null()
                    } else {
                        input.as_ptr().cast()
                    },
                    input.len() as u32,
                    if output.is_empty() {
                        std::ptr::null_mut()
                    } else {
                        output.as_mut_ptr().cast()
                    },
                    output.len() as u32,
                    &mut returned,
                    std::ptr::null_mut(),
                ) != 0
            }
        }
    }

    impl Drop for Device {
        fn drop(&mut self) {
            // SAFETY: the handle came from CreateFileW and is closed once.
            unsafe { CloseHandle(self.0) };
        }
    }

    fn as_bytes<T>(v: &T) -> &[u8] {
        // SAFETY: plain-old-data Win32 structs, read as bytes.
        unsafe { std::slice::from_raw_parts((v as *const T).cast(), std::mem::size_of::<T>()) }
    }

    /// True when the drive sits on a USB, SD or MMC bus (external SSDs and
    /// USB hard disks report themselves as fixed drives).
    fn on_external_bus(letter: char) -> bool {
        let Some(dev) = Device::open(letter, 0) else {
            return false;
        };
        let query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut out = [0u8; 1024];
        if !dev.ioctl(IOCTL_STORAGE_QUERY_PROPERTY, as_bytes(&query), &mut out) {
            return false;
        }
        // SAFETY: the buffer is larger than the descriptor and was filled by
        // the driver; read unaligned.
        let desc: STORAGE_DEVICE_DESCRIPTOR =
            unsafe { std::ptr::read_unaligned(out.as_ptr().cast()) };
        [BusTypeUsb, BusTypeSd, BusTypeMmc].contains(&desc.BusType)
    }

    fn system_drive() -> Option<char> {
        std::env::var("SystemDrive")
            .ok()
            .and_then(|s| s.chars().next())
            .map(|c| c.to_ascii_uppercase())
    }

    pub fn list() -> Vec<Volume> {
        // SAFETY: no arguments.
        let mask = unsafe { GetLogicalDrives() };
        let system = system_drive();
        let mut out = Vec::new();
        for i in 0..26u8 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let letter = (b'A' + i) as char;
            if Some(letter) == system {
                continue;
            }
            let root = PathBuf::from(format!("{letter}:\\"));
            let root_w = wide(root.as_os_str());
            // SAFETY: NUL-terminated root path.
            let kind = unsafe { GetDriveTypeW(root_w.as_ptr()) };
            let removable = match kind {
                DRIVE_REMOVABLE => true,
                DRIVE_FIXED => on_external_bus(letter),
                _ => false,
            };
            if !removable {
                continue;
            }
            let mut label = [0u16; 261];
            let mut fs = [0u16; 261];
            // SAFETY: buffers and their lengths match; unused outputs are null.
            let ok = unsafe {
                GetVolumeInformationW(
                    root_w.as_ptr(),
                    label.as_mut_ptr(),
                    label.len() as u32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    fs.as_mut_ptr(),
                    fs.len() as u32,
                )
            };
            // An empty card reader slot has no medium.
            if ok == 0 {
                continue;
            }
            let label = from_wide(&label);
            let (total, free) = space(&root).unwrap_or((0, 0));
            out.push(Volume {
                name: if label.is_empty() {
                    format!("{letter}:")
                } else {
                    label
                },
                mount_point: root,
                fs_type: from_wide(&fs),
                total_bytes: total,
                free_bytes: free,
                device_node: Some(format!("{letter}:")),
            });
        }
        out
    }

    /// Flush, lock, dismount, allow removal, eject (the sequence Explorer's
    /// "Eject" uses for removable media).
    pub fn eject(volume: &Volume) -> Result<(), String> {
        let letter = volume
            .device_node
            .as_deref()
            .and_then(|d| d.chars().next())
            .or_else(|| volume.mount_point.to_string_lossy().chars().next())
            .filter(char::is_ascii_alphabetic)
            .ok_or_else(|| "no drive letter".to_string())?;
        let dev = Device::open(letter, GENERIC_READ | GENERIC_WRITE)
            .ok_or_else(|| std::io::Error::last_os_error().to_string())?;
        // SAFETY: valid handle.
        unsafe { FlushFileBuffers(dev.0) };
        let mut locked = false;
        for _ in 0..10 {
            if dev.ioctl(FSCTL_LOCK_VOLUME, &[], &mut []) {
                locked = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        if !locked {
            return Err("the drive is in use by another program".into());
        }
        if !dev.ioctl(FSCTL_DISMOUNT_VOLUME, &[], &mut []) {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let allow = PREVENT_MEDIA_REMOVAL {
            PreventMediaRemoval: false,
        };
        dev.ioctl(IOCTL_STORAGE_MEDIA_REMOVAL, as_bytes(&allow), &mut []);
        // Fixed USB disks refuse the eject request; dismounted is safe to
        // unplug, so that is not an error.
        dev.ioctl(IOCTL_STORAGE_EJECT_MEDIA, &[], &mut []);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescapes_mount_points() {
        assert_eq!(
            unescape_mount("/media/me/MY\\040STICK"),
            "/media/me/MY STICK"
        );
    }

    #[test]
    fn space_of_tempdir() {
        let d = tempfile::tempdir().unwrap();
        let (total, free) = space(d.path()).unwrap();
        assert!(total > 0 && free <= total);
    }

    #[test]
    fn test_hook_lists_directories() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("STICK")).unwrap();
        let v = list_dir_as_volumes(d.path());
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].name, "STICK");
    }
}
