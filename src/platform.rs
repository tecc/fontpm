//! Platform-specific routines.
//!
//! # Global and local fonts
//!
//! *Global fonts* are fonts that are available system-wide.
//!
//! *Local fonts* are fonts that are available only to the user who installed
//! them.
//!
//! # Notes
//!
//! ## Linux
//!
//! Global fonts should be installed to `/usr/share/fonts`. For clarity FontPM
//! should use the `/usr/share/fonts/fontpm` subdirectory for installed fonts.
//!
//! Local fonts should be installed to `$XDG_DATA_HOME/fonts`
//! (e.g. `/home/alice/.local/share/fonts`). Similarly, prefer the `fontpm`
//! subdirectory.
//!
//! ## Windows
//!
//! Global fonts should be installed to `%windir%\Fonts` (typically
//! `C:\Windows\Fonts`). It is unclear whether Windows supports using
//! subdirectories there, so for that reason put the files directly there.
//!
//! Local fonts should be installed to `%LocalAppData%\Microsoft\Windows\Fonts`
//! (e.g. `C:\Users\alice\Microsoft\Windows\Fonts`). See above note on
//! subdirectories.
//!
//! After installing a font (either globally or locally),

use crate::cli::CliContext;
use crate::config::Config;
use crate::source::SourceContext;
use crate::util::font::{FontFileKind, FontReference};
use crate::util::store::{Object, ObjectId};
use crate::util::string_enum;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

pub fn default_global_font_dir() -> anyhow::Result<PathBuf> {
    <imp::Platform as PlatformImpl>::default_global_font_dir()
}
pub fn default_local_font_dir() -> anyhow::Result<PathBuf> {
    <imp::Platform as PlatformImpl>::default_local_font_dir()
}

pub struct Platform {
    inner: imp::Platform,
}
impl Platform {
    pub fn load(cli: &CliContext, config: &Config) -> anyhow::Result<Self> {
        Ok(Self {
            inner: imp::Platform::load(cli, config)?,
        })
    }

    pub async fn install_fonts_local(
        &mut self,
        context: &SourceContext,
        fonts_to_install: Vec<FontToInstall>,
    ) -> anyhow::Result<()> {
        self.inner
            .install_fonts_local(context, fonts_to_install)
            .await
    }
}

/// Platform-specific functionality required by FontPM.
///
/// This should be implemented by platform-specific code and wrapped by a
/// platform-agnostic layer.
trait PlatformImpl: Sized {
    fn load(cli: &CliContext, config: &Config) -> anyhow::Result<Self>;

    /// Default global (system-wide) font installation directory.
    fn default_global_font_dir() -> anyhow::Result<PathBuf>;
    /// Default local (user-specific) font installation directory.
    fn default_local_font_dir() -> anyhow::Result<PathBuf>;

    /// Install fonts locally.
    ///
    /// # Implementation notes
    ///
    /// This method MUST respect `modify_files`.
    async fn install_fonts_local(
        &mut self,
        cli: &SourceContext,
        fonts: Vec<FontToInstall>,
    ) -> anyhow::Result<()>;
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
pub struct FontToInstall {
    /// The font to install
    pub reference: FontReference,
    pub objects: Vec<FontObject>,
}
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
pub struct FontObject {
    pub name: RelativePathBuf,
    pub kind: FontFileKind,
    pub object: Arc<Object>,
}

string_enum!(
    /// Linking strategies.
    ///
    /// This is vaguely inspired by Bun (see
    /// https://bun.com/docs/pm/global-cache#installation-strategies).
    pub enum InstallStrategy {
        /// Creates a hard link from the target file to the source file.
        ///
        /// Will not work if the source and destination are on two different
        /// volumes.
        Hardlink = "hardlink",
        /// Copy the source file to the target file.
        Copy = "copy",
    }
);

pub const DEFAULT_INSTALL_STRATEGIES: &[InstallStrategy] =
    imp::DEFAULT_INSTALL_STRATEGIES;
