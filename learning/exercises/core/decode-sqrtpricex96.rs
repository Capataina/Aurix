//! Exercise: Decode sqrtPriceX96
//!
//! The Uniswap V3 pool stores its current price as `sqrtPriceX96` — the square
//! root of the price (token1/token0), multiplied by 2^96, stored as a 160-bit
//! unsigned integer. Aurix decodes this in `src-tauri/src/dex/uniswap_v3.rs`
//! using the BigUint type from `num-bigint`.
//!
//! Your task: implement the decode from a known sqrtPriceX96 to a USD price,
//! validate against a hand-computed reference, and explain in a comment what
//! the decimal scaling is doing.
//!
//! Goal:
//!     Implement `decode_v3_price(sqrt_price_x96: &BigUint) -> f64` that returns
//!     the WETH/USDC price (USDC per WETH, scaled to human dollars) given a
//!     V3 pool's sqrtPriceX96 value.
//!
//! Starting Point:
//!     A skeleton function below with the correct signature.
//!     A test fixture: a known sqrtPriceX96 value with the expected price.
//!     A `main` that runs your function and prints the result.
//!
//! Tasks:
//!     - [ ] Implement decode_v3_price using BigUint arithmetic
//!     - [ ] Verify against the test fixture (within 0.01 USD tolerance)
//!     - [ ] Add a comment explaining what the 10^12 scaling does
//!     - [ ] Add a second test case with a different sqrtPriceX96 value
//!
//! Hints:
//!     1. The math is: P = (sqrtPriceX96² × 10^(decimals_diff)) / 2^192
//!        ...but you need it in the form (numerator / denominator) where both
//!        are BigUint. Compute the numerator first, then divide.
//!     2. For WETH/USDC: token0 is USDC (6 decimals), token1 is WETH (18 decimals).
//!        decimals_diff = 18 - 6 = 12.
//!        The scaling adjusts for the fact that USDC has fewer decimals than WETH —
//!        without it, the price would be 12 orders of magnitude wrong.
//!     3. Look at src-tauri/src/dex/uniswap_v3.rs::derive_weth_price_usd for the
//!        production implementation. Try to match its math.
//!
//! Expected Behaviour:
//!     For sqrtPriceX96 = 1457652066949847389969617 (a real V3 mainnet value),
//!     the WETH/USDC price should be approximately $3,000-$3,100.
//!     The actual value depends on when the snapshot was taken — verify your
//!     output is in this ballpark and reasonable for an ETH price.
//!
//! Related Files:
//!     Concepts:        learning/concepts/advanced/uniswap-v3-tick-mathematics.md
//!     Project systems: src-tauri/src/dex/uniswap_v3.rs
//!     Paths:           learning/paths/vector-prep-path.md (Vector A prep)

use num_bigint::BigUint;
use num_traits::ToPrimitive;
use std::str::FromStr;

const TOKEN0_DECIMALS: u32 = 6;   // USDC
const TOKEN1_DECIMALS: u32 = 18;  // WETH

// ── Your implementation starts here ──────────────────────────────────────────

fn decode_v3_price(sqrt_price_x96: &BigUint) -> f64 {
    // TODO: implement the decode
    todo!("Implement V3 sqrtPriceX96 → price decoding")
}

// ── Test harness ──────────────────────────────────────────────────────────────

fn main() {
    // A real sqrtPriceX96 value from V3 mainnet (representative)
    let test_sqrt_price = BigUint::from_str("1457652066949847389969617").unwrap();

    let price = decode_v3_price(&test_sqrt_price);

    println!("Decoded price: ${:.2}", price);

    // Add your own test case here:
    // let test_2 = BigUint::from_str("...").unwrap();
    // let price_2 = decode_v3_price(&test_2);
    // println!("Decoded price 2: ${:.2}", price_2);
}
