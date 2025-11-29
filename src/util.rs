pub mod font;
pub mod keyed;
pub mod store;

use std::io;
use std::path::Path;

pub fn create_parent_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
    } else {
        Ok(())
    }
}

/// Returns true if and only if exactly one of the booleans are true.
pub const fn exactly_one_is_true(conditions: &[bool]) -> bool {
    let mut any_is_true = false;
    let mut outer_idx = 0;
    let mut inner_idx;
    while outer_idx < conditions.len() {
        let first_condition = conditions[outer_idx];
        any_is_true = any_is_true || first_condition;
        inner_idx = outer_idx + 1;
        while inner_idx < conditions.len() {
            let second_condition = conditions[inner_idx];
            if first_condition && second_condition {
                return false;
            }
            inner_idx += 1;
        }
        outer_idx += 1;
    }
    any_is_true
}

pub fn create_runtime(_config: &Config) -> io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

/// Define an enum that is usually represented as a string.
macro_rules! string_enum {
    (
        $(#[ $enum_meta:meta ])*
        $enum_vis:vis enum $enum_name:ident {
            $(
            $(#[ $variant_meta:meta ])*
            $variant_name:ident = $variant_str:literal $(| $variant_str_alias:literal)*
            ),*
            $(,)?
        }
    ) => {
        $(#[ $enum_meta ])*
        #[derive(Debug, Copy, Clone, serde::Deserialize, serde::Serialize)]
        $enum_vis enum $enum_name {
            $(
            $(#[ $variant_meta ])*
            #[serde(rename = $variant_str)]
            $variant_name
            ),*
        }
        impl $enum_name {
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($variant_str $(| $variant_str_alias)* => Some(Self::$variant_name)),*,
                    _ => None
                }
            }
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant_name => $variant_str),*
                }
            }

        }
        const _: () = {
            use std::fmt;
            impl std::str::FromStr for $enum_name {
                type Err = $crate::util::NoSuchVariant;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Self::from_str(s).ok_or($crate::util::NoSuchVariant)
                }
            }
            impl fmt::Display for $enum_name {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str(self.as_str())
                }
            }
        };
    };
}

use crate::config::Config;
pub(crate) use string_enum;

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("no such variant")]
pub struct NoSuchVariant;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_one_is_true() {
        assert!(exactly_one_is_true(&[false, false, true]));
        assert!(exactly_one_is_true(&[false, true, false]));
        assert!(exactly_one_is_true(&[true, false, false]));

        assert!(!exactly_one_is_true(&[false, false, false]));
        assert!(!exactly_one_is_true(&[false, true, true]));
        assert!(!exactly_one_is_true(&[true, false, true]));
        assert!(!exactly_one_is_true(&[true, true, false]));
        assert!(!exactly_one_is_true(&[true, true, true]));
    }
}
