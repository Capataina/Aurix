use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

use crate::ethereum::client::{EthereumRpcClient, EthereumRpcError};
use crate::market::types::PriceSnapshot;

const UNISWAP_V2_FACTORY: &str = "0x5C69bEe701ef814A2B6a3EDD4B1652CB9cc5aA6f";
const SUSHISWAP_V2_FACTORY: &str = "0xC0AEe478e3658e2610c5F7A4A2E1777Ce9e4f2Ac";
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2";
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const GET_PAIR_SELECTOR: &str = "0xe6a43905";
const GET_RESERVES_SELECTOR: &str = "0x0902f1ac";
const TOKEN0_SELECTOR: &str = "0x0dfe1681";

/// Fetches the current WETH/USDC price from the canonical Uniswap V2 pool.
pub async fn fetch_uniswap_v2_snapshot(
    rpc_client: &EthereumRpcClient,
) -> Result<PriceSnapshot, UniswapV2Error> {
    fetch_v2_snapshot(
        rpc_client,
        "Uniswap V2",
        UNISWAP_V2_FACTORY,
        "reserve ratio spot price",
        30,
    )
    .await
}

/// Fetches the current WETH/USDC price from the canonical SushiSwap V2 pool.
pub async fn fetch_sushiswap_snapshot(
    rpc_client: &EthereumRpcClient,
) -> Result<PriceSnapshot, UniswapV2Error> {
    fetch_v2_snapshot(
        rpc_client,
        "SushiSwap",
        SUSHISWAP_V2_FACTORY,
        "reserve ratio spot price",
        30,
    )
    .await
}

async fn fetch_v2_snapshot(
    rpc_client: &EthereumRpcClient,
    dex_name: &str,
    factory_address: &str,
    price_source_label: &str,
    fee_tier_bps: u16,
) -> Result<PriceSnapshot, UniswapV2Error> {
    let pair_address = resolve_pair_address(rpc_client, factory_address).await?;
    let token0_address = read_token0_address(rpc_client, &pair_address).await?;
    let reserves = read_reserves(rpc_client, &pair_address).await?;
    let price_usd = derive_price_usd(&token0_address, reserves)?;
    let fetched_at_unix_ms = current_unix_timestamp_ms()?;

    Ok(PriceSnapshot {
        chain: "Ethereum Mainnet".to_string(),
        dex_name: dex_name.to_string(),
        pair_label: "WETH / USDC".to_string(),
        price_usd,
        pool_address: pair_address,
        fee_tier_bps,
        price_source_label: price_source_label.to_string(),
        fetched_at_unix_ms,
    })
}

async fn resolve_pair_address(
    rpc_client: &EthereumRpcClient,
    factory_address: &str,
) -> Result<String, UniswapV2Error> {
    let calldata = format!(
        "{selector}{token0}{token1}",
        selector = GET_PAIR_SELECTOR.trim_start_matches("0x"),
        token0 = encode_address(USDC_ADDRESS),
        token1 = encode_address(WETH_ADDRESS),
    );

    let response = rpc_client.eth_call(factory_address, &format!("0x{calldata}")).await?;
    let pair_address = decode_address_word(&response)?;

    if pair_address == "0x0000000000000000000000000000000000000000" {
        return Err(UniswapV2Error::MissingPair);
    }

    Ok(pair_address)
}

async fn read_token0_address(
    rpc_client: &EthereumRpcClient,
    pair_address: &str,
) -> Result<String, UniswapV2Error> {
    let response = rpc_client.eth_call(pair_address, TOKEN0_SELECTOR).await?;
    decode_address_word(&response)
}

async fn read_reserves(
    rpc_client: &EthereumRpcClient,
    pair_address: &str,
) -> Result<(u128, u128), UniswapV2Error> {
    let response = rpc_client.eth_call(pair_address, GET_RESERVES_SELECTOR).await?;
    let normalised_hex = response.trim_start_matches("0x");

    if normalised_hex.len() < 128 {
        return Err(UniswapV2Error::MalformedReserves);
    }

    let reserve0 = decode_u128_word(&normalised_hex[..64])?;
    let reserve1 = decode_u128_word(&normalised_hex[64..128])?;

    Ok((reserve0, reserve1))
}

fn derive_price_usd(
    token0_address: &str,
    reserves: (u128, u128),
) -> Result<f64, UniswapV2Error> {
    let reserve0 = reserves.0 as f64;
    let reserve1 = reserves.1 as f64;

    if reserve0 == 0.0 || reserve1 == 0.0 {
        return Err(UniswapV2Error::MalformedReserves);
    }

    let token0_is_usdc = token0_address.eq_ignore_ascii_case(USDC_ADDRESS);

    if token0_is_usdc {
        Ok((reserve0 / reserve1) * 10_f64.powi(12))
    } else {
        Ok((reserve1 / reserve0) * 10_f64.powi(12))
    }
}

fn decode_address_word(word_hex: &str) -> Result<String, UniswapV2Error> {
    let normalised_hex = word_hex.trim_start_matches("0x");

    if normalised_hex.len() < 64 {
        return Err(UniswapV2Error::MalformedAddress);
    }

    let address_hex = &normalised_hex[24..64];
    Ok(format!("0x{address_hex}"))
}

fn decode_u128_word(word_hex: &str) -> Result<u128, UniswapV2Error> {
    u128::from_str_radix(word_hex, 16).map_err(|_| UniswapV2Error::MalformedReserves)
}

fn encode_address(address: &str) -> String {
    format!("{:0>64}", address.trim_start_matches("0x"))
}

fn current_unix_timestamp_ms() -> Result<u64, UniswapV2Error> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| UniswapV2Error::ClockUnavailable)?;

    duration
        .as_millis()
        .try_into()
        .map_err(|_| UniswapV2Error::ClockUnavailable)
}

/// Errors encountered while reading Uniswap V2-style liquidity pools.
#[derive(Debug, Error)]
pub enum UniswapV2Error {
    #[error(transparent)]
    Rpc(#[from] EthereumRpcError),
    #[error("requested pair does not exist on the selected factory")]
    MissingPair,
    #[error("pair contract returned malformed reserves")]
    MalformedReserves,
    #[error("pair contract returned a malformed address")]
    MalformedAddress,
    #[error("system clock is unavailable")]
    ClockUnavailable,
}
