mod github;
mod data;

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use default_env::default_env;
use fontpm_api::{debug, FpmHost, Source, trace};
use fontpm_api::async_trait::async_trait;
use fontpm_api::host::EmptyFpmHost;
use fontpm_api::source::{Error, RefreshOutput};
use std::io::{ErrorKind as IOErrorKind, Read, Write};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use fontpm_api::util::create_parent;
use crate::data::Data;
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
        let data = response.json().await?;

        Ok(data.commit.sha)
    }

    async fn get_data(&self) -> Result<Data, reqwest::Error> {
        let response = self.client().get(FONT_INDEX_URL).send().await?;
        let data = response.json::<Data>().await?;
        Ok(data)
    }

    fn cache_write_str<S, V>(&self, file: S, value: V) -> Result<(), Error> where S: Into<String>, V: Into<String> {
        let mut path = self.cache_dir();
        path.push(file.into());
        let path = path;

        create_parent(&path)?;

        let str: String = value.into();
        let mut file = File::create(path)?;
        file.write_all(str.as_bytes())?;

        Ok(())
    }
    fn cache_write_serialise<S, T>(&self, file: S, value: &T) -> Result<(), Error> where S: Into<String>, T: Serialize {
        let mut path = self.cache_dir();
        path.push(file.into());
        let path = path;

        create_parent(&path)?;

        let file = File::create(path)?;
        serde_json::ser::to_writer(file, value)
            .map_err(|v| Error::Generic(format!("Error whilst serialising: {}", v)))?;

        Ok(())
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

    async fn refresh(&self) -> Result<RefreshOutput, Error> {
        let cache_file = self.cache_file(DATA_FILE);
        let current = self.last_downloaded_commit();
        trace!("[google-fonts] Last downloaded commit: {}", current.clone().unwrap_or("<none>".into()));
        let latest = self.latest_commit().await?;
        trace!("Latest commit: {}", latest);

        if current != None && cache_file.exists() && current.unwrap() == latest {
            return Ok(RefreshOutput::AlreadyUpToDate)
        }

        let index = self.get_data().await?;
        self.cache_write_serialise(DATA_FILE, &index)?;
        self.cache_write_str(COMMIT_FILE, latest)?;

        Ok(RefreshOutput::Downloaded)
    }
}