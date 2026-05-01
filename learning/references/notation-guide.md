# Notation Guide

Reference for symbols and notation used throughout the archive's mathematical sections.

## AMM Math

| Symbol | Meaning | Used in |
|---|---|---|
| `x` | Reserve of token0 in an AMM pool | V2 + V3 |
| `y` | Reserve of token1 in an AMM pool | V2 + V3 |
| `k` | Constant product invariant: `x × y = k` | V2 (and V3 within active range) |
| `Δx` | Amount of token0 added by a trader | V2 swap math |
| `Δy` | Amount of token1 received by a trader | V2 swap math |
| `P` | Implied price (token1 per token0) | V2 + V3 |
| `√P` | Square root of price; V3 stores this rather than `P` | V3 |
| `√P_X96` | `√P × 2^96`, stored as Q64.96 fixed-point | V3 |
| `L` | V3 pool liquidity; `L = √(x × y)` (geometric mean of reserves) | V3 |
| `tick i` | Discrete price level in V3 | V3 |
| `tick_spacing` | Distance between selectable ticks (10, 60, 200 by fee tier) | V3 |
| `r` | Price ratio: `current_price / entry_price` (used in IL formulas) | LP analysis |

## Statistics

| Symbol | Meaning | Used in |
|---|---|---|
| `μ` | Mean | Stats |
| `σ` | Standard deviation | Vol |
| `σ²` | Variance | Vol |
| `ρ` | Correlation coefficient | Multi-asset |
| `β` | Beta (regression slope vs market) | CAPM-style |
| `α` | Alpha (return above benchmark) | Active strategies |
| `r_t` | Return at time t | Time series |
| `IR` | Information Ratio | Active strategies |
| `VaR` | Value-at-Risk | Risk |
| `CVaR` | Conditional VaR / Expected Shortfall | Risk |
| `ECE` | Expected Calibration Error | ML |

## Crypto / DeFi Conventions

| Notation | Meaning |
|---|---|
| `0x...` | Hexadecimal value (typically a 20-byte address or 32-byte calldata word) |
| `WETH/USDC` | Trading pair; first token is base, second is quote (price = USDC per WETH) |
| `5bps` | 5 basis points = 0.05% (V3 fee tier) |
| `30bps` | 30 basis points = 0.30% (V3 fee tier; also V2's only fee tier) |
| `1bps` | 1 basis point = 0.01% (the bps abbreviation; not an actual V3 fee tier) |
| `gwei` | 10⁻⁹ ETH; standard denomination for gas prices |
| `wei` | 10⁻¹⁸ ETH; the smallest indivisible unit |
| `T+0`, `T+1` | "At time T," "one block after T" |

## Aurix-Specific Identifiers

| Identifier | Meaning |
|---|---|
| `dex_name` | String label for venue (e.g. `"Uniswap V3 5bps"`) |
| `pair_label` | String label for trading pair (currently always `"WETH / USDC"`) |
| `price_usd` | f64 price in USD per WETH (USDC unit ≈ USD) |
| `gas_price_gwei` | f64 current gas price in gwei |
| `fee_tier_bps` | u16 fee tier in basis points |
| `pool_address` | String hex address of the pool contract |
| `fetched_at_unix_ms` | u64 milliseconds since UNIX epoch |
| `sqrtPriceX96` | BigUint representation of √price × 2^96 |
| `slot0` | V3 pool's primary state-reading function |

## Code Conventions

| Convention | Meaning |
|---|---|
| `BigUint` | Rust's `num-bigint::BigUint` arbitrary-precision unsigned integer |
| `u128` | Rust 128-bit unsigned (sufficient for V2 reserves) |
| `f64` | IEEE 754 double; used for prices and gas across the cross-runtime boundary |
| Selector `0x...` | First 4 bytes of `keccak256(function_signature)` |
| Padded address | 20-byte address left-padded with 12 zero bytes to fill a 32-byte word |

## Time and Cadence

| Reference | Aurix's value |
|---|---|
| Polling cadence | 1 Hz (`REFRESH_INTERVAL_MS = 1_000`) |
| History limit | 100 samples (`HISTORY_LIMIT = 100`) |
| Persistence window for insights | 4 samples (`PERSISTENCE_WINDOW = 4`) |
| Baseline window for insights | 20 samples (`BASELINE_WINDOW = 20`) |
| Short window for insights | 5 samples (`SHORT_WINDOW = 5`) |
| Block time on Ethereum | ~12 seconds |
| Gas estimate per swap | 220,000 (`GAS_UNITS_ESTIMATE = 220_000`) |

## Greek Letters Used

For convenience when reading mathematical sections aloud:

| Letter | Lowercase | Uppercase | Used for |
|---|---|---|---|
| alpha | α | Α | Excess return; learning rate |
| beta | β | Β | Market sensitivity; regression slope |
| gamma | γ | Γ | Discount factor (RL); options gamma |
| delta | δ | Δ | Change in (Δx = "change in x") |
| epsilon | ε | Ε | Small quantity; noise term; ε-greedy |
| theta | θ | Θ | Model parameters (general); options theta |
| lambda | λ | Λ | Regularization strength; eigenvalue |
| mu | μ | Μ | Mean |
| pi | π | Π | Policy (RL); ratio constant |
| rho | ρ | Ρ | Correlation |
| sigma | σ | Σ | Standard deviation; sum |
| tau | τ | Τ | Time constant; threshold |
| phi | φ | Φ | Feature map; CDF of standard normal |

## Related Files

- `GLOSSARY.md` — terminology with full definitions
- `references/status-conventions.md` — labels for current/planned/superseded
