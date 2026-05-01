//! Exercise: Simulate V2 Swap
//!
//! Implement a function that simulates a Uniswap V2 swap given the current pool
//! reserves and a swap input amount. Apply the constant-product formula `x * y = k`
//! correctly, including the 0.30% fee, and return both the output amount and the
//! new pool state.
//!
//! Goal:
//!     Implement `simulate_v2_swap(reserves: (u128, u128), amount_in: u128, token_in_is_token0: bool) -> (u128, (u128, u128))`
//!     that returns (amount_out, new_reserves) for a V2 swap.
//!
//! Starting Point:
//!     A skeleton function below with the correct signature.
//!     A `main` that runs three test cases against expected outputs.
//!
//! Tasks:
//!     - [ ] Implement simulate_v2_swap with V2's x*y=k math + 0.30% fee
//!     - [ ] Verify against the three test cases below
//!     - [ ] Compute slippage for each test case and confirm it grows non-linearly with size
//!     - [ ] Add error handling for edge cases (zero reserves, swap larger than reserves)
//!
//! Hints:
//!     1. The fee mechanism: only 99.7% of amount_in actually enters the formula.
//!        amount_in_with_fee = (amount_in * 997) / 1000
//!     2. The output formula: amount_out = (amount_in_with_fee * reserve_out) / (reserve_in + amount_in_with_fee)
//!     3. Use u128 carefully — for typical pool sizes (10s of WETH, 100,000s of USDC)
//!        you're safe, but a large swap could overflow if you're not careful. Consider
//!        using u256 (via `primitive_types::U256`) if you need more headroom.
//!     4. token_in_is_token0 is the flag that tells you which side is being swapped IN.
//!        If true: amount_in increases reserve0; output is from reserve1.
//!        If false: amount_in increases reserve1; output is from reserve0.
//!
//! Expected Behaviour:
//!     Test case 1: pool (100 WETH, 300,000 USDC), swap 1 WETH in (token1).
//!         Expected output: ~2,961.41 USDC (with fee)
//!     Test case 2: pool (100 WETH, 300,000 USDC), swap 5 WETH in (token1).
//!         Expected output: ~14,242.86 USDC (with fee). Slippage: ~5%
//!     Test case 3: pool (100 WETH, 300,000 USDC), swap 10,000 USDC in (token0).
//!         Expected output: ~3.226 WETH (with fee).
//!
//! Related Files:
//!     Concepts:        learning/concepts/core/amm-mechanics-v2-and-v3.md
//!     Project systems: src-tauri/src/dex/uniswap_v2.rs
//!     Foundations:     learning/exercises/foundations/amm-constant-product-by-hand.md

// ── Your implementation starts here ──────────────────────────────────────────

fn simulate_v2_swap(
    reserves: (u128, u128),
    amount_in: u128,
    token_in_is_token0: bool,
) -> (u128, (u128, u128)) {
    // TODO: implement V2 constant-product swap with 0.30% fee
    todo!("Implement V2 swap simulation")
}

// ── Test harness ──────────────────────────────────────────────────────────────

fn main() {
    // Pool: 100 WETH, 300,000 USDC (with appropriate decimals applied)
    // We'll use raw integer reserves: WETH has 18 decimals, USDC has 6
    let weth_reserves: u128 = 100 * 10u128.pow(18);
    let usdc_reserves: u128 = 300_000 * 10u128.pow(6);

    // Convention: token0 = USDC, token1 = WETH
    let pool = (usdc_reserves, weth_reserves);

    // Test 1: swap 1 WETH for USDC
    let amount_in = 1 * 10u128.pow(18); // 1 WETH in raw units
    let (out, new_pool) = simulate_v2_swap(pool, amount_in, false); // token1 in
    println!("Test 1: 1 WETH in → {} USDC out", out / 10u128.pow(6));
    println!("  Slippage vs marquee ($3000): {:.2}%",
             (3000.0 - (out as f64 / 10u128.pow(6) as f64)) / 3000.0 * 100.0);

    // Test 2: swap 5 WETH for USDC
    let amount_in = 5 * 10u128.pow(18);
    let (out, _) = simulate_v2_swap(pool, amount_in, false);
    println!("Test 2: 5 WETH in → {} USDC out", out / 10u128.pow(6));
    println!("  Slippage vs marquee ($15000 for 5 WETH): {:.2}%",
             (15000.0 - (out as f64 / 10u128.pow(6) as f64)) / 15000.0 * 100.0);

    // Test 3: swap 10,000 USDC for WETH
    let amount_in = 10_000 * 10u128.pow(6);
    let (out, _) = simulate_v2_swap(pool, amount_in, true); // token0 in
    println!("Test 3: 10000 USDC in → {} WETH out", out / 10u128.pow(15));
    // Should be ~3.226 WETH; we print in milliWETH (10^15 units) for readability
}
