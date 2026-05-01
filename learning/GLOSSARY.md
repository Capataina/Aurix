# Glossary

Terminology used throughout the Aurix project, the DeFi domain, and the surrounding Ethereum ecosystem. Entries are alphabetical. Each entry includes a precise definition, a plain-language interpretation, a concrete example or project link where useful, and a `See:` line pointing to deeper coverage in the archive.

This glossary is infrastructure. It exists to remove ambiguity, not to be exhaustive — a term that appears once and isn't load-bearing for understanding doesn't need an entry.

---

### ABI (Application Binary Interface)

The encoding standard that defines how function calls and return values are serialised when interacting with Ethereum smart contracts. To call a function on a contract, you encode the function selector (4 bytes derived from the function signature) followed by the arguments in 32-byte words; the contract returns its result encoded the same way.

Plain language: ABI is the "calling convention" for Ethereum contracts — like how a C function call passes arguments on the stack in a specific order, ABI defines the byte layout for cross-contract calls and JSON-RPC reads.

Aurix relevance: Aurix hand-crafts its ABI encoding rather than using `ethers-rs`. The selectors `0x3850c7bd` (V3 `slot0()`), `0xe6a43905` (V2 factory `getPair()`), `0x0902f1ac` (V2 pair `getReserves()`), and `0x0dfe1681` (V2 pair `token0()`) are first-4-bytes-of-keccak256 of each function signature, hard-coded as constants in the DEX adapters.

See: `concepts/foundations/markets-and-prices.md`, `project/architecture/cross-runtime-contract.md`

---

### AMM (Automated Market Maker)

A class of decentralised exchange that prices assets via a math formula on a pool of tokens, rather than through an order book matching buyers and sellers.

Plain language: instead of "I want to sell at $3,000" and "I want to buy at $2,995" being matched by an exchange, an AMM holds (say) 100 ETH and 300,000 USDC, and *anyone* can swap against the pool — the price comes from the ratio of the two piles plus a math formula. No matching engine needed.

Aurix relevance: every venue Aurix watches (Uniswap V3, Uniswap V2, SushiSwap) is an AMM. The 1 Hz polling loop reads pool state from each one and derives a price from that state.

See: `concepts/core/amm-mechanics-v2-and-v3.md`, `concepts/foundations/exchanges-orderbook-vs-amm.md`

---

### Arbitrage

The practice of profiting from price differences for the same asset across different venues. In DeFi, an arbitrageur buys cheap on one DEX, sells expensive on another, and pockets the difference (minus gas and slippage).

Plain language: if Uniswap V3 says 1 WETH = $3,000 and SushiSwap says 1 WETH = $3,008, you could buy 1 WETH on V3 for $3,000, sell on Sushi for $3,008, and walk away with $8 — minus the costs of executing both swaps in a single transaction.

Aurix relevance: Aurix's primary purpose is to *observe* arbitrage opportunities — the live spread between venues, the gas-adjusted profitability estimate, the persistence of sign across samples. It does not execute. The "Positive setup holding" insight surfaces when the cheapest-to-richest route stays profitable for at least 4 consecutive samples (`PERSISTENCE_WINDOW = 4` in `insights.ts`).

See: `concepts/core/arbitrage-and-cross-venue-equilibrium.md`, `project/systems/what-aurix-observes.md`

---

### Backtesting

Running a hypothetical strategy against historical data to compute "what would have happened" — what fees would have been earned, what drawdowns would have occurred, what the final return would have been.

Plain language: "if I had done X starting last March, where would I be now?" Backtesting is one of the cheapest forms of validation in finance because the historical record is fixed and the simulation is deterministic.

Aurix relevance: Vector A is a Uniswap V3 LP backtester — given historical swap events, simulate any chosen LP position's fees and impermanent loss with exact tick math.

See: `concepts/advanced/uniswap-v3-tick-mathematics.md`, `context/plans/vector-a-v3-lp-backtester.md`

---

### `BigUint`

The Rust crate `num-bigint`'s arbitrary-precision unsigned integer type. Used in Aurix to handle Ethereum's 256-bit values (like `sqrtPriceX96`) that exceed any native Rust integer type.

Plain language: Rust's `u128` only goes up to ~3.4×10³⁸. Ethereum routinely deals with 2²⁵⁶ values (~10⁷⁷). `BigUint` represents these as variable-length byte arrays and provides the arithmetic operations for them.

Aurix relevance: `dex/uniswap_v3.rs` uses `BigUint::from_bytes_be` to decode the 32-byte sqrtPriceX96 word, then computes `(2^192 × 10^12) / sqrt_price^2` to derive the WETH/USDC price.

See: `project/systems/insight-engine-anatomy.md` (V3 decode walkthrough)

---

### Block

The fundamental unit of Ethereum's chain — a batch of transactions that have been ordered, executed, and committed. New blocks are produced approximately every 12 seconds since the merge to proof-of-stake.

Plain language: think of a block as one "page" in Ethereum's ledger. Every 12 seconds, a new page gets written with the latest batch of transactions in a specific order.

See: `concepts/domain-patterns/the-mempool-public-vs-private.md`

---

### Block Builder

The entity responsible for constructing the next block — selecting which transactions from the mempool to include and in what order. After Ethereum's move to proof-of-stake and the rise of MEV-Boost, block building is often separated from block proposing (the validator who actually attests to the block).

Plain language: a block builder is the curator of the next page — they pick what goes in, in what order. This ordering authority is the source of MEV.

See: `concepts/domain-patterns/mev-and-transaction-ordering.md`

---

### Calldata

The raw bytes that comprise a transaction's input — the function selector plus encoded arguments. Calldata is what's stored on-chain when a transaction is committed.

Plain language: calldata is the actual byte stream the EVM executes. When you "call" a contract function, the EVM is just reading these bytes and routing based on the selector.

Aurix relevance: Aurix sends calldata directly via `eth_call` JSON-RPC requests — for example, `0x3850c7bd` is the calldata for V3 `slot0()`, sent with no arguments. The response is also raw bytes that Aurix decodes manually.

See: `project/architecture/cross-runtime-contract.md`

---

### Concentrated Liquidity

The Uniswap V3 innovation: liquidity providers choose a price range within which their capital is active. Inside the range, they earn fees on every swap and bear impermanent loss; outside the range, their position converts entirely to one token and earns nothing.

Plain language: V2 spreads your capital uniformly across all possible prices ($0 to ∞). V3 lets you say "only put my capital to work between $2,800 and $3,200" — you earn more fees per dollar in that range, but you earn zero outside it.

See: `concepts/core/amm-mechanics-v2-and-v3.md`, `concepts/advanced/uniswap-v3-tick-mathematics.md`

---

### Constant Product Formula

The Uniswap V2 (and SushiSwap, and most "x*y=k" AMMs) pricing rule: the product of the two reserves stays constant across swaps (modulo fees). If a pool has `x` of token A and `y` of token B with `x * y = k`, then any swap must preserve `k`.

Plain language: when you put 1 ETH into a pool with (100 ETH, 300,000 USDC), the pool now has 101 ETH; for `k` to stay constant at 30 million, USDC drops to 30,000,000 / 101 = 297,029.70. You receive the difference: 2,970.30 USDC. The formula handles slippage automatically.

See: `concepts/core/amm-mechanics-v2-and-v3.md`

---

### DEX (Decentralised Exchange)

A smart-contract-based exchange that operates without a central operator. Users trade peer-to-pool (in the AMM model) or peer-to-peer (in less common order-book DEXes), with all logic enforced by on-chain code.

Plain language: a "DEX" is a contract that lets anyone swap two tokens by following the contract's rules. Nobody runs it; it just exists on Ethereum.

Aurix relevance: Aurix watches four DEXes — Uniswap V3 (two fee tiers), Uniswap V2, SushiSwap.

See: `concepts/foundations/exchanges-orderbook-vs-amm.md`

---

### `dex_name`

The string identifier Aurix uses for each venue: `"Uniswap V3 5bps"`, `"Uniswap V3 30bps"`, `"Uniswap V2"`, `"SushiSwap"`. This string is the implicit cross-system identity key — both `VENUES` (in `ArbitragePage.tsx`) and `SERIES_META` (in `MarketChart.tsx`) look up venues by exact `dex_name` match.

Aurix relevance: renaming a `dex_name` in the backend silently breaks the frontend's price lookup (price renders as `$0.00`) and chart colour lookup (chart crashes on undefined). This contract is documented in `context/notes/dex-name-contract.md` and called out in the architecture's blast-radius table.

See: `project/decisions/no-ethers-rs-handcrafted-abi.md` (related convention)

---

### EOA (Externally Owned Account)

An Ethereum account controlled by a private key, as opposed to a smart contract account. Most user wallets (MetaMask, Rabby) are EOAs.

See: `concepts/domain-patterns/the-mempool-public-vs-private.md`

---

### ERC-20

The Ethereum standard interface for fungible tokens. Defines required functions (`transfer`, `transferFrom`, `approve`, `balanceOf`, `totalSupply`, `allowance`) and standard events (`Transfer`, `Approval`).

Plain language: ERC-20 is the contract template every token follows. If you implement these functions, your token works with every wallet, exchange, and DeFi protocol that supports ERC-20.

Aurix relevance: WETH and USDC (the two assets Aurix watches) are both ERC-20 tokens.

See: `concepts/foundations/tokens-and-the-defi-money-stack.md`

---

### `eth_call`

A JSON-RPC method that simulates a smart-contract function call without submitting a transaction. Read-only — no state changes, no gas paid by the caller.

Plain language: `eth_call` is the read primitive. You ask a contract "what would you return if I called this function?" and the node simulates it locally and returns the answer.

Aurix relevance: every Aurix backend operation is an `eth_call` — V3 `slot0()`, V2 `getReserves()`, factory `getPair()`. Aurix never submits a transaction (read-only by design).

See: `project/decisions/read-only-by-design.md`

---

### `eth_subscribe newPendingTransactions`

A JSON-RPC subscription method (over WebSocket) that pushes transaction hashes to subscribers as they enter the public mempool. The basis for any mempool-watching tool.

Aurix relevance: Vector B (MEV detector) would use this subscription as its primary input.

See: `concepts/advanced/mempool-mev-detection-mechanics.md`, `context/plans/vector-b-mev-detector.md`

---

### Flashbots

A protocol and ecosystem for submitting Ethereum transactions privately to a network of "searchers" who bundle them into block proposals, bypassing the public mempool. Used to avoid sandwich attacks (privacy from MEV bots) and to enable atomic multi-tx strategies (bundles).

Plain language: instead of broadcasting your tx publicly (where bots can sandwich it), you send it through Flashbots to a private channel; it appears in a block without anyone seeing it in the mempool.

See: `concepts/domain-patterns/the-mempool-public-vs-private.md`

---

### Gas

The unit Ethereum uses to price computation and storage. Every operation has a gas cost (e.g. an `ADD` opcode is 3 gas, a storage write is 20,000 gas). The total gas a transaction consumes, multiplied by the gas price (denominated in gwei), determines the ETH fee paid.

Plain language: gas is Ethereum's metering system. You pay for the work the network does on your behalf. A simple ETH transfer costs ~21,000 gas; a Uniswap V3 swap costs ~150,000-300,000 gas depending on tick crossings.

Aurix relevance: Aurix uses `GAS_UNITS_ESTIMATE = 220_000` as its assumption for swap gas cost when computing gas-adjusted profitability. The gas price is read once per tick via `eth_gasPrice` and converted to gwei.

See: `concepts/domain-patterns/gas-and-execution-costs.md`

---

### Gwei

A unit of ether equal to 10⁻⁹ ETH (or 10⁹ wei). Gas prices are typically denominated in gwei. "20 gwei" means each unit of gas costs 0.00000002 ETH.

Plain language: gwei is a convenient denomination for gas prices, the way "cents" is convenient for sub-dollar amounts.

See: `concepts/domain-patterns/gas-and-execution-costs.md`

---

### Impermanent Loss (IL)

The "loss" an LP experiences relative to simply holding the original token amounts, due to the AMM's auto-rebalancing as price moves. It's "impermanent" because the loss only crystallises if you withdraw at a different price than you entered.

Plain language: if you LP `1 ETH + 3,000 USDC` at price $3,000 and price doubles to $6,000, the AMM has been selling your ETH for USDC the whole way up. When you withdraw, you have less ETH than you started with. The dollar value at withdrawal is less than if you'd just held the original `1 ETH + 3,000 USDC` (which would now be worth $6,000 + $3,000 = $9,000).

See: `concepts/core/liquidity-providers-and-impermanent-loss.md`

---

### IPC (Inter-Process Communication)

In Aurix's context, the bridge between the Rust backend and the React frontend, mediated by Tauri. The frontend invokes typed Rust functions (`#[tauri::command]`) and receives serialised responses.

Aurix relevance: Aurix has exactly one IPC command (`fetch_market_overview`) that returns a `MarketOverview` to the frontend each tick. The serialisation contract uses Serde with `#[serde(rename_all = "camelCase")]`.

See: `project/architecture/cross-runtime-contract.md`

---

### LP (Liquidity Provider)

Anyone who deposits two tokens into an AMM pool to earn fees. In return for providing capital, they receive a share of every swap fee proportional to their share of the pool's liquidity.

Plain language: an LP is a market maker on autopilot. They deposit tokens once; the pool's math handles all subsequent pricing automatically; they earn fees from every trade through the pool.

See: `concepts/core/liquidity-providers-and-impermanent-loss.md`

---

### Mempool

The queue of transactions that have been broadcast to the Ethereum network but not yet included in a block. Each node maintains its own (slightly different) view of the mempool. Block builders pick from the mempool when constructing the next block.

Plain language: the mempool is the waiting room. Your transaction sits there, visible to any node, for up to ~12 seconds before getting picked up into a block (or remaining pending).

See: `concepts/domain-patterns/the-mempool-public-vs-private.md`

---

### MEV (Maximal Extractable Value)

The maximum profit a block builder can extract by choosing optimal transaction ordering, inclusion, and exclusion. Originally "Miner Extractable Value"; renamed after Ethereum's merge to proof-of-stake.

Plain language: the right to choose the order of transactions has financial value. MEV measures that value. Sandwich attacks are the canonical example.

See: `concepts/domain-patterns/mev-and-transaction-ordering.md`

---

### `MarketOverview`

The Rust struct (in `src-tauri/src/market/types.rs`) representing one sampling tick of Aurix's market state: chain label, pair label, fetched-at timestamp, gas price in gwei, and a vector of `PriceSnapshot` entries (one per venue). Serialised as camelCase JSON when crossing the IPC boundary.

See: `project/architecture/cross-runtime-contract.md`

---

### Price Impact

The amount a swap moves the pool's price as a result of the trade itself. A consequence of the AMM math: any change in the reserve ratio changes the implied price.

Plain language: when you swap 10 WETH into a pool, the pool now has more WETH and less USDC, so each WETH is now worth less USDC. The pre-swap and post-swap prices differ by your "price impact."

Related: slippage. Price impact is the cause; slippage is what the trader experiences as a worse-than-marquee price.

See: `concepts/core/traders-and-slippage.md`

---

### `PriceSnapshot`

The Rust struct (in `src-tauri/src/market/types.rs`) representing one venue's state at one moment: chain, dex_name, pair_label, price_usd (f64), pool_address, fee_tier_bps, price_source_label, fetched_at_unix_ms.

See: `project/architecture/cross-runtime-contract.md`

---

### Reciprocal Rank Fusion (RRF)

A method for combining rankings from multiple ranking systems by summing the reciprocals of each item's rank in each system (typically with a small constant `k` added to the denominator for stability).

Aurix relevance: not used in Aurix itself, but used in the Image Browser project (Caner's other Tauri+React project) and worth knowing as an example of a multi-source ranking technique that could apply to multi-venue arbitrage scoring.

See: `materials/quant-finance-resources.md`

---

### Reserves (V2)

The two token amounts a Uniswap V2 pool currently holds. Read via `getReserves()` which returns `(reserve0, reserve1, blockTimestampLast)`. Token order (which is `token0` vs `token1`) is determined at pool creation by lexicographic comparison of the token addresses.

Aurix relevance: `dex/uniswap_v2.rs` reads `getReserves()` and `token0()`, then derives price as `reserve0/reserve1 × 10^12` (or the inverse, depending on which token is USDC). The `× 10^12` adjusts for the decimal difference: USDC has 6 decimals, WETH has 18, so the raw ratio needs scaling by `10^(18-6)`.

See: `project/systems/insight-engine-anatomy.md`

---

### Sandwich Attack

A specific MEV strategy: a bot observes a victim's pending swap in the mempool, front-runs it with a swap that moves the price unfavourably for the victim, lets the victim's swap execute at the worse price, then back-runs with an opposite swap to capture the victim's price impact as profit.

Plain language: front-run pushes the price up, victim buys at inflated price, back-run sells at the inflated price the bot's own front-run created. Net result: victim pays more, bot pockets the difference.

See: `concepts/domain-patterns/mev-and-transaction-ordering.md`

---

### Serde

The Rust serialisation/deserialisation framework. Aurix uses Serde with the `rename_all = "camelCase"` attribute to convert Rust's snake_case field names to JavaScript-friendly camelCase when serialising IPC payloads.

Aurix relevance: the entire Rust-to-TypeScript wire format is Serde-controlled. A field rename in `market/types.rs` without a matching change in `src/features/arbitrage/types.ts` compiles cleanly on both sides and fails silently at runtime — the only enforcement is convention.

See: `project/architecture/cross-runtime-contract.md`

---

### Sharpe Ratio

A risk-adjusted return metric: `(strategy_return - risk_free_rate) / strategy_volatility`. Higher Sharpe = better return per unit of risk taken.

Plain language: Sharpe lets you compare "10% return at 5% volatility" to "20% return at 20% volatility" — the first has Sharpe ~2.0, the second ~1.0, so the first is more efficient even though its raw return is lower.

See: `concepts/advanced/statistical-primitives-for-risk-modelling.md`

---

### Slippage

The difference between the price you expected to get and the price you actually got, due to the swap's own price impact. Larger trades on thinner pools = more slippage.

Plain language: if the marquee price is $3,000 per WETH and you try to buy 10 WETH, the pool's price moves up as you buy, so your average execution price might be $3,030 — that $30/WETH gap is slippage.

See: `concepts/core/traders-and-slippage.md`

---

### `slot0`

The Uniswap V3 pool's primary state-reading function. Returns `(sqrtPriceX96, tick, observationIndex, observationCardinality, observationCardinalityNext, feeProtocol, unlocked)`. Aurix only uses the first 32-byte word (sqrtPriceX96) and discards the rest.

See: `project/systems/insight-engine-anatomy.md`

---

### `sqrtPriceX96`

Uniswap V3's encoded representation of the current pool price. It's the square root of the price (token1/token0), multiplied by 2^96, stored as a 160-bit unsigned integer. The square root and the binary scaling are both deliberate design choices that make tick math more efficient.

Plain language: V3 stores price in a weird format because (a) using sqrt avoids needing to take square roots during swap math, and (b) scaling by 2^96 lets the math use bit shifts instead of divisions. It's an optimisation, not a fundamental invariant.

Aurix relevance: `dex/uniswap_v3.rs` decodes sqrtPriceX96 from `slot0()` and converts it to a USD price via `(2^192 × 10^(decimals_diff)) / sqrtPriceX96^2`.

See: `concepts/advanced/uniswap-v3-tick-mathematics.md`

---

### Stablecoin

A token designed to maintain a $1 peg (or another fiat-pegged value). Stablecoins are the "dollar layer" of crypto — they let you hold dollar-equivalent value on-chain.

Plain language: when you hold "1 USDC" you have a token that's redeemable for $1 from the issuer. Different stablecoins use different mechanisms — USDC and USDT are backed by real bank deposits (centralised), DAI is backed by on-chain collateral (decentralised), and there are various algorithmic designs (most have failed historically).

Aurix relevance: USDC is the stablecoin half of every venue Aurix watches. Pricing WETH against USDC gives the "WETH in dollars" reading.

See: `concepts/foundations/tokens-and-the-defi-money-stack.md`

---

### Tauri

A framework for building cross-platform desktop applications using web technologies for the frontend and Rust for the backend. Smaller binary and lower memory footprint than Electron because Tauri uses the OS's native webview rather than bundling Chromium.

Aurix relevance: Aurix is a Tauri 2 application. The Rust backend and React frontend communicate via Tauri's IPC layer.

See: `project/decisions/tauri-over-electron.md`

---

### Tick (Uniswap V3)

A discrete price level in V3's pricing space. Ticks are evenly spaced in log-price (specifically, each tick is a 1.0001× price change), which means concentrated liquidity ranges can be expressed as `[lower_tick, upper_tick]` with no loss of precision.

Plain language: ticks discretise V3's continuous price into a grid. An LP says "I want to provide liquidity from tick -100 to tick +100" and the contract translates that to a precise price range using the formula `price = 1.0001^tick`.

See: `concepts/advanced/uniswap-v3-tick-mathematics.md`

---

### Tokio

The Rust async runtime used by Aurix's backend. Provides futures executor, async I/O primitives, and concurrency utilities like `tokio::join!`.

Aurix relevance: `commands/market.rs` uses `tokio::join!` to fetch all four venue prices and the gas price concurrently in one call.

See: `project/systems/insight-engine-anatomy.md`

---

### `tokio::join!`

A macro that polls multiple futures concurrently and returns their results as a tuple, completing only when all of them finish. Unlike `try_join!`, it doesn't short-circuit on the first error — every future is driven to completion.

Aurix relevance: every market overview tick uses `tokio::join!` over five futures (4 venue fetches + 1 gas price read). This is fail-fast at the command level (any error rejects the whole command) but concurrent at the network level (all 5 RPC calls run in parallel).

See: `project/systems/insight-engine-anatomy.md`

---

### USDC

USD Coin. An ERC-20 stablecoin issued by Circle, backed 1:1 by USD held in regulated bank accounts. Decimals: 6.

See: `concepts/foundations/tokens-and-the-defi-money-stack.md`

---

### Value-at-Risk (VaR)

A risk metric: the maximum expected loss over a given time period at a given confidence level. "1-day VaR at 95% confidence = $X" means "we expect to lose less than $X on 95% of days."

See: `concepts/advanced/statistical-primitives-for-risk-modelling.md`

---

### Volatility

A measure of how much an asset's price varies over time. Typically computed as the annualised standard deviation of log returns. Higher volatility = more price variability per unit time.

See: `concepts/advanced/statistical-primitives-for-risk-modelling.md`

---

### WebSocket

A persistent bidirectional connection between client and server, supporting push-based event streams. Used in Ethereum tooling for subscribing to chain events (new blocks, pending transactions, log filters) rather than polling.

Aurix relevance: not used in current Aurix (which polls via JSON-RPC). Vector B (MEV detector) would use WebSocket for `eth_subscribe newPendingTransactions`.

See: `concepts/advanced/mempool-mev-detection-mechanics.md`

---

### WETH (Wrapped Ether)

An ERC-20 token contract (WETH9 at `0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2`) that lets you deposit ETH and receive an equivalent amount of WETH 1:1 (and burn WETH back to ETH 1:1). Necessary because ETH itself isn't an ERC-20 and can't be used directly in DEX contracts. Decimals: 18.

Plain language: WETH is just ETH wearing ERC-20 clothes so it works with smart contracts that expect tokens.

See: `concepts/foundations/tokens-and-the-defi-money-stack.md`
