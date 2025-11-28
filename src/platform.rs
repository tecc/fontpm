//! Platform-specific routines.
//!
//! # Local and global fonts
//!
//! *Local fonts* are fonts that are available only to the user who installed
//! them.
//!
//! *Global fonts* are fonts that are available system-wide.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

mod imp {
    #[cfg(target_os = "linux")]
    pub use super::linux::*;
    #[cfg(target_os = "macos")]
    pub use super::macos::*;
    #[cfg(target_os = "windows")]
    pub use super::windows::*;
}

// TODO: Add parameters to the functions to test them
trait PlatformImpl {
    /// Install a local font.
    fn install_font_local();
    /// Uninstall a local font.
    fn uninstall_font_local();
    /// Install a global font.
    fn install_font_global();
    /// Uninstall a font.
    fn uninstall_font_global();
}
