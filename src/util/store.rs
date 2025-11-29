//! The object store; the central system for storing and reusing data.
//!
//! This currently includes a content-addressable storage system.
//!
//! TODO: See if it can be used as an HTTP cache.

use crate::cli::CliContext;
use crate::config::Config;
use crate::util::keyed::Keyed;
use anyhow::Context;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use dashmap::DashMap;
use futures_util::StreamExt;
use relative_path::RelativePath;
use serde::{de, ser};
use sha2::digest::FixedOutput;
use sha2::Digest;
use std::borrow::Borrow;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, marker};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

pub struct ObjectStore {
    pub base_path: Arc<Path>,
    pub index_file: Arc<Path>,
    pub tmp_dir: Arc<Path>,
    /// The current working index.
    index: ObjectStoreIndex,
}
impl ObjectStore {
    pub fn load(
        context: &CliContext,
        config: &Config,
    ) -> anyhow::Result<Arc<Self>> {
        let base_path = config.fontpm.store_dir.resolved.clone();
        let index_file: Arc<Path> = base_path.join("index").into();
        let tmp_dir = config.fontpm.store_tmp_dir.resolved.clone();
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
                        context.warn_v(),
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

    pub async fn save(&self, context: &CliContext) -> anyhow::Result<()> {
        if !context.modify_files {
            let _ = writeln!(
                context.warn_v(),
                "Object store index would be written but will not"
            );
            return Ok(());
        }
        let data = toml::to_string(&self.index).context("serialising")?;
        tokio::fs::write(&self.index_file, &data)
            .await
            .context("writing")?;
        Ok(())
    }

    pub async fn download_to_tmp(
        &self,
        response: reqwest::Response,
        file_name: &str,
        write_file: bool,
        in_memory: bool,
        mut update: impl FnMut(usize, Option<usize>),
    ) -> anyhow::Result<TmpDownload> {
        // TODO: Figure out the interaction between this and dry-run mode
        let mut digest = ObjectHashAlgorithm::new();

        let expected_length = response.content_length().map(|a| a as usize);

        let mut memory: Option<Vec<u8>> = in_memory.then(|| {
            expected_length
                .map(|a| Vec::with_capacity(a as _))
                .unwrap_or(vec![])
        });

        let mut file = if write_file {
            let path = self.tmp_dir.join(file_name);
            if !self.tmp_dir.exists() {
                tokio::fs::create_dir_all(&self.tmp_dir).await?;
            }
            let file = tokio::fs::File::create(&path)
                .await
                .context("creating file")?;
            if let Some(len) = expected_length {
                file.set_len(len as u64).await?;
            }

            let file = file.into_std().await;
            let file = tokio::task::spawn_blocking(|| {
                file.try_lock().map(|_| tokio::fs::File::from_std(file))
            })
            .await
            .context("spawning task")?
            .context("locking file")?;
            Some((path, file))
        } else {
            None
        };

        let mut stream = response.bytes_stream();
        let mut total = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            digest.update(&chunk);
            if let Some(memory) = &mut memory {
                memory.extend_from_slice(chunk.as_ref());
            }
            if let Some((_path, file)) = &mut file {
                file.write_all(chunk.as_ref()).await?;
            }
            total += chunk.len();
            update(total, expected_length)
        }

        if let Some((_path, file)) = &mut file {
            file.flush().await?;
        }

        Ok(TmpDownload {
            hash: ObjectId(digest.finalize_fixed()),
            content: memory,
            file,
        })
    }

    /// Get an already indexed object.
    ///
    /// Returns `Some(object)` if an object with the hash `hash` exists.
    /// Returns `None` otherwise.
    pub fn get_object(
        &self,
        hash: impl AsRef<ObjectId>,
    ) -> Option<Arc<Object>> {
        self.index
            .objects
            .get(hash.as_ref())
            .map(|a| a.value().clone())
    }
    pub fn object_exists(&self, hash: impl AsRef<ObjectId>) -> bool {
        self.index.objects.contains_key(hash.as_ref())
    }

    pub fn resolve_object_path(&self, relative: &RelativePath) -> PathBuf {
        relative.to_path(&self.base_path)
    }

    /// Adds an object to the index.
    ///
    /// This function does not create a file; it is the responsibility of the
    /// caller to create the file indicated by the returned [`Object`].
    ///
    /// Returns `None` if the object was already indexed.
    /// Returns `Some(object)` if the object was added to the index.
    pub fn index_object(
        &self,
        hash: impl Into<ObjectId>,
    ) -> Option<Arc<Object>> {
        let hash = hash.into();
        if self.index.objects.contains_key(&hash) {
            return None;
        }
        let hash = Arc::new(hash);

        let hash_encoded = URL_SAFE_NO_PAD.encode(hash.0);
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

#[must_use]
pub struct TmpDownload {
    pub hash: ObjectId,
    pub content: Option<Vec<u8>>,
    pub file: Option<(PathBuf, tokio::fs::File)>,
}
impl TmpDownload {
    pub async fn move_to(
        self,
        target_path: &Path,
        cli: &CliContext,
    ) -> anyhow::Result<()> {
        if let Some((source_path, mut source_file)) = self.file {
            if let Some(parent) = target_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            match tokio::fs::rename(&source_path, target_path).await {
                Ok(_) => {}
                Err(err) => match err.kind() {
                    io::ErrorKind::CrossesDevices => {
                        source_file.seek(io::SeekFrom::Start(0)).await?;
                        let mut target_file =
                            tokio::fs::File::create(target_path).await?;
                        tokio::io::copy(&mut source_file, &mut target_file)
                            .await?;
                        drop(source_file);
                        tokio::fs::remove_file(&source_path).await?;
                    }
                    io::ErrorKind::AlreadyExists => {
                        let _ = writeln!(cli.warn_v(), "Attempting to move file {} to {} but the file already exists; ignoring", source_path.display(), target_path.display());
                    }
                    _ => return Err(err).context("moving file failed"),
                },
            }
        }

        Ok(())
    }
    pub async fn discard(self, cli: &CliContext) {
        if let Some((path, file)) = self.file {
            drop(file);
            if let Err(e) = tokio::fs::remove_file(&path).await {
                let _ = writeln!(
                    cli.warn(),
                    "Could not delete temporary file {}: {}",
                    path.display(),
                    e
                );
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ObjectStoreIndex {
    #[serde(rename = "object", with = "crate::util::keyed::AsKeyedValues")]
    objects: DashMap<Arc<ObjectId>, Arc<Object>>,
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
    pub hash: Arc<ObjectId>,
    pub path: Arc<RelativePath>,
}

impl Keyed<Arc<ObjectId>> for Object {
    fn key(&self) -> Arc<ObjectId> {
        self.hash.clone()
    }
}

/// Algorithm used to calculate hashes for object's data.
///
/// TODO: Evaluate choices. Currently it is set as SHA-256 due to its speed.
///       The primary criteria are speed and size; the faster it is the better,
///       but it can't be too big either.
pub type ObjectHashAlgorithm = sha2::Sha256;

#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub struct ObjectId(ObjectHashOutput);
impl ObjectId {
    pub fn from_ref(output: &ObjectHashOutput) -> &ObjectId {
        unsafe { std::mem::transmute(output) }
    }
}
impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This is a needlessly large buffer but it'll work
        let mut buf = [0u8; 128];
        let size = URL_SAFE_NO_PAD
            .encode_slice(self.0.as_slice(), &mut buf)
            .map_err(|_| fmt::Error)?;
        let data = unsafe { str::from_utf8_unchecked(&buf[0..size]) };
        f.write_str(data)
    }
}
impl From<ObjectHashOutput> for ObjectId {
    fn from(value: ObjectHashOutput) -> Self {
        Self(value)
    }
}
impl<T> Borrow<T> for ObjectId
where
    ObjectHashOutput: Borrow<T>,
{
    fn borrow(&self) -> &T {
        self.0.borrow()
    }
}
impl AsRef<ObjectId> for ObjectId {
    fn as_ref(&self) -> &ObjectId {
        self
    }
}
impl AsRef<ObjectHashOutput> for ObjectId {
    fn as_ref(&self) -> &ObjectHashOutput {
        &self.0
    }
}
impl AsRef<[u8]> for ObjectId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl ser::Serialize for ObjectId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let value = URL_SAFE_NO_PAD.encode(self.0);
        serializer.serialize_str(&value)
    }
}
impl<'de> de::Deserialize<'de> for ObjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct VisitorImpl<T>(marker::PhantomData<T>);
        impl<'de, T> de::Visitor<'de> for VisitorImpl<T>
        where
            for<'a> T: TryFrom<&'a [u8]>,
            for<'a> <T as TryFrom<&'a [u8]>>::Error: fmt::Display,
        {
            type Value = T;

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
                T::try_from(value.as_slice()).map_err(<E as de::Error>::custom)
            }
        }
        deserializer
            .deserialize_str(VisitorImpl(marker::PhantomData))
            .map(Self)
    }
}

pub type ObjectHashOutput = sha2::digest::Output<ObjectHashAlgorithm>;
