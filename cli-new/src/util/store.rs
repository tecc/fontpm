//! The object store; the central system for storing and reusing data.
//!
//! This currently includes a content-addressable storage system.
//!
//! TODO: See if it can be used as an HTTP cache.

use crate::cli::CliContext;
use crate::util::keyed::Keyed;
use anyhow::Context;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use dashmap::DashMap;
use relative_path::RelativePath;
use serde::{de, ser};
use std::path::Path;
use std::sync::Arc;
use std::{fmt, marker};

pub struct ObjectStore {
    pub base_path: Arc<Path>,
    pub index_file: Arc<Path>,
    /// The current working index.
    index: ObjectStoreIndex,
}
impl ObjectStore {
    pub fn load(
        context: &CliContext,
        base_path: Arc<Path>,
    ) -> anyhow::Result<Arc<Self>> {
        let index_file: Arc<Path> = base_path.join("index").into();
        if !base_path.exists() && context.modify_files {
            let _ = std::fs::create_dir_all(&base_path)?;
        }
        let index = if !index_file.exists() {
            let index = ObjectStoreIndex::new();
            let create = || {
                if context.modify_files {
                    if let Err(e) = crate::util::create_parent_all(&index_file)
                    {
                        let _ = writeln!(context.error(), "Could not create parent file for object store index: {}", e);
                    } else {
                    }
                } else {
                    let _ = writeln!(
                        context.warn(),
                        "Object store index would be created but will not be"
                    );
                }
            };
            create();
            index
        } else {
            let index = std::fs::read(&index_file)
                .context("could not read index file")?;
            toml::de::from_slice(&index)
                .context("could not deserialise index file")?
        };

        Ok(Arc::new(Self {
            base_path,
            tmp_dir,
            index_file,
            index,
        }))
    }

    /// Get an already indexed object.
    ///
    /// Returns `Some(object)` if an object with the hash `hash` exists.
    /// Returns `None` otherwise.
    pub fn get_object(&self, hash: &ObjectHash) -> Option<Arc<Object>> {
        self.index.objects.get(hash).map(|a| a.value().clone())
    }
    /// Adds an object to the index.
    ///
    /// This function does not create a file; it is the responsibility of the
    /// caller to create the file indicated by the returned [`Object`].
    ///
    /// Returns `None` if the object was already indexed.
    /// Returns `Some(object)` if the object was added to the index.
    pub fn index_object(&self, hash: ObjectHash) -> Option<Arc<Object>> {
        if self.index.objects.contains_key(&hash) {
            return None;
        }
        let hash = Arc::new(hash);

        let hash_encoded = URL_SAFE_NO_PAD.encode(hash.as_ref());
        // The path to the object becomes the first two characters as a
        // parent to a file with the name of the hash as encoded.
        let path = RelativePath::new(&hash_encoded[0..2]).join(&hash_encoded);
        let path = Arc::from(path.into_boxed_relative_path());

        let object = Arc::new(Object {
            hash: hash.clone(),
            path,
        });
        self.index.objects.insert(hash, object.clone());
        Some(object)
    }
}

fn remove_newline(content: &'_ [u8]) -> Option<&'_ [u8]> {
    let mut newline = false;
    for (i, byte) in content.iter().copied().enumerate() {
        if !byte.is_ascii_whitespace() {
            return newline.then_some(&content[i..]);
        }
        if byte == b'\n' {
            newline = true;
        }
    }
    newline.then_some(&content[content.len()..])
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ObjectStoreIndex {
    #[serde(rename = "object", with = "crate::util::keyed::AsKeyedValues")]
    objects: DashMap<Arc<ObjectHash>, Arc<Object>>,
}

impl ObjectStoreIndex {
    fn new() -> Self {
        Self {
            objects: DashMap::new(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Object {
    /// A hash of the object's contents.
    #[serde(with = "AsBase64::<Arc<ObjectHash>>")]
    pub hash: Arc<ObjectHash>,
    pub path: Arc<RelativePath>,
}

impl Keyed<Arc<ObjectHash>> for Object {
    fn key(&self) -> Arc<ObjectHash> {
        self.hash.clone()
    }
}

/// Algorithm used to calculate hashes for object's data.
///
/// TODO: Evaluate choices. Currently it is set as SHA-256 due to its speed.
///       The primary criteria are speed and size; the faster it is the better,
///       but it can't be too big either.
pub type ObjectHashAlgorithm = sha2::Sha256;
pub type ObjectHash = sha2::digest::Output<ObjectHashAlgorithm>;

struct AsBase64<T>(marker::PhantomData<T>);
impl<T> AsBase64<Arc<T>> {
    pub fn serialize<S>(
        value: &Arc<T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
        T: AsRef<[u8]>,
    {
        let value = URL_SAFE_NO_PAD.encode(value.as_ref());
        serializer.serialize_str(&value)
    }
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Arc<ObjectHash>, D::Error>
    where
        D: de::Deserializer<'de>,
        for<'a> T: TryFrom<&'a [u8]>,
    {
        struct VisitorImpl;
        impl<'de> de::Visitor<'de> for VisitorImpl {
            type Value = ObjectHash;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a base64 string")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let value = URL_SAFE_NO_PAD
                    .decode(v)
                    .map_err(<E as de::Error>::custom)?;
                ObjectHash::try_from(value.as_slice())
                    .map_err(<E as de::Error>::custom)
            }
        }
        deserializer.deserialize_str(VisitorImpl).map(Arc::new)
    }
}
