#[cfg(target_os = "windows")]
#[path = "sys/windows.rs"]
mod imp;
#[cfg(not(target_os = "windows"))]
#[path = "sys/noop.rs"]
mod imp;

pub use self::imp::{setup, System};
