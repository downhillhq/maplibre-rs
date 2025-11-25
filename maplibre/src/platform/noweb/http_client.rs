use std::{fs, path::PathBuf};

use async_trait::async_trait;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::{Client, StatusCode};
use reqwest_middleware::ClientWithMiddleware;

use crate::io::source_client::{HttpClient, SourceFetchError};

#[derive(Clone)]
pub struct ReqwestHttpClient {
    client: ClientWithMiddleware,
}

impl From<reqwest::Error> for SourceFetchError {
    fn from(err: reqwest::Error) -> Self {
        SourceFetchError(Box::new(err))
    }
}

impl From<reqwest_middleware::Error> for SourceFetchError {
    fn from(err: reqwest_middleware::Error) -> Self {
        SourceFetchError(Box::new(err))
    }
}

impl ReqwestHttpClient {
    /// cache_path: Under which path should we cache requests.
    pub fn new<P>(cache_path: Option<P>) -> Self
    where
        P: Into<PathBuf>,
    {
        let mut builder = reqwest_middleware::ClientBuilder::new(Client::new());

        if let Some(cache_path) = cache_path {
            let cache_path = cache_path.into();
            // Ensure the cache directory exists to avoid "fopen failed for data file" errors
            if let Err(e) = fs::create_dir_all(&cache_path) {
                log::warn!(
                    "Failed to create cache directory at {:?}: {}. Cache will be disabled.",
                    cache_path,
                    e
                );
                // Continue without cache if directory creation fails
            } else {
                builder = builder.with(Cache(HttpCache {
                    mode: CacheMode::Default,
                    manager: CACacheManager {
                        path: cache_path,
                    },
                    options: HttpCacheOptions::default(),
                }))
            }
        }
        let client = builder.build();

        Self { client }
    }
}

#[cfg_attr(not(feature = "thread-safe-futures"), async_trait(?Send))]
#[cfg_attr(feature = "thread-safe-futures", async_trait)]
impl HttpClient for ReqwestHttpClient {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, SourceFetchError> {
        let response = self.client.get(url).send().await?;
        match response.error_for_status() {
            Ok(response) => {
                if response.status() == StatusCode::NOT_MODIFIED {
                    log::info!("Using data from cache");
                }

                let body = response.bytes().await?;

                Ok(Vec::from(body.as_ref()))
            }
            Err(e) => Err(SourceFetchError(Box::new(e))),
        }
    }
}
