use crate::cli::CliContext;
use crate::config::Config;
use crate::platform::{FontInstallState, FontToInstall, InstallStrategy};
use crate::source::SourceContext;
use crate::util::font::FontReference;
use anyhow::Context;
use relative_path::RelativePathBuf;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::OnceCell as AsyncOnceCell;

pub type Platform = Arc<PlatformInner>;

pub struct PlatformInner {
    global_dir: Arc<Path>,
    local_dir: Arc<Path>,
    local_strategies: Arc<[InstallStrategy]>,
    local_storage: AsyncOnceCell<StorageLocation>,
}

impl PlatformInner {
    /// Links an object according to a strategy.
    ///
    /// Does not respect `modify_files`.
    async fn link_object(
        &self,
        cli: &CliContext,
        strategies: &[InstallStrategy],
        object_path: &Path,
        target_path: &Path,
    ) -> Result<InstallStrategy, LinkFailed> {
        async fn hardlink(from: &Path, to: &Path) -> anyhow::Result<()> {
            if let Some(parent) = to.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::hard_link(from, to).await?;
            Ok(())
        }
        async fn copy(from: &Path, to: &Path) -> anyhow::Result<()> {
            if let Some(parent) = to.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(from, to).await?;
            Ok(())
        }

        for strategy in strategies.iter().copied() {
            let result = match strategy {
                InstallStrategy::Hardlink => {
                    hardlink(object_path, target_path).await
                }
                InstallStrategy::Copy => copy(object_path, target_path).await,
            };
            match result {
                Ok(_) => return Ok(strategy),
                Err(e) => {
                    let _ = writeln!(
                        cli.debug(),
                        "Strategy {:?} failed from {} to {}: {}",
                        strategy,
                        object_path.display(),
                        target_path.display(),
                        e
                    );
                }
            }
        }

        Err(LinkFailed)
    }

    async fn get_local_storage(
        &self,
        cli: &CliContext,
    ) -> anyhow::Result<&StorageLocation> {
        self.local_storage
            .get_or_try_init(|| async {
                StorageLocation::load(
                    cli,
                    self.local_dir.clone(),
                    self.local_dir.join("local-index.json"),
                )
                .await
            })
            .await
    }
}

#[derive(Debug, thiserror::Error)]
#[error("could not link object because all strategies failed")]
struct LinkFailed;

impl super::PlatformImpl for Platform {
    fn load(_cli: &CliContext, config: &Config) -> anyhow::Result<Self> {
        Ok(Arc::new(PlatformInner {
            global_dir: config.fontpm.platform.global_font_dir.resolved.clone(),
            local_dir: config.fontpm.platform.local_font_dir.resolved.clone(),
            local_strategies: config.fontpm.install_strategy.resolved.clone(),
            local_storage: AsyncOnceCell::new(),
        }))
    }

    fn default_global_font_dir() -> anyhow::Result<PathBuf> {
        Ok("/usr/share/fonts/fontpm".into())
    }
    fn default_local_font_dir() -> anyhow::Result<PathBuf> {
        dirs::font_dir()
            .map(|path| path.join("fontpm"))
            .context("no font directory available")
    }

    async fn is_font_installed_local(
        &self,
        ctx: &SourceContext,
        font: &FontReference,
    ) -> anyhow::Result<FontInstallState> {
        let storage = self.get_local_storage(&ctx.cli).await?;
        let Ok(guard) = storage.lockfile.read() else {
            anyhow::bail!("lockfile data is poisoned")
        };

        Ok(FontInstallState {
            fontpm: guard.fonts.contains_key(font),
            // TODO: Use fontconfig to check if it is installed externally
            external: false,
        })
    }

    async fn install_fonts_local(
        &self,
        ctx: &SourceContext,
        fonts: Vec<FontToInstall>,
    ) -> anyhow::Result<()> {
        let local_storage = self.get_local_storage(&ctx.cli).await?;
        for font in fonts {
            // NOTE: These paths should generally be safe on Linux
            //       It might break if you're trying to write things on FAT
            //       filesystems (or NTFS, for that matter, but surely not
            //       right?).
            let font_base_path = RelativePathBuf::from(format!(
                "{}:{}",
                font.reference.source, font.reference.id
            ));
            let target_path = font_base_path.to_path(&self.local_dir);
            let _ = writeln!(
                ctx.cli.debug(),
                "Installing font {} to {}",
                font.reference,
                target_path.display()
            );

            let mut files = vec![];
            for object in font.objects {
                let target_path = object.name.to_path(&target_path);
                let object_path =
                    ctx.store.resolve_object_path(&object.object.path);

                if !ctx.cli.modify_files {
                    let _ = writeln!(
                        ctx.cli.debug(),
                        "Would install object {} to {} but will not",
                        object.object.hash,
                        target_path.display()
                    );
                    continue;
                }

                if target_path.exists() {
                    let _ = writeln!(
                        ctx.cli.debug(),
                        "Target path {} already exists, skipping",
                        target_path.display(),
                    );
                    continue;
                }

                let strategy = self
                    .link_object(
                        &ctx.cli,
                        &self.local_strategies,
                        &object_path,
                        &target_path,
                    )
                    .await?;
                let _ = writeln!(
                    ctx.cli.debug(),
                    "Strategy {} worked from {} to {}",
                    strategy,
                    object_path.display(),
                    target_path.display()
                );
                files.push(object.name)
            }

            let Ok(mut guard) = local_storage.lockfile.write() else {
                anyhow::bail!("could not acquire lock to lockfile data because it was poisoned")
            };

            guard.fonts.insert(
                font.reference.clone(),
                LockedFont {
                    reference: font.reference,
                    base_path: font_base_path,
                    files,
                },
            );
        }

        local_storage.save(&ctx.cli).await?;

        Ok(())
    }
}

struct StorageLocation {
    data_path: Arc<Path>,
    index_path: Arc<Path>,
    lockfile: RwLock<LockfileData>,
}
impl StorageLocation {
    async fn load(
        cli: &CliContext,
        base_path: impl Into<Arc<Path>> + AsRef<Path>,
        index_path: impl Into<Arc<Path>> + AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let base_path_r = base_path.as_ref();
        if !base_path_r.exists() && cli.modify_files {
            let _ = tokio::fs::create_dir_all(&base_path)
                .await
                .context("creating lockfile")?;
        }
        let index_path_r = index_path.as_ref();

        let lockfile = if !index_path_r.exists() {
            let lockfile = LockfileData::new();
            if let Err(e) = Self::save_data(cli, index_path_r, &lockfile).await
            {
                let _ = writeln!(
                    cli.error(),
                    "Could not create lockfile for storage location: {}",
                    e
                );
            }
            lockfile
        } else {
            let data = std::fs::read(&index_path_r)
                .context("could not read lockfile")?;
            serde_json::from_slice(&data)
                .context("could not deserialise lockfile")?
        };

        Ok(Self {
            data_path: base_path.into(),
            index_path: index_path.into(),
            lockfile: RwLock::new(lockfile),
        })
    }
    async fn save_data<T>(
        cli: &CliContext,
        out: &Path,
        value: &T,
    ) -> anyhow::Result<()>
    where
        T: serde::Serialize,
    {
        if !cli.modify_files {
            let _ = writeln!(
                cli.debug(),
                "{} would be saved but will not be",
                out.display()
            );
            return Ok(());
        }
        if let Some(parent) = out.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("creating parent directory")?;
        }
        let serialised =
            serde_json::to_vec_pretty(value).context("serialising data")?;
        tokio::fs::write(out, &serialised)
            .await
            .context("writing lockfile")?;
        Ok(())
    }

    async fn save(&self, cli: &CliContext) -> anyhow::Result<()> {
        let Ok(mut data) = self.lockfile.write() else {
            anyhow::bail!("lockfile data is poisoned")
        };
        data.ensure_consistency();
        Self::save_data(cli, &self.index_path, &*data)
            .await
            .context("saving lockfile")
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LockfileData {
    fonts: BTreeMap<FontReference, LockedFont>,
}
impl LockfileData {
    pub fn new() -> Self {
        Self {
            fonts: BTreeMap::default(),
        }
    }
    pub fn ensure_consistency(&mut self) {
        self.fonts
            .values_mut()
            .for_each(LockedFont::ensure_consistency);
    }
}

#[derive(
    Clone,
    Debug,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
)]
struct LockedFont {
    pub reference: FontReference,
    pub base_path: RelativePathBuf,
    pub files: Vec<RelativePathBuf>,
}
impl LockedFont {
    pub fn ensure_consistency(&mut self) {
        self.files.sort();
    }
}

pub const DEFAULT_INSTALL_STRATEGIES: &[InstallStrategy] =
    &[InstallStrategy::Hardlink, InstallStrategy::Copy];
