//! Solution: Decode sqrtPriceX96
//!
//! Working implementation matching Aurix's `src-tauri/src/dex/uniswap_v3.rs`
//! pattern. The 10^12 scaling adjusts for the decimal asymmetry between
//! USDC (6 decimals) and WETH (18 decimals).

use num_bigint::BigUint;
use num_traits::ToPrimitive;
use std::str::FromStr;

const TOKEN0_DECIMALS: u32 = 6;   // USDC
const TOKEN1_DECIMALS: u32 = 18;  // WETH

// ── Solution ──────────────────────────────────────────────────────────────────

fn decode_v3_price(sqrt_price_x96: &BigUint) -> f64 {
    // Numerator: 2^192 × 10^(token1_decimals - token0_decimals) = 2^192 × 10^12
    // The 2^192 factor inverts the (× 2^96)^2 scaling in sqrtPriceX96
    // The 10^12 factor adjusts for USDC having 6 decimals vs WETH's 18
    let numerator: BigUint = (BigUint::from(1u8) << 192)
        * BigUint::from(10u64).pow(TOKEN1_DECIMALS - TOKEN0_DECIMALS);

    // Denominator: sqrtPriceX96 squared (recovers price² × 2^192)
    let denominator: BigUint = sqrt_price_x96.pow(2u32);

    // Convert to f64 for the final division (precision loss acceptable for display)
    let numerator_f64 = numerator.to_f64().expect("numerator overflow");
    let denominator_f64 = denominator.to_f64().expect("denominator overflow");

    if denominator_f64 == 0.0 {
        panic!("sqrtPriceX96 is zero");
    }

    numerator_f64 / denominator_f64
}

fn main() {
    let test_sqrt_price = BigUint::from_str("1457652066949847389969617").unwrap();
    let price = decode_v3_price(&test_sqrt_price);
    println!("Decoded price: ${:.2}", price);

    // Sanity check: WETH should be in the $1,000-$10,000 range
    assert!(price > 100.0 && price < 100_000.0,
            "Price {} is outside reasonable WETH range", price);
}

// Run with: rustc decode-sqrtpricex96.rs --edition 2021 --extern num_bigint --extern num_traits
// Or add to a Cargo project with num-bigint = "0.4" and num-traits = "0.2"
