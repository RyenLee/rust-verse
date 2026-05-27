//! System-level utilities — environment variables, PATH, binary detection.

#[cfg(target_os = "windows")]
pub mod env;
#[cfg(not(target_os = "windows"))]
pub mod env;

pub use env::*;
