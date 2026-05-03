use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;

/// Lightweight JSON-RPC client for read-only Ethereum calls.
#[derive(Debug, Clone)]
pub struct EthereumRpcClient {
    http_client: Client,
    rpc_url: String,
}

impl EthereumRpcClient {
    /// Builds a reusable JSON-RPC client for the supplied endpoint.
    ///
    /// Inputs: an HTTPS JSON-RPC URL.
    /// Outputs: a client capable of issuing Ethereum RPC requests.
    /// Errors: none during construction.
    /// Side effects: allocates an internal HTTP client.
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            http_client: Client::new(),
            rpc_url: rpc_url.into(),
        }
    }

    /// Performs a read-only contract call using `eth_call`.
    ///
    /// Inputs: target contract address and ABI-encoded calldata.
    /// Outputs: the raw hex result returned by the EVM.
    /// Errors: returned when the transport fails or the RPC server reports an error.
    /// Side effects: performs a network request.
    pub async fn eth_call(
        &self,
        contract_address: &str,
        calldata: &str,
    ) -> Result<String, EthereumRpcError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_call",
            "params": [
                {
                    "to": contract_address,
                    "data": calldata
                },
                "latest"
            ]
        });

        let rpc_response = self
            .http_client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<RpcResponse>()
            .await?;

        if let Some(error) = rpc_response.error {
            return Err(EthereumRpcError::Rpc(error.message));
        }

        rpc_response
            .result
            .ok_or(EthereumRpcError::MissingResultField)
    }

    /// Reads the current network gas price in gwei.
    ///
    /// Inputs: none.
    /// Outputs: the latest gas price converted into gwei.
    /// Errors: returned when the transport fails or the RPC server reports an error.
    /// Side effects: performs a network request.
    pub async fn gas_price_gwei(&self) -> Result<f64, EthereumRpcError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_gasPrice",
            "params": []
        });

        let rpc_response = self
            .http_client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<RpcResponse>()
            .await?;

        if let Some(error) = rpc_response.error {
            return Err(EthereumRpcError::Rpc(error.message));
        }

        let gas_price_hex = rpc_response
            .result
            .ok_or(EthereumRpcError::MissingResultField)?;
        let gas_price_wei = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
            .map_err(|_| EthereumRpcError::MalformedHexValue)?;

        Ok(gas_price_wei as f64 / 1_000_000_000.0)
    }

    /// Generic JSON-RPC call returning a `serde_json::Value`. Used for
    /// methods whose result is not a plain hex string — `eth_getLogs`
    /// (array), `eth_getBlockByNumber` (object), etc.
    ///
    /// Inputs: the method name and the params value (already shaped as
    /// the RPC expects).
    /// Outputs: the parsed `result` field as a `Value`.
    /// Errors: returned when the transport fails or the RPC server reports an error.
    /// Side effects: performs a network request.
    pub async fn rpc_call(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, EthereumRpcError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let response: RpcResponseGeneric = self
            .http_client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(error) = response.error {
            return Err(EthereumRpcError::Rpc(error.message));
        }
        response
            .result
            .ok_or(EthereumRpcError::MissingResultField)
    }
}

#[derive(Debug, Deserialize)]
struct RpcResponseGeneric {
    result: Option<Value>,
    error: Option<RpcErrorPayload>,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    result: Option<String>,
    error: Option<RpcErrorPayload>,
}

#[derive(Debug, Deserialize)]
struct RpcErrorPayload {
    message: String,
}

/// Errors produced by the transport or RPC server while reading Ethereum state.
#[derive(Debug, Error)]
pub enum EthereumRpcError {
    #[error("ethereum rpc transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("ethereum rpc error: {0}")]
    Rpc(String),
    #[error("ethereum rpc response did not include a result field")]
    MissingResultField,
    #[error("ethereum rpc response contained a malformed hex value")]
    MalformedHexValue,
}
