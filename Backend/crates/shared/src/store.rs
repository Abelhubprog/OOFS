use anyhow::Result;
use std::{fs, path::PathBuf};
use url::Url;

#[async_trait::async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<String>;
}

pub struct FileStore {
    base: PathBuf,
}

impl FileStore {
    pub fn new(uri: &str) -> Result<Self> {
        let url = Url::parse(uri)?;
        if url.scheme() != "file" {
            anyhow::bail!("not a file:// uri")
        }
        let p = url
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("bad path"))?;
        fs::create_dir_all(&p).ok();
        Ok(Self { base: p })
    }
}

#[async_trait::async_trait]
impl ObjectStore for FileStore {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<String> {
        let mut path = self.base.clone();
        path.push(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        tokio::fs::write(&path, bytes).await?;
        Ok(format!("{}", key))
    }
}

#[cfg(feature = "with-r2")]
pub struct R2Store {
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix: String,
    public_url_base: String,
}

#[cfg(feature = "with-r2")]
impl R2Store {
    /// Create R2Store from configuration
    pub async fn from_config(
        config: &crate::config::AppConfig,
        prefix: Option<&str>,
    ) -> Result<Self> {
        use aws_credential_types::Credentials;
        use aws_types::region::Region;

        let prefix = prefix.unwrap_or("").to_string();

        let credentials =
            Credentials::from_keys(&config.r2_access_key_id, &config.r2_secret_access_key, None);

        let sdk_config = aws_config::SdkConfig::builder()
            .region(Region::new("auto"))
            .credentials_provider(credentials)
            .build();

        let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
            .endpoint_url(&config.r2_endpoint)
            .force_path_style(true)
            .build();

        let client = aws_sdk_s3::Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: config.r2_bucket_name.clone(),
            prefix,
            public_url_base: config.r2_public_url.clone(),
        })
    }

    /// Create R2Store from URI (legacy support)
    pub async fn new(uri: &str) -> Result<Self> {
        use aws_credential_types::Credentials;
        use aws_types::region::Region;

        let url = Url::parse(uri)?;
        if url.scheme() != "r2" {
            anyhow::bail!("not an r2:// uri")
        }

        let bucket = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("missing bucket (use r2://bucket)"))?
            .to_string();
        let prefix = url.path().trim_start_matches('/').to_string();

        // Use new environment variable names
        let access_key = std::env::var("R2_ACCESS_KEY_ID")
            .map_err(|_| anyhow::anyhow!("R2_ACCESS_KEY_ID required"))?;
        let secret_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .map_err(|_| anyhow::anyhow!("R2_SECRET_ACCESS_KEY required"))?;
        let endpoint =
            std::env::var("R2_ENDPOINT").map_err(|_| anyhow::anyhow!("R2_ENDPOINT required"))?;
        let public_url_base = std::env::var("R2_PUBLIC_URL")
            .map_err(|_| anyhow::anyhow!("R2_PUBLIC_URL required"))?;

        let credentials = Credentials::from_keys(access_key, secret_key, None);

        let sdk_config = aws_config::SdkConfig::builder()
            .region(Region::new("auto"))
            .credentials_provider(credentials)
            .build();

        let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();

        let client = aws_sdk_s3::Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket,
            prefix,
            public_url_base,
        })
    }

    /// Get public URL for an object
    pub fn get_public_url(&self, key: &str) -> String {
        let full_key = if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix, key)
        };

        format!(
            "{}/{}",
            self.public_url_base.trim_end_matches('/'),
            full_key
        )
    }
}

#[cfg(feature = "with-r2")]
#[async_trait::async_trait]
impl ObjectStore for R2Store {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<String> {
        let full_key = if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix, key)
        };

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(bytes.to_vec().into())
            .content_type("application/octet-stream") // Default content type
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upload to R2: {}", e))?;

        // Return the public URL instead of just the key
        Ok(self.get_public_url(key))
    }
}

/// Create object store from configuration
pub async fn make_store_from_config(
    config: &crate::config::AppConfig,
    prefix: Option<&str>,
) -> Result<std::sync::Arc<dyn ObjectStore>> {
    #[cfg(feature = "with-r2")]
    {
        Ok(std::sync::Arc::new(
            R2Store::from_config(config, prefix).await?,
        ))
    }
    #[cfg(not(feature = "with-r2"))]
    {
        // Fallback to file store for development
        let file_path = format!("file:///tmp/oof-assets/{}", prefix.unwrap_or(""));
        Ok(std::sync::Arc::new(FileStore::new(&file_path)?))
    }
}

/// Create object store from URI (legacy support)
pub async fn make_store(uri: &str) -> Result<std::sync::Arc<dyn ObjectStore>> {
    if uri.starts_with("file://") {
        Ok(std::sync::Arc::new(FileStore::new(uri)?))
    } else if uri.starts_with("r2://") {
        #[cfg(feature = "with-r2")]
        {
            Ok(std::sync::Arc::new(R2Store::new(uri).await?))
        }
        #[cfg(not(feature = "with-r2"))]
        {
            anyhow::bail!("R2 support not enabled; build with feature 'with-r2'")
        }
    } else {
        anyhow::bail!("unsupported store uri")
    }
}
