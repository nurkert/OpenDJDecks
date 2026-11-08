//! Environment variables of the app: `OPENDJDECKS_<NAME>`.

use std::ffi::OsString;

/// The prefix of every variable.
pub const PREFIX: &str = "OPENDJDECKS_";

/// The value of `OPENDJDECKS_<name>`.
pub fn var_os(name: &str) -> Option<OsString> {
    std::env::var_os(format!("{PREFIX}{name}"))
}

/// [`var_os`] as UTF-8 (a non-UTF-8 value counts as unset).
pub fn var(name: &str) -> Option<String> {
    var_os(name).and_then(|v| v.into_string().ok())
}
