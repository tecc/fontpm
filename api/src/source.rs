use std::collections::HashMap;
use std::path::PathBuf;
use crate::host::FpmHost;
use async_trait::async_trait;
use crate::error::Error;
use crate::font::{DefinedFontInstallSpec, DefinedFontVariantSpec, FontDescription, FontInstallSpec};

#[derive(PartialEq, Eq)]
pub enum RefreshOutput {
    AlreadyUpToDate,
    Downloaded
}

#[async_trait]
pub trait Source<'host> {
    fn id(&self) -> &'host str;
    fn name(&self) -> &'host str;

    fn set_host(&mut self, host: &'host dyn FpmHost);

    async fn refresh(&self, force_refresh: bool) -> Result<RefreshOutput, Error>;
    async fn resolve_font(&self, spec: &FontInstallSpec) -> Result<(DefinedFontInstallSpec, FontDescription), Error>;
    async fn download_font(&self, spec: &DefinedFontInstallSpec, dir: &PathBuf) -> Result<HashMap<DefinedFontVariantSpec, PathBuf>, Error>;
    fn description(&self) -> SourceDescription {
        SourceDescription {
            id: self.id().to_string(),
            name: self.id().to_string()
        }
    }
}

#[derive(Clone, Debug)]
pub struct SourceDescription {
    pub id: String,
    pub name: String
}
pub trait SourceExt {
    fn description(&self) -> SourceDescription;
}