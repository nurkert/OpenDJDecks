//! The whole update path against the real GitHub releases: find the newest
//! release, download it, verify it and swap it in. Needs the network, so it
//! only runs on request: `cargo test -p od-update --test live -- --ignored`.

use od_update::{Arch, InstallKind, Os, Policy, Prepared, Probe, Target, Updater, Version};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

fn install(dir: &Path) -> (InstallKind, PathBuf) {
    if cfg!(target_os = "macos") {
        let bundle = dir.join("OpenDJDecks.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        std::fs::write(bundle.join("Contents/old"), b"old").unwrap();
        (InstallKind::MacBundle { bundle: bundle.clone() }, bundle)
    } else {
        let folder = dir.join("OpenDJDecks");
        std::fs::create_dir_all(folder.join("mappings")).unwrap();
        std::fs::write(folder.join("old"), b"old").unwrap();
        (InstallKind::Portable { dir: folder.clone() }, folder)
    }
}

/// A process id that is certainly gone, for the helper to wait on.
fn finished_pid() -> u32 {
    let mut child = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/c", "exit"]);
        c.spawn().unwrap()
    } else {
        std::process::Command::new("true").spawn().unwrap()
    };
    let pid = child.id();
    child.wait().unwrap();
    pid
}

#[test]
#[ignore = "downloads the newest release from GitHub"]
fn updates_from_the_real_release() {
    let d = tempfile::tempdir().unwrap();
    let (kind, installed) = install(d.path());
    let target = Target {
        os: Os::current(),
        arch: Arch::current(),
        kind,
        probe: Probe {
            dir_writable: true,
            ..Default::default()
        },
        args: Vec::new(),
    };
    let cache = d.path().join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let updater = Updater::with_target(cache.clone(), Version::new(0, 0, 1), target);
    let update = updater
        .check(&Policy {
            include_prereleases: false,
            skip: None,
        })
        .unwrap()
        .expect("a release newer than 0.0.1");
    println!("found {} with {:?}", update.version, update.strategy);
    let cancel = AtomicBool::new(false);
    let prepared = updater.prepare(&update, &mut |_, _| {}, &cancel).unwrap();
    let Prepared::Restart(finish) = prepared else {
        panic!("expected an update that swaps in place, got {prepared:?}");
    };
    assert!(finish.new.exists(), "staged next to the install");
    let status = run_helper(&finish, &cache);
    assert!(status, "the helper reports success");
    assert!(installed.exists());
    assert!(!installed.join("old").exists() && !installed.join("Contents/old").exists());
    assert!(!finish.new.exists());
}

#[cfg(not(windows))]
fn run_helper(finish: &od_update::Finish, cache: &Path) -> bool {
    let script = cache.join("update-helper.sh");
    std::fs::write(&script, od_update::install::HELPER_SCRIPT).unwrap();
    std::process::Command::new("/bin/sh")
        .arg(&script)
        .arg(finish.mode)
        .arg(finished_pid().to_string())
        .arg(&finish.current)
        .arg(&finish.new)
        .status()
        .unwrap()
        .success()
}

#[cfg(windows)]
fn run_helper(finish: &od_update::Finish, cache: &Path) -> bool {
    let finish = od_update::Finish {
        relaunch: Vec::new(),
        ..finish.clone()
    };
    std::fs::write(
        cache.join(od_update::install::WINDOWS_HELPER_NAME),
        od_update::install::windows_helper(&finish, finished_pid()),
    )
    .unwrap();
    std::process::Command::new("cmd")
        .args(["/d", "/c", od_update::install::WINDOWS_HELPER_NAME])
        .current_dir(cache)
        .status()
        .unwrap()
        .success()
}
