use std::env;
use std::sync::Once;

use thiserror::Error;

static ENVIRONMENT_BOOTSTRAP: Once = Once::new();

/// Application configuration required by the backend.
#[derive(Debug, Clone)]
pub struct AppConfig {
    ethereum_mainnet_rpc_url: String,
}

impl AppConfig {
    /// Loads backend configuration from process environment or local dotenv files.
    ///
    /// Inputs: none; reads environment variables from the current process and
    /// optional `.env` files.
    /// Outputs: a validated configuration object containing the resolved RPC URL.
    /// Errors: returned when neither `MAINNET_RPC_URL` nor `ALCHEMY_API_KEY` is
    /// available.
    /// Side effects: lazily loads dotenv files once per process.
    pub fn from_environment() -> Result<Self, ConfigError> {
        load_dotenv_files();

        if let Ok(rpc_url) = env::var("MAINNET_RPC_URL") {
            if !rpc_url.trim().is_empty() {
                return Ok(Self {
                    ethereum_mainnet_rpc_url: rpc_url,
                });
            }
        }

        let alchemy_api_key = env::var("ALCHEMY_API_KEY")
            .map_err(|_| ConfigError::MissingEnvironmentVariable("ALCHEMY_API_KEY"))?;

        if alchemy_api_key.trim().is_empty() {
            return Err(ConfigError::MissingEnvironmentVariable("ALCHEMY_API_KEY"));
        }

        Ok(Self {
            ethereum_mainnet_rpc_url: format!(
                "https://eth-mainnet.g.alchemy.com/v2/{}",
                alchemy_api_key.trim()
            ),
        })
    }

    /// Returns the Ethereum mainnet JSON-RPC HTTPS endpoint used for read-only calls.
    pub fn ethereum_mainnet_rpc_url(&self) -> &str {
        &self.ethereum_mainnet_rpc_url
    }
}

fn load_dotenv_files() {
    ENVIRONMENT_BOOTSTRAP.call_once(|| {
        let _ = dotenvy::dotenv();
        let _ = dotenvy::from_filename("../.env");
    });
}

/// Configuration failures that prevent the backend from initialising its network client.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingEnvironmentVariable(&'static str),
}
