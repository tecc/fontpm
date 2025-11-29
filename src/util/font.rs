use crate::source::SourceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Font specification.
///
/// TODO: Version specification.
#[derive(Clone, Debug)]
pub struct FontSpec {
    /// ID of the font.
    pub id: String,
    pub source: Option<SourceId>,
}
impl FromStr for FontSpec {
    type Err = FontSpecError;

    fn from_str(remaining: &str) -> Result<Self, Self::Err> {
        let (source, remaining) =
            if let Some((source, remaining)) = remaining.split_once(':') {
                let source = SourceId::from_str(source)
                    .map_err(FontSpecError::Source)?;
                (Some(source), remaining)
            } else {
                (None, remaining)
            };

        Ok(Self {
            source,
            id: remaining.to_string(),
        })
    }
}
impl fmt::Display for FontSpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(source) = &self.source {
            write!(f, "{}:", source)?;
        }
        write!(f, "{}", &self.id)
    }
}
#[derive(thiserror::Error, Debug)]
pub enum FontSpecError {
    #[error("invalid source: {0}")]
    Source(<SourceId as FromStr>::Err),
}

/// Generic resolved font struct.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
pub struct ResolvedFont {
    pub id: String,
    pub source: SourceId,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}
impl fmt::Display for ResolvedFont {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}@{}", self.source, self.id, self.version)
    }
}
