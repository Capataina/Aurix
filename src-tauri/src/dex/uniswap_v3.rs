use std::time::{SystemTime, UNIX_EPOCH};

use num_bigint::BigUint;
use num_traits::ToPrimitive;
use thiserror::Error;

use crate::ethereum::client::{EthereumRpcClient, EthereumRpcError};
use crate::market::types::PriceSnapshot;

const UNISWAP_V3_WETH_USDC_005_POOL: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640";
const UNISWAP_V3_WETH_USDC_030_POOL: &str = "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8";
const SLOT0_CALLDATA: &str = "0x3850c7bd";
const TOKEN0_DECIMALS: u32 = 6;
const TOKEN1_DECIMALS: u32 = 18;

/// Fetches the current WETH/USDC market state from the canonical Uniswap V3 0.05% pool.
///
/// Inputs: an Ethereum JSON-RPC client configured for Ethereum mainnet.
/// Outputs: a normalised price snapshot containing the latest decoded spot price.
/// Errors: returned when the RPC call fails or the pool response is malformed.
/// Side effects: performs a single read-only `eth_call`.
pub async fn fetch_weth_usdc_snapshot(
    rpc_client: &EthereumRpcClient,
) -> Result<PriceSnapshot, UniswapV3Error> {
    fetch_pool_snapshot(
        rpc_client,
        "Uniswap V3 5bps",
        UNISWAP_V3_WETH_USDC_005_POOL,
        5,
    )
    .await
}

/// Fetches the current WETH/USDC market state from the major Uniswap V3 0.30% pool.
pub async fn fetch_weth_usdc_30bps_snapshot(
    rpc_client: &EthereumRpcClient,
) -> Result<PriceSnapshot, UniswapV3Error> {
    fetch_pool_snapshot(
        rpc_client,
        "Uniswap V3 30bps",
        UNISWAP_V3_WETH_USDC_030_POOL,
        30,
    )
    .await
}

async fn fetch_pool_snapshot(
    rpc_client: &EthereumRpcClient,
    dex_name: &str,
    pool_address: &str,
    fee_tier_bps: u16,
) -> Result<PriceSnapshot, UniswapV3Error> {
    let slot0_hex = rpc_client.eth_call(pool_address, SLOT0_CALLDATA).await?;
    let sqrt_price_x96 = decode_sqrt_price_x96(&slot0_hex)?;
    let price_usd = derive_weth_price_usd(&sqrt_price_x96)?;
    let fetched_at_unix_ms = current_unix_timestamp_ms()?;

    Ok(PriceSnapshot {
        chain: "Ethereum Mainnet".to_string(),
        dex_name: dex_name.to_string(),
        pair_label: "WETH / USDC".to_string(),
        price_usd,
        pool_address: pool_address.to_string(),
        fee_tier_bps,
        price_source_label: "slot0() spot price".to_string(),
        fetched_at_unix_ms,
    })
}

fn decode_sqrt_price_x96(slot0_hex: &str) -> Result<BigUint, UniswapV3Error> {
    let normalised_hex = slot0_hex.trim_start_matches("0x");

    if normalised_hex.len() < 64 {
        return Err(UniswapV3Error::MalformedSlot0);
    }

    let sqrt_price_word = &normalised_hex[..64];
    let bytes = hex::decode(sqrt_price_word).map_err(UniswapV3Error::InvalidHex)?;

    Ok(BigUint::from_bytes_be(&bytes))
}

fn derive_weth_price_usd(sqrt_price_x96: &BigUint) -> Result<f64, UniswapV3Error> {
    let numerator: BigUint =
        (BigUint::from(1u8) << 192) * BigUint::from(10u64).pow(TOKEN1_DECIMALS - TOKEN0_DECIMALS);
    let denominator: BigUint = sqrt_price_x96.pow(2u32);

    let numerator_f64 = numerator
        .to_f64()
        .ok_or(UniswapV3Error::PrecisionOverflow)?;
    let denominator_f64 = denominator
        .to_f64()
        .ok_or(UniswapV3Error::PrecisionOverflow)?;

    if denominator_f64 == 0.0 {
        return Err(UniswapV3Error::MalformedSlot0);
    }

    Ok(numerator_f64 / denominator_f64)
}

fn current_unix_timestamp_ms() -> Result<u64, UniswapV3Error> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| UniswapV3Error::ClockUnavailable)?;

    let milliseconds = duration.as_millis();
    milliseconds
        .try_into()
        .map_err(|_| UniswapV3Error::ClockUnavailable)
}

/// Errors encountered while reading or decoding the first Uniswap V3 market feed.
#[derive(Debug, Error)]
pub enum UniswapV3Error {
    #[error(transparent)]
    Rpc(#[from] EthereumRpcError),
    #[error("received malformed slot0 payload from Uniswap V3")]
    MalformedSlot0,
    #[error("received invalid hex data from Uniswap V3")]
    InvalidHex(hex::FromHexError),
    #[error("decoded value exceeded supported precision")]
    PrecisionOverflow,
    #[error("system clock is unavailable")]
    ClockUnavailable,
}
