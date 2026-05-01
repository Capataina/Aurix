//! Solution: Simulate V2 Swap
//!
//! Implements V2's `x * y = k` math with the 0.30% fee.
//!
//! The fee mechanism: only 99.7% of amount_in actually enters the pool's
//! constant-product formula; the remaining 0.3% stays in the pool as fees,
//! distributed pro-rata to LPs.
//!
//! The output formula: amount_out = (amount_in_with_fee × reserve_out)
//!                                  / (reserve_in + amount_in_with_fee)

// ── Solution ──────────────────────────────────────────────────────────────────

fn simulate_v2_swap(
    reserves: (u128, u128),
    amount_in: u128,
    token_in_is_token0: bool,
) -> (u128, (u128, u128)) {
    let (reserve_in, reserve_out) = if token_in_is_token0 {
        (reserves.0, reserves.1)
    } else {
        (reserves.1, reserves.0)
    };

    // Apply the 0.30% fee: amount_in_with_fee = amount_in * 997 / 1000
    let amount_in_with_fee = amount_in.checked_mul(997).expect("overflow") / 1000;

    // V2 swap formula: amount_out = (amount_in_with_fee * reserve_out)
    //                              / (reserve_in + amount_in_with_fee)
    let numerator = amount_in_with_fee.checked_mul(reserve_out).expect("overflow in numerator");
    let denominator = reserve_in.checked_add(amount_in_with_fee).expect("overflow in denominator");
    let amount_out = numerator / denominator;

    // New reserves: input side gains the FULL amount_in (including the fee
    // portion that stayed in the pool); output side loses amount_out
    let new_reserves = if token_in_is_token0 {
        (reserves.0 + amount_in, reserves.1 - amount_out)
    } else {
        (reserves.0 - amount_out, reserves.1 + amount_in)
    };

    (amount_out, new_reserves)
}

fn main() {
    let weth_reserves: u128 = 100 * 10u128.pow(18);
    let usdc_reserves: u128 = 300_000 * 10u128.pow(6);
    let pool = (usdc_reserves, weth_reserves);

    // Test 1: 1 WETH for USDC
    let amount_in = 1 * 10u128.pow(18);
    let (out, _) = simulate_v2_swap(pool, amount_in, false);
    let usdc_out = out as f64 / 10u128.pow(6) as f64;
    println!("Test 1: 1 WETH in → {:.2} USDC out", usdc_out);
    println!("  Slippage vs marquee ($3,000): {:.2}%",
             (3000.0 - usdc_out) / 3000.0 * 100.0);

    // Test 2: 5 WETH for USDC
    let amount_in = 5 * 10u128.pow(18);
    let (out, _) = simulate_v2_swap(pool, amount_in, false);
    let usdc_out = out as f64 / 10u128.pow(6) as f64;
    println!("Test 2: 5 WETH in → {:.2} USDC out", usdc_out);
    println!("  Slippage vs marquee ($15,000 for 5 WETH): {:.2}%",
             (15000.0 - usdc_out) / 15000.0 * 100.0);

    // Test 3: 10,000 USDC for WETH
    let amount_in = 10_000 * 10u128.pow(6);
    let (out, _) = simulate_v2_swap(pool, amount_in, true);
    let weth_out = out as f64 / 10u128.pow(18) as f64;
    println!("Test 3: 10,000 USDC in → {:.4} WETH out", weth_out);
}
