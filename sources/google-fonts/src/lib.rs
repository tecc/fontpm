mod github;
mod data;

use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};
use default_env::default_env;
use fontpm_api::{FpmHost, Source, trace};
use fontpm_api::async_trait::async_trait;
use fontpm_api::host::EmptyFpmHost;
use fontpm_api::source::{RefreshOutput};
use fontpm_api::Error;
use std::io::{Error as IOError, ErrorKind as IOErrorKind, Read, Write};
use reqwest::{Client, ClientBuilder};
use serde::{Serialize};
use serde::de::DeserializeOwned;
use sha2::{Sha256, Digest};
use fontpm_api::font::{DefinedFontInstallSpec, DefinedFontVariantSpec, FontInstallSpec};
use fontpm_api::util::create_parent;
use crate::data::Data;
use crate::data::description::variant_to_string;
use crate::github::GithubBranchData;

pub struct GoogleFontsSource<'host> {
    host: &'host dyn FpmHost,
    client: Option<Client>
}

// GitHub API
const COMMIT_DATA_URL: &str = default_env!("COMMIT", "https://api.github.com/repos/fontpm/data/branches/data");
// Raw content™
const FONT_INDEX_URL: &str = default_env!("FONT_INDEX_URL", "https://raw.githubusercontent.com/fontpm/data/data/google-fonts.json");

const COMMIT_FILE: &str = "commit.sha";
const DATA_FILE: &str = "data.json";

impl<'host> GoogleFontsSource<'host> {
    pub const ID: &'host str = "google-fonts";
    pub const NAME: &'host str = "Google Fonts";

    pub fn new() -> Self {
        return GoogleFontsSource {
            host: &EmptyFpmHost::EMPTY_HOST,
            client: None
        };
    }

    fn client(&self) -> &Client {
        return self.client.as_ref().unwrap();
    }

    fn cache_dir(&self) -> PathBuf {
        return self.host.cache_dir_for(Self::ID.into());
    }
    fn cache_file<P>(&self, s: P) -> PathBuf where P: AsRef<Path> {
        let mut path = self.cache_dir();
        path.push(s);
        path
    }

    fn last_downloaded_commit(&self) -> Option<String> {
        log::debug!("Reading last downloaded commit");
        let mut path = self.cache_dir();
        path.push(COMMIT_FILE);
        let path_str = path.clone().into_os_string();
        let path_str = path_str.to_string_lossy();

        let path = path;

        let mut file = match File::open(path) {
            Ok(v) => v,
            Err(e) => {
                match e.kind() {
                    IOErrorKind::NotFound => return None,
                    // there should probably be better error handling than this, but it's fine for now
                    _ => log::error!("Error whilst opening {}: {}", path_str, e)
                }
                return None
            }
        };

        let mut data = String::new();
        match file.read_to_string(&mut data) {
            Ok(v) => log::trace!("Read {} bytes from {}", v, path_str),
            Err(e) => {
                log::error!("Error whilst reading data: {}", e);
                return None
            }
        }

        Some(data)
    }

    async fn latest_commit(&self) -> Result<String, Error> {
        let response = self.client().get(COMMIT_DATA_URL).send().await?;
        #[cfg(debug_assertions)]
        let data = {
            let text = response.text().await?;
            serde_json::from_str::<GithubBranchData>(text.as_str())
                .map_err(|v| Error::Deserialisation(v.to_string()))?
        };
        #[cfg(not(debug_assertions))]
        let data: GithubBranchData = response.json().await?;

        Ok(data.commit.sha)
    }

    async fn get_data(&self) -> Result<Data, reqwest::Error> {
        let response = self.client().get(FONT_INDEX_URL).send().await?;
        let data = response.json::<Data>().await?;
        Ok(data)
    }

    fn cache_write_str<S, V>(&self, file: S, value: V) -> Result<(), Error> where S: AsRef<Path>, V: Into<String> {
        let path = self.cache_file(file);
        create_parent(&path)?;

        let str: String = value.into();
        let mut file = File::create(path)?;
        file.write_all(str.as_bytes())?;

        Ok(())
    }
    fn cache_write_serialise<P, T>(&self, file: P, value: &T) -> Result<(), Error> where P: AsRef<Path>, T: Serialize {
        let path = self.cache_file(file);
        create_parent(&path)?;

        let file = File::create(path)?;
        serde_json::ser::to_writer(file, value)
            .map_err(|v| Error::Generic(format!("Error whilst serialising: {}", v)))?;

        Ok(())
    }
    fn cache_read_deserialise<'de, T, P>(&self, file: P) -> Result<T, Error> where P: AsRef<Path>, T: DeserializeOwned {
        let path = self.cache_file(file);
        if !path.exists() {
            return Err(Error::IO(IOErrorKind::NotFound.into()))
        }

        let file = File::open(path)?;
        let result = serde_json::de::from_reader::<_, T>(file)
            .map_err(|v| Error::Deserialisation(format!("{}", v)));

        result
    }

    fn read_data(&self) -> Result<Data, Error> {
        let data_file = self.cache_file(DATA_FILE);
        if !data_file.exists() {
            return Err(Error::IO(IOError::new(IOErrorKind::NotFound, "Data file does not exist")))
        }

        self.cache_read_deserialise(data_file)
    }
}

#[async_trait]
impl<'host> Source<'host> for GoogleFontsSource<'host> {
    fn id(&self) -> &'host str {
        return Self::ID;
    }
    fn name(&self) -> &'host str {
        return Self::NAME;
    }

    fn set_host(&mut self, host: &'host dyn FpmHost) {
        self.host = host;
        self.client = Some(
            ClientBuilder::new()
                .user_agent(host.user_agent())
                .build()
                .expect("HTTP client required")
        )
    }

    async fn refresh(&self, force: bool) -> Result<RefreshOutput, Error> {
        let cache_file = self.cache_file(DATA_FILE);
        let latest = self.latest_commit().await?;
        trace!("[{}] Latest commit: {}", Self::ID, latest);

        if !force {
            let current = self.last_downloaded_commit();
            trace!("[{}] Last downloaded commit: {}", Self::ID, current.clone().unwrap_or("<none>".into()));
            if current != None && cache_file.exists() && current.unwrap() == latest {
                return Ok(RefreshOutput::AlreadyUpToDate)
            }
        }

        let index = self.get_data().await?;
        self.cache_write_serialise(DATA_FILE, &index)?;
        self.cache_write_str(COMMIT_FILE, latest)?;

        Ok(RefreshOutput::Downloaded)
    }

    async fn resolve_font(&self, spec: &FontInstallSpec) -> Result<DefinedFontInstallSpec, Error> {
        let data = self.read_data()?;

        let family = data.get_family(&spec.id).ok_or(Error::NoSuchFamily(spec.id.clone()))?;

        family.try_into()
    }

    async fn download_font(&self, font_id: &DefinedFontInstallSpec, dir: &PathBuf) -> Result<HashMap<DefinedFontVariantSpec, PathBuf>, Error> {
        let data = self.read_data()?;

        let font = if let Some(desc) = data.get_family(&font_id.id) {
            desc
        } else {
            return Err(Error::Generic(format!("Font {} does not exist", font_id.id)))
        };

        let dir = dir.join(&font_id.id);
        let mut paths = HashMap::new();

        for variant in &font_id.styles {
            let variant_name = variant_to_string(variant);
            let remote_file = match font.files.get(variant_name.as_str()) {
                Some(file) => file.clone(),
                None => return Err(Error::Generic(format!("Could not get file for font variant {variant_name}")))
            };

            let extension = PathBuf::from(&remote_file).extension().map_or(String::new(), |v| ".".to_string() + v.to_str().unwrap());
            let url_hash = Sha256::new()
                .chain_update(&remote_file)
                .finalize();
            let url_hash = format!("{:x}", url_hash);
            let path = dir.join(variant_name).join(format!("{}{}", url_hash, extension));

            paths.insert(variant.clone(), path.clone());
            if path.exists() {
                continue;
            }
            path.parent().map(create_dir_all);

            let remote_data = reqwest::get("https://".to_string() + remote_file.as_str()).await?.error_for_status()?;
            let mut file = File::create(&path)?;
            let remote_data = remote_data.bytes().await?;
            file.write_all(remote_data.as_ref())?;
        }

        Ok(paths)
    }
}