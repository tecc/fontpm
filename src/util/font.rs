use crate::source::SourceId;
use crate::util::{impl_serde_as_string, string_enum};
use chrono::{DateTime, Utc};
use relative_path::RelativePathBuf;
use reqwest::Url;
use serde::{de, ser, Deserialize, Serialize};
use std::cmp::Ordering;
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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResolvedFont {
    pub reference: FontReference,
    pub files: Vec<FontFile>,
}

/// Generic resolved font struct.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
pub struct FontReference {
    pub identifier: FontIdentifier,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}
impl fmt::Display for FontReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}", self.identifier, self.version)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FontIdentifier {
    pub source: SourceId,
    pub id: String,
}
impl_serde_as_string!(impl for FontIdentifier);

impl fmt::Display for FontIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.id)
    }
}
impl FromStr for FontIdentifier {
    type Err = InvalidFontIdentifier;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((source, id)) = s.split_once(":") else {
            return Err(InvalidFontIdentifier);
        };
        Ok(Self {
            source: SourceId::from(source),
            id: id.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid font identifier")]
pub struct InvalidFontIdentifier;

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
pub struct FontFile {
    pub name: RelativePathBuf,
    pub kind: FontFileKind,
    pub download: Download,
}
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
pub enum Download {
    Url(Url),
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
pub enum FontFileKind {
    Family {
        axes: Vec<FontAxis>,
        weight: Option<u32>,
        italic: bool,
    },
}

string_enum!(
    #[derive(Eq, PartialEq)]
    pub enum FontAxis {
        Weight = "wght",
        // TODO: Research well-known axes
    }
);
impl PartialOrd for FontAxis {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FontAxis {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}
