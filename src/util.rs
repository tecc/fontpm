pub mod font;
pub mod keyed;
pub mod store;

use serde::de::Error;
use serde::{de, ser};
use std::fmt::Write;
use std::path::Path;
use std::str::FromStr;
use std::{fmt, io, marker};

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

        const _: () = {
            use std::fmt;

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

            impl clap::ValueEnum for $enum_name {
                fn value_variants<'a>() -> &'a [Self] {
                    &[$(Self::$variant_name),*]
                }
                fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
                    Some(clap::builder::PossibleValue::new(self.as_str()))
                }
                fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
                    if ignore_case {
                         match input {
                             $(
                             s
                             if (
                                 s.eq_ignore_ascii_case($variant_str)
                                 $(|| s.eq_ignore_ascii_case($variant_str_alias))*
                             )
                             => return Ok(Self::$variant_name)
                             ),*,
                             _ => {}
                         }
                    } else {
                         match input {
                             $(
                             $variant_str $(| $variant_str_alias)* => return Ok(Self::$variant_name)
                             ),*,
                             _ => {}
                         }
                    }
                    Err(format!("invalid variant: {}", input))
                }
            }

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

pub(crate) use string_enum;

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("no such variant")]
pub struct NoSuchVariant;

pub struct AsString;
impl AsString {
    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: fmt::Display,
    {
        let mut tmp = String::new();
        write!(tmp, "{}", value).map_err(<S::Error as ser::Error>::custom)?;
        serializer.serialize_str(&tmp)
    }
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: de::Deserializer<'de>,
        T: FromStr,
        T::Err: fmt::Display,
    {
        struct VisitorImpl<T>(marker::PhantomData<T>);
        impl<'de, T> de::Visitor<'de> for VisitorImpl<T>
        where
            T: FromStr,
            T::Err: fmt::Display,
        {
            type Value = T;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a string")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                v.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_str(VisitorImpl(marker::PhantomData))
    }
}

macro_rules! impl_serde_as_string {
    (
        impl $( < $($impl_generics:ident)* > )? for $target_ty:ty
    ) => {
        const _: () = {
            use serde::{de, ser};
            impl$(<$($impl_generics)*>)? ser::Serialize for $target_ty
            where
                Self: fmt::Display
            {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ser::Serializer
                {
                    $crate::util::AsString::serialize(&self, serializer)
                }
            }
            impl<'de, $($($impl_generics)*)?> de::Deserialize<'de> for $target_ty
            where
                Self: std::str::FromStr,
                <Self as std::str::FromStr>::Err: fmt::Display
            {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: de::Deserializer<'de>
                {
                    $crate::util::AsString::deserialize(deserializer)
                }
            }
        };
    };
}
use crate::config::Config;
pub(crate) use impl_serde_as_string;

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
