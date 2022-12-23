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
    const ID: &'host str;
    const NAME: &'host str;

    fn id(&self) -> &'host str {
        return Self::ID;
    }
    fn name(&self) -> &'host str {
        return Self::NAME;
    }

    fn new() -> Self;

    fn set_host(&mut self, host: &'host dyn FpmHost);

    async fn refresh(&self) -> Result<RefreshOutput, Error>;
}