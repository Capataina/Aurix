# Gas and Execution Costs

## Why This Matters Here

Gas is Ethereum's metering system and the dominant cost layer for almost every DeFi operation. Aurix's `GAS_UNITS_ESTIMATE = 220_000` and the live `gas_price_gwei` reading are central to the gas-adjusted profitability calculations. Understanding what gas is, how it's denominated, and why it's the reason most retail arbitrage doesn't work is essential to interpreting Aurix's dashboard correctly.

## Prerequisites

- `concepts/foundations/tokens-and-the-defi-money-stack.md` (you should know what ETH is)

## What Gas Is

Every operation the Ethereum Virtual Machine (EVM) performs has a **gas cost**. This is a unit of computational and storage work. An `ADD` opcode costs 3 gas. A `MUL` costs 5. Reading from storage costs 2,100 gas (or 100 if recently accessed in the same transaction). Writing to storage costs 20,000 gas (for a new slot).

The gas system serves two purposes:

1. **Pricing computation.** Without metering, anyone could submit infinite-loop transactions and grind the network to a halt. Gas means every operation has a cost paid by the transaction sender.

2. **Pricing scarcity of block space.** Each block has a maximum gas limit (currently ~30 million gas). When demand exceeds supply, the gas price (per unit) rises. This is the mechanism by which Ethereum allocates scarce block space.

## Gas Price Denomination

Gas is paid in ETH, but the per-unit price is denominated in **gwei** (gigawei) for convenience.

- 1 ETH = 10⁹ gwei = 10¹⁸ wei
- "20 gwei gas price" means 0.00000002 ETH per unit of gas

Why use gwei? Because expressing gas prices in ETH would give numbers like "0.000000020" — hard to read. Expressing in wei would give "20,000,000,000 wei" — also annoying. Gwei is the Goldilocks zone: gas prices are typically 5-200 gwei in normal conditions, easy to compare at a glance.

The total fee a transaction pays:

> **fee_in_wei = gas_units_used × gas_price_in_wei**

Or, more humanly:

> **fee_in_eth = gas_units_used × (gas_price_in_gwei × 10⁻⁹)**

Then convert to USD by multiplying by the current ETH price.

## Gas Costs of Common Operations

Approximate gas units for common DeFi operations:

| Operation | Gas units | Notes |
|---|---|---|
| Simple ETH transfer | 21,000 | Minimum transaction cost |
| ERC-20 token transfer | 35,000–55,000 | Depends on whether recipient has prior balance (cold vs warm SSTORE) |
| Approve token spending | 45,000 | One-time cost per spender per token |
| Uniswap V2 swap | 90,000–150,000 | Single hop |
| Uniswap V3 swap (same tier) | 120,000–200,000 | Higher than V2 due to tick crossings |
| Uniswap V3 swap (cross-tier route) | 180,000–300,000 | More complex routing |
| Round-trip arbitrage (V3 → V2) | 250,000–400,000 | Two swaps in one tx |
| Mint a Uniswap V3 LP position | 300,000–500,000 | One-time cost per position |
| Burn a Uniswap V3 LP position | 200,000–300,000 | One-time cost when withdrawing |
| Aave deposit/withdraw | 200,000–350,000 | Depends on collateral involvement |
| Aave liquidation | 250,000–500,000 | Cross-asset operation |

Aurix's `GAS_UNITS_ESTIMATE = 220_000` is in the ballpark for "a single swap on a typical pool." It's not pool-specific — Vector A could improve this by computing per-venue gas estimates from historical swap transactions.

## What Determines the Gas Price

The gas price you pay is determined by two layers (post-EIP-1559):

1. **Base fee** — set algorithmically per block based on the previous block's fullness. If the previous block was >50% full, base fee rises 12.5%; if <50% full, drops 12.5%. The base fee is BURNED (removed from circulation) — it's not paid to validators.

2. **Priority fee (tip)** — what you pay validators on top of the base fee to incentivise them to include your transaction. Typically 1-3 gwei in normal conditions, can spike during MEV competition.

Total gas price = base fee + priority fee. Both are denominated in gwei.

Aurix's backend reads `eth_gasPrice` which returns the current "suggested" gas price (a function of recent base fee + typical priority fee). It's accurate enough for the dashboard's purpose but not precise enough for execution decisions.

## Gas Cost Examples

To make this concrete — here's what swap costs look like at different gas regimes (assuming ETH = $3,000, swap = 200,000 gas units):

| Gas price (gwei) | Cost in ETH | Cost in USD | When you see it |
|---|---|---|---|
| 5 | 0.001 | $3 | Late night UTC, no NFT mints, post-merge calm |
| 15 | 0.003 | $9 | Typical "normal" daytime conditions |
| 30 | 0.006 | $18 | Moderately congested, busy DeFi day |
| 50 | 0.010 | $30 | Active market period, common during volatility |
| 100 | 0.020 | $60 | NFT mint frenzy, major launch, high demand |
| 200 | 0.040 | $120 | Black swan, major liquidation cascade |
| 500 | 0.100 | $300 | Network basically unusable for retail |

For retail arbitrage to work, you typically need to capture gross spreads of $30+ at 30 gwei or $60+ at 100 gwei (because you pay round-trip gas). Below that, arbitrage is unprofitable.

## Why This is the Killer for Retail Arbitrage

Aurix shows lots of "+$3 to +$8 spreads holding for minutes." Almost none of these are actionable because:

- A round-trip arbitrage (buy on A, sell on B) costs ~250,000–400,000 gas
- At 20 gwei (a fairly low gas environment): ~$15–$24 in real money
- A $6 spread minus $20 in gas = -$14 net loss

Pro arbitrageurs solve this two ways:
1. **Atomic batching**: combine many small arbs into a single transaction, amortising the fixed gas overhead
2. **Private orderflow**: bypass public mempool gas competition entirely via Flashbots

Neither is available to retail without specialised infrastructure. So retail "watching arbitrage" projects (like Aurix) are educational tools — they show you the market microstructure, but they don't generate profitable signals you could act on.

## How This Appears in Aurix

Aurix's gas-adjusted profitability calculation (in `insights.ts`):

```typescript
const gasCostUsd = (gasPriceGwei * GAS_UNITS_ESTIMATE * medianPrice) / 1_000_000_000;
const gasAdjustedUsd = spreadUsd - gasCostUsd;
```

Walking through the units:
- `gasPriceGwei` is gwei (10⁻⁹ ETH per gas unit)
- `GAS_UNITS_ESTIMATE` is gas units (220,000)
- `medianPrice` is USDC per WETH (so it's the WETH/USD price)
- Result of multiplication: gwei × gas × (USD/ETH) — the units don't quite line up because there's an implicit assumption that 1 ETH = 1 WETH (true: WETH is 1:1 redeemable)
- Dividing by 10⁹ converts gwei to ETH: total_gas_in_eth = gasPriceGwei × GAS_UNITS_ESTIMATE / 10⁹
- Multiplying by `medianPrice` (USD/WETH = USD/ETH) gives gas cost in USD

So `gasCostUsd` is "what 220,000 gas units cost in USD at the current gas price and ETH price." Subtract from gross spread to get gas-adjusted profitability.

This calculation is present in three files (Gap 4 — analytical primitive duplication). When you ship Vector A or any fix to this formula, you must update all three together.

## Gas Optimisation Strategies

This isn't directly Aurix-relevant, but it's worth knowing:

- **Batch operations** in a single transaction to amortise fixed costs
- **Use multicall contracts** to combine multiple reads into one transaction
- **Avoid storage writes** when possible (most expensive opcode by far)
- **Use packed structs** to fit multiple values in single storage slots
- **Compute off-chain when possible**, store proof on-chain (the basis of zk-rollups)

These matter when WRITING contracts. For Aurix (which only READS), gas costs of `eth_call` are paid by the RPC provider, not by Aurix — that's why Aurix's polling loop is free even though it makes hundreds of RPC requests per minute.

## Common Misunderstandings

❌ **"Gas price is set by miners/validators."** Post-EIP-1559, base fee is algorithmically determined by block fullness; only the priority fee (tip) is a market between users and validators.

❌ **"More gas = my transaction is faster."** Setting a higher priority fee makes validators more likely to include your tx in the next block, but it doesn't make the EVM execute faster. The "speed" you're buying is inclusion priority, not computational speed.

❌ **"Gas is wasted ETH."** Base fee is BURNED (removed from circulation), so it's net deflationary for ETH supply. Priority fees go to validators. From the network's perspective, gas isn't wasted — it's the mechanism that allocates scarce block space.

❌ **"Aurix's RPC calls cost gas."** No. `eth_call` is a free read operation; gas is only consumed by transactions that get included in blocks. Reading data via RPC is free (rate limits aside). The RPC provider absorbs the cost of running the node.

❌ **"Gas price is the only execution cost."** Gas is one of three: gas, slippage, and competition. Aurix only models gas explicitly. Real execution would need to model slippage (which depends on trade size) and competition (which depends on the opportunity's visibility to other arbitrageurs).

## Related Files

- `concepts/core/arbitrage-and-cross-venue-equilibrium.md` — gas as the cost floor that keeps spreads from closing to zero
- `concepts/domain-patterns/mev-and-transaction-ordering.md` — gas competition as a vector of MEV
- `concepts/core/traders-and-slippage.md` — the other major execution cost
- `materials/ethereum-internals-resources.md` — for going deeper on the EVM and EIP-1559 mechanics
