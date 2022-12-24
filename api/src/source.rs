use crate::host::FpmHost;
use async_trait::async_trait;
pub use crate::error::Error;

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

    async fn refresh(&self) -> Result<RefreshOutput, Error>;
}