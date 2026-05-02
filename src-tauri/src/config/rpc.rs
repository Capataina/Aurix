use std::env;
use std::sync::Once;

use thiserror::Error;

static ENVIRONMENT_BOOTSTRAP: Once = Once::new();

/// RPC + transport configuration loaded from process environment.
///
/// Inputs/Outputs/Errors/Side effects:
///   - Inputs: `MAINNET_RPC_URL` (preferred) or `ALCHEMY_API_KEY` from process
///     environment, optionally read from a `.env` file in the working directory
///     or the parent directory.
///   - Outputs: a validated configuration object exposing the resolved RPC URL.
///   - Errors: returned when neither variable is present.
///   - Side effects: lazily loads `.env` files exactly once per process.
#[derive(Debug, Clone)]
pub struct AppConfig {
    ethereum_mainnet_rpc_url: String,
}

impl AppConfig {
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

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingEnvironmentVariable(&'static str),
}
