use crate::platform::InstallStrategy;
use crate::util::font::ResolvedFont;
use anyhow::Context;
use relative_path::RelativePathBuf;
use std::path::PathBuf;

pub struct Platform {}

impl super::PlatformImpl for Platform {
    fn default_global_font_dir() -> anyhow::Result<PathBuf> {
        Ok("/usr/share/fonts/fontpm".into())
    }
    fn default_local_font_dir() -> anyhow::Result<PathBuf> {
        dirs::font_dir()
            .map(|path| path.join("fontpm"))
            .context("no font directory could be gotten")
    }

    fn install_font_global(&mut self) {
        todo!()
    }

    fn uninstall_font_global(&mut self) {
        todo!()
    }

    fn install_font_local(&mut self) {
        todo!()
    }

    fn uninstall_font_local(&mut self) {
        todo!()
    }
}

pub struct GlobalFonts {
    lockfile: Lockfile,
}

struct Lockfile {
    fonts: Vec<LockedFont>,
}
impl Lockfile {
    pub fn ensure_consistency(&mut self) {
        self.fonts
            .iter_mut()
            .for_each(LockedFont::ensure_consistency);
        self.fonts.sort();
    }
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct LockedFont {
    pub resolved: ResolvedFont,
    pub base_path: PathBuf,
    pub files: Vec<RelativePathBuf>,
}
impl LockedFont {
    pub fn ensure_consistency(&mut self) {
        self.files.sort();
    }
}

pub const DEFAULT_INSTALL_STRATEGIES: &[InstallStrategy] =
    &[InstallStrategy::Hardlink, InstallStrategy::Copy];
