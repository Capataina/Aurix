use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

use crate::config::{PairConfig, Token};
use crate::ethereum::client::{EthereumRpcClient, EthereumRpcError};
use crate::market::types::PriceSnapshot;

const GET_PAIR_SELECTOR: &str = "0xe6a43905";
const GET_RESERVES_SELECTOR: &str = "0x0902f1ac";

/// Fetches a price snapshot from a Uniswap V2 / V2-fork pool for the supplied pair.
///
/// Inputs:
///   - `rpc_client`: a configured Ethereum JSON-RPC client.
///   - `pair`: the active pair configuration; supplies tokens (with decimals)
///     and the human-readable label.
///   - `dex_name`: stable identifier surfaced as `PriceSnapshot.dex_name`.
///   - `factory_address`: 20-byte hex factory address. The pair address is
///     resolved at fetch time via `getPair(base, quote)`.
///   - `fee_tier_bps`: display fee tier in basis points (V2 fork fees are
///     typically 30 bps).
///   - `chain_label`: the chain string surfaced on the snapshot.
/// Outputs: a normalised price snapshot containing the decoded base-in-quote price.
/// Errors: returned when the factory has no pair, the reserves payload is
/// malformed, or the RPC transport fails.
/// Side effects: performs two read-only `eth_call`s (factory.getPair, pair.getReserves).
pub async fn fetch_snapshot(
    rpc_client: &EthereumRpcClient,
    pair: &PairConfig,
    dex_name: &str,
    factory_address: &str,
    fee_tier_bps: u16,
    chain_label: &str,
) -> Result<PriceSnapshot, UniswapV2Error> {
    let pair_address =
        resolve_pair_address(rpc_client, factory_address, &pair.base, &pair.quote).await?;
    let reserves = read_reserves(rpc_client, &pair_address).await?;
    let price_usd = derive_price_base_in_quote(&pair.base, &pair.quote, reserves)?;
    let fetched_at_unix_ms = current_unix_timestamp_ms()?;

    Ok(PriceSnapshot {
        chain: chain_label.to_string(),
        dex_name: dex_name.to_string(),
        pair_label: pair.label.clone(),
        price_usd,
        pool_address: pair_address,
        fee_tier_bps,
        price_source_label: "reserve ratio spot price".to_string(),
        fetched_at_unix_ms,
    })
}

async fn resolve_pair_address(
    rpc_client: &EthereumRpcClient,
    factory_address: &str,
    base: &Token,
    quote: &Token,
) -> Result<String, UniswapV2Error> {
    // Pool ordering is canonical (token0 has lower address) but the factory's
    // `getPair(a, b)` is symmetric: it returns the same address regardless of
    // argument order. Passing (base, quote) directly is therefore fine.
    let calldata = format!(
        "{selector}{token_a}{token_b}",
        selector = GET_PAIR_SELECTOR.trim_start_matches("0x"),
        token_a = encode_address(&base.address),
        token_b = encode_address(&quote.address),
    );

    let response = rpc_client
        .eth_call(factory_address, &format!("0x{calldata}"))
        .await?;
    let pair_address = decode_address_word(&response)?;

    if pair_address == "0x0000000000000000000000000000000000000000" {
        return Err(UniswapV2Error::MissingPair);
    }

    Ok(pair_address)
}

async fn read_reserves(
    rpc_client: &EthereumRpcClient,
    pair_address: &str,
) -> Result<(u128, u128), UniswapV2Error> {
    let response = rpc_client
        .eth_call(pair_address, GET_RESERVES_SELECTOR)
        .await?;
    let normalised_hex = response.trim_start_matches("0x");

    if normalised_hex.len() < 128 {
        return Err(UniswapV2Error::MalformedReserves);
    }

    let reserve0 = decode_u128_word(&normalised_hex[..64])?;
    let reserve1 = decode_u128_word(&normalised_hex[64..128])?;

    Ok((reserve0, reserve1))
}

/// Derives the base-in-quote spot price from V2 reserves.
///
/// V2 pools store tokens in canonical order (token0 has lower address). The
/// raw reserve ratio `reserve1 / reserve0` is "raw token1 per raw token0";
/// scaling by `10^(d0 - d1)` converts that into "human token0 in human token1".
/// We then invert if the base happens to be token1.
fn derive_price_base_in_quote(
    base: &Token,
    quote: &Token,
    reserves: (u128, u128),
) -> Result<f64, UniswapV2Error> {
    let reserve0 = reserves.0 as f64;
    let reserve1 = reserves.1 as f64;

    if reserve0 == 0.0 || reserve1 == 0.0 {
        return Err(UniswapV2Error::MalformedReserves);
    }

    let base_is_token0 = base.is_lower_than(quote);
    let (d0, d1) = if base_is_token0 {
        (base.decimals as i32, quote.decimals as i32)
    } else {
        (quote.decimals as i32, base.decimals as i32)
    };

    // raw_token1_per_raw_token0 = reserve1 / reserve0
    // human_token0_in_human_token1 = (reserve1 / reserve0) * 10^(d0 - d1)
    let raw_t1_per_raw_t0 = reserve1 / reserve0;
    let scale = 10f64.powi(d0 - d1);
    let token0_in_token1 = raw_t1_per_raw_t0 * scale;

    if token0_in_token1 == 0.0 {
        return Err(UniswapV2Error::MalformedReserves);
    }

    if base_is_token0 {
        Ok(token0_in_token1)
    } else {
        Ok(1.0 / token0_in_token1)
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
