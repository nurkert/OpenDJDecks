//! Keeps the computer and its display awake while decks play (roadmap
//! [11]), with the operating system's own mechanism:
//!
//! - macOS: an IOKit power assertion ("PreventUserIdleDisplaySleep"),
//!   released when the decks stop.
//! - Windows: `SetThreadExecutionState` with `ES_SYSTEM_REQUIRED |
//!   ES_DISPLAY_REQUIRED`, called on the UI thread and cleared again.
//! - Linux: a `systemd-inhibit --what=idle:sleep` process holding a logind
//!   inhibitor lock. It waits on its standard input, so it also ends if the
//!   app is killed. Without systemd-inhibit (no systemd, a minimal Raspberry
//!   Pi image) this is best effort: nothing is held and [`KeepAwake::note`]
//!   says so.
//!
//! Dropping the guard always releases what it holds.

/// Why the assertion exists, as the operating system shows it.
pub(crate) const REASON: &str = "OpenDJDecks: decks are playing";

/// Guards alive in this process (the unit test tells its own assertion
/// from those of apps running beside it in the same test process).
static LIVE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A held "stay awake" request.
pub(crate) struct KeepAwake {
    inner: Option<imp::Held>,
    note: String,
}

impl KeepAwake {
    /// Asks the system to stay awake. Never fails: when the system offers
    /// no way, the guard holds nothing and [`KeepAwake::note`] says why.
    pub(crate) fn acquire() -> Self {
        match imp::acquire() {
            Ok(h) => {
                LIVE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Self {
                    inner: Some(h),
                    note: String::new(),
                }
            }
            Err(why) => {
                tracing::info!(%why, "cannot keep the computer awake");
                Self {
                    inner: None,
                    note: why,
                }
            }
        }
    }

    /// True when the system confirmed the request.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn held(&self) -> bool {
        self.inner.is_some()
    }

    /// Why nothing is held ("" when it is).
    pub(crate) fn note(&self) -> &str {
        &self.note
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        if let Some(h) = self.inner.take() {
            imp::release(h);
            LIVE.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::{CString, c_char, c_void};

    type CFStringRef = *const c_void;
    const UTF8: u32 = 0x0800_0100;
    const LEVEL_ON: u32 = 255;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(alloc: *const c_void, s: *const c_char, enc: u32) -> CFStringRef;
        fn CFRelease(cf: *const c_void);
    }

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOPMAssertionCreateWithName(
            kind: CFStringRef,
            level: u32,
            name: CFStringRef,
            id: *mut u32,
        ) -> i32;
        fn IOPMAssertionRelease(id: u32) -> i32;
    }

    pub(crate) struct Held(u32);

    fn cf(s: &str) -> CFStringRef {
        let c = CString::new(s).unwrap_or_default();
        // SAFETY: a valid NUL-terminated UTF-8 string; the result is
        // released by the caller.
        unsafe { CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), UTF8) }
    }

    pub(crate) fn acquire() -> Result<Held, String> {
        let kind = cf("PreventUserIdleDisplaySleep");
        let name = cf(super::REASON);
        if kind.is_null() || name.is_null() {
            return Err("CoreFoundation string".into());
        }
        let mut id = 0u32;
        // SAFETY: both strings are valid CFStrings, `id` is a valid out
        // pointer; the strings are released after the call.
        let r = unsafe {
            let r = IOPMAssertionCreateWithName(kind, LEVEL_ON, name, &mut id);
            CFRelease(kind);
            CFRelease(name);
            r
        };
        if r == 0 {
            Ok(Held(id))
        } else {
            Err(format!("IOPMAssertionCreateWithName returned {r}"))
        }
    }

    pub(crate) fn release(h: Held) {
        // SAFETY: `h.0` was returned by IOPMAssertionCreateWithName and is
        // released once.
        unsafe {
            IOPMAssertionRelease(h.0);
        }
    }
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::Power::{
        ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
    };

    pub(crate) struct Held;

    pub(crate) fn acquire() -> Result<Held, String> {
        // SAFETY: plain flags; the state belongs to the calling (UI) thread,
        // which also releases it.
        let prev = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED) };
        if prev == 0 {
            Err("SetThreadExecutionState failed".into())
        } else {
            Ok(Held)
        }
    }

    pub(crate) fn release(_h: Held) {
        // SAFETY: as above; ES_CONTINUOUS alone clears the request.
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use std::process::{Child, Command, Stdio};

    pub(crate) struct Held(Child);

    pub(crate) fn acquire() -> Result<Held, String> {
        // `cat` holds the lock until its input closes: when the guard is
        // dropped, or when the app dies and the pipe goes with it.
        let child = Command::new("systemd-inhibit")
            .args([
                "--what=idle:sleep",
                "--who=OpenDJDecks",
                &format!("--why={}", super::REASON),
                "--mode=block",
                "cat",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("systemd-inhibit is not available ({e})"))?;
        Ok(Held(child))
    }

    pub(crate) fn release(mut h: Held) {
        drop(h.0.stdin.take());
        let _ = h.0.kill();
        let _ = h.0.wait();
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    pub(crate) struct Held;

    pub(crate) fn acquire() -> Result<Held, String> {
        Err("not supported on this system".into())
    }

    pub(crate) fn release(_h: Held) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The assertion is visible to the system while held and gone after.
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_assertion_is_held_and_released() {
        let listed = || {
            std::process::Command::new("pmset")
                .args(["-g", "assertions"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains(REASON))
                .unwrap_or(false)
        };
        // Apps under test in this process (a playing deck) hold their own
        // assertion with the same name: wait until none does.
        let quiet = |secs: u64| {
            let end = std::time::Instant::now() + std::time::Duration::from_secs(secs);
            while LIVE.load(std::sync::atomic::Ordering::SeqCst) > 0 && std::time::Instant::now() < end {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        };
        quiet(60);
        let guard = KeepAwake::acquire();
        assert!(guard.held(), "{}", guard.note());
        assert!(listed(), "pmset lists the assertion");
        drop(guard);
        let others = LIVE.load(std::sync::atomic::Ordering::SeqCst) > 0;
        assert!(others || !listed(), "released again");
    }

    /// Wherever it runs, acquiring and dropping never panics, and a guard
    /// that holds nothing says why.
    #[test]
    fn a_guard_holds_or_says_why() {
        let g = KeepAwake::acquire();
        assert!(g.held() || !g.note().is_empty());
    }
}
