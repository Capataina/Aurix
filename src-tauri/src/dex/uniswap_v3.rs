use std::time::{SystemTime, UNIX_EPOCH};

use num_bigint::BigUint;
use num_traits::ToPrimitive;
use thiserror::Error;

use crate::config::{PairConfig, Token};
use crate::ethereum::client::{EthereumRpcClient, EthereumRpcError};
use crate::market::types::PriceSnapshot;

const SLOT0_CALLDATA: &str = "0x3850c7bd";

/// Fetches a price snapshot from a Uniswap V3 pool for the supplied pair.
///
/// Inputs:
///   - `rpc_client`: a configured Ethereum JSON-RPC client.
///   - `pair`: the active pair configuration; supplies tokens (with decimals)
///     and the human-readable label.
///   - `dex_name`: stable identifier surfaced as `PriceSnapshot.dex_name`.
///   - `pool_address`: 20-byte hex address of the V3 pool.
///   - `fee_tier_bps`: pool fee tier in basis points (5, 30, 100, …).
///   - `chain_label`: the chain string surfaced on the snapshot.
/// Outputs: a normalised price snapshot containing the decoded base-in-quote price.
/// Errors: returned when the RPC call fails, the slot0 payload is malformed,
/// or the decoded price overflows f64 precision.
/// Side effects: performs a single read-only `eth_call`.
pub async fn fetch_snapshot(
    rpc_client: &EthereumRpcClient,
    pair: &PairConfig,
    dex_name: &str,
    pool_address: &str,
    fee_tier_bps: u16,
    chain_label: &str,
) -> Result<PriceSnapshot, UniswapV3Error> {
    let slot0_hex = rpc_client.eth_call(pool_address, SLOT0_CALLDATA).await?;
    let sqrt_price_x96 = decode_sqrt_price_x96(&slot0_hex)?;
    let price_usd = derive_price_base_in_quote(&sqrt_price_x96, &pair.base, &pair.quote)?;
    let fetched_at_unix_ms = current_unix_timestamp_ms()?;

    Ok(PriceSnapshot {
        chain: chain_label.to_string(),
        dex_name: dex_name.to_string(),
        pair_label: pair.label.clone(),
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

/// Derives the base-in-quote spot price from a Uniswap V3 `sqrtPriceX96`.
///
/// V3 pools order tokens by ascending hex address: `token0` is whichever of
/// (base, quote) has the lower address. The raw price encoded by sqrtPriceX96
/// is `token1` measured in `token0`, in raw integer units.
///
/// We compute, in BigUint to preserve the 192-bit shift precision:
///   * if base == token0: `result = sqrt^2 * 10^(d0 - d1) / 2^192`
///     (price of token0 expressed in token1, i.e. base in quote)
///   * if base == token1: `result = 2^192 * 10^(d1 - d0) / sqrt^2`
///     (price of token1 expressed in token0, i.e. base in quote)
fn derive_price_base_in_quote(
    sqrt_price_x96: &BigUint,
    base: &Token,
    quote: &Token,
) -> Result<f64, UniswapV3Error> {
    let base_is_token0 = base.is_lower_than(quote);
    let sqrt_squared: BigUint = sqrt_price_x96.pow(2u32);
    let two_192: BigUint = BigUint::from(1u8) << 192;

    let (token0_decimals, token1_decimals) = if base_is_token0 {
        (base.decimals as i32, quote.decimals as i32)
    } else {
        (quote.decimals as i32, base.decimals as i32)
    };

    // (raw_numerator, raw_denominator, scaling exponent) before applying the
    // 10^|exp| factor. The exponent is signed so we route the scale into
    // either side as needed and avoid pow(0u32).
    let (raw_numerator, raw_denominator, exponent) = if base_is_token0 {
        (sqrt_squared, two_192, token0_decimals - token1_decimals)
    } else {
        (two_192, sqrt_squared, token1_decimals - token0_decimals)
    };

    let abs_exp = exponent.unsigned_abs();
    let scale = if abs_exp == 0 {
        BigUint::from(1u8)
    } else {
        BigUint::from(10u64).pow(abs_exp)
    };

    let (final_numerator, final_denominator) = if exponent >= 0 {
        (raw_numerator * scale, raw_denominator)
    } else {
        (raw_numerator, raw_denominator * scale)
    };

    let numerator_f64 = final_numerator
        .to_f64()
        .ok_or(UniswapV3Error::PrecisionOverflow)?;
    let denominator_f64 = final_denominator
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

    duration
        .as_millis()
        .try_into()
        .map_err(|_| UniswapV3Error::ClockUnavailable)
}

/// Errors encountered while reading or decoding a Uniswap V3 pool slot0 word.
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
