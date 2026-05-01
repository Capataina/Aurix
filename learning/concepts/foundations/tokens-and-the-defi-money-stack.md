# Tokens and the DeFi Money Stack

## Why This Matters Here

Aurix watches a single trading pair: WETH/USDC. Before you can understand what Aurix is observing, you need to know what those two things actually are, why they exist, and what role they play in the broader Ethereum economy. This file builds the mental model from "what is money on Ethereum" up to "why specifically WETH and USDC."

## Prerequisites

None. This is a foundations file — start from cold.

## The Core Distinction

Ethereum has **one native asset** called **ETH**. Everything else is a smart contract that *behaves like* a currency by maintaining a ledger.

That distinction matters. ETH is to Ethereum what gold is to a gold standard — a physical (well, digital) thing the system itself is built on. ERC-20 tokens are to Ethereum what bank deposits are to a fiat economy — IOUs maintained by some institution (in this case, smart contracts), tracked as `address → balance` mappings.

When someone says "I sent you 5 USDC," what actually happened on-chain is:

1. The sender called the `transfer(recipient, amount)` function on the USDC contract
2. The contract decremented the sender's `balances[sender_address]` by 5,000,000 (USDC has 6 decimals, so "5 USDC" = 5 × 10⁶ in the raw integer representation)
3. The contract incremented `balances[recipient_address]` by the same 5,000,000
4. The contract emitted a `Transfer(from, to, value)` event
5. All of this consumed gas, paid in ETH

There's no physical token. There's no "USDC coin" anywhere. There's a contract, and the contract has a ledger, and the ledger says you have 5 USDC. That's what "having" a token means.

## ERC-20 — The Standard

ERC-20 is the **interface standard** that every fungible token on Ethereum implements. It's a small set of mandatory functions and events:

```
Required functions:
    transfer(to, amount)
    transferFrom(from, to, amount)
    approve(spender, amount)
    balanceOf(account)
    totalSupply()
    allowance(owner, spender)

Required events:
    Transfer(from, to, value)
    Approval(owner, spender, value)
```

Plus a few optional helpers (`name()`, `symbol()`, `decimals()`).

The standard's job is interoperability. Because every wallet, exchange, and DeFi protocol knows how to call these functions, any new token that implements them works everywhere immediately. There's no "USDC integration" needed in MetaMask — MetaMask just calls `balanceOf(your_address)` on the USDC contract.

## Why "Decimals" Matters

ERC-20 tokens don't actually have decimals in the math — every value is an unsigned integer. The `decimals()` function tells the *interface* layer how to display the integer.

| Token | Decimals | "1 unit" actually stored as |
|---|---|---|
| ETH | 18 | 1,000,000,000,000,000,000 wei |
| WETH | 18 | 1,000,000,000,000,000,000 |
| USDC | 6 | 1,000,000 |
| USDT | 6 | 1,000,000 |
| WBTC | 8 | 100,000,000 |

This is why Aurix's V2 price derivation in `dex/uniswap_v2.rs` multiplies by `10^12`:

```rust
Ok((reserve0 / reserve1) * 10_f64.powi(12))
```

The reserve ratio gives you the raw integer ratio of USDC-units to WETH-units. To convert that to "USD per WETH" you have to scale by `10^(WETH_decimals - USDC_decimals) = 10^(18 - 6) = 10^12`. Forget this scaling and your prices will be 12 orders of magnitude off.

## The Two Assets in Aurix's World

### USDC

Issued by **Circle**, a US-regulated fintech. Each USDC in circulation is backed by $1 USD held in regulated bank accounts (Circle publishes monthly attestations confirming this). USDC is the most-trusted stablecoin on Ethereum: regulated issuer, transparent reserve composition, established redemption mechanics.

Why Aurix uses USDC: pricing WETH against USDC gives a clean "WETH in dollars" reading. If Aurix priced against USDT instead, the price would be slightly different (USDT trades at ~$0.998-$1.002 against USDC due to liquidity asymmetries). USDC vs USD is essentially 1:1.

### WETH

The contract WETH9 lives at `0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2`. It does two things:

```solidity
function deposit() public payable {
    balanceOf[msg.sender] += msg.value;  // Receive ETH, mint same amount of WETH
}

function withdraw(uint amount) public {
    balanceOf[msg.sender] -= amount;
    msg.sender.transfer(amount);          // Burn WETH, send ETH back
}
```

That's it. WETH is a 1:1 wrapper around ETH that exists for one specific reason: ETH itself is not an ERC-20 token. ETH predates the ERC-20 standard. Smart contracts written to handle ERC-20 tokens (which is most of DeFi) cannot directly handle ETH because ETH doesn't have a `transfer(to, amount)` function — it's a special case in the EVM, not a contract.

So when you "send ETH to Uniswap to swap for USDC," you actually:

1. Wrap your ETH → get WETH
2. Swap WETH → USDC on Uniswap
3. (Optionally) Unwrap WETH → ETH if you want to hold ETH again

The Uniswap router handles step 1 and step 3 automatically when you specify ETH as the input/output, but the underlying contract operations always involve WETH.

For Aurix's purposes: when you see "1 WETH = $3,047 on Uniswap V3," that price applies equally to ETH because WETH is 1:1 redeemable for ETH at any time. WETH is just ETH wearing ERC-20 clothes.

## The Stablecoin Family

Stablecoins are tokens designed to maintain a $1 peg (or another fiat-pegged value). They're how you "hold dollars" on-chain — there's no native USD on Ethereum, only USD-pegged tokens issued by various entities.

| Stablecoin | Issuer | Backing model | Trustworthiness |
|---|---|---|---|
| **USDC** | Circle (US-regulated) | Real bank-held USD, monthly attestations | High — clear reserves, regulated issuer |
| **USDT (Tether)** | Tether Limited | Mixed (cash, commercial paper, other) | Controversial — historically opaque, less regulated |
| **DAI** | MakerDAO (decentralised) | Crypto collateral (overcollateralised) | Different trust model — algorithmic, transparent, but vulnerable to collateral cascade |
| **BUSD** | Paxos | Real bank-held USD | Discontinued in 2024 after regulatory pressure |
| **FRAX** | Frax Finance | Mixed (collateral + algorithmic) | Algorithmic stablecoins have failed historically (Terra/UST) |

Aurix watches USDC specifically because it's the most-traded, most-trusted, and has the deepest liquidity on Ethereum DEXes. USDT has comparable volume but the trust gap matters.

## Common Misunderstandings

❌ **"WETH is different from ETH."** WETH and ETH are 1:1 redeemable at any time via the WETH9 contract. The price of WETH and ETH is always equal. They're functionally the same asset; WETH is just the ERC-20-compatible representation.

❌ **"USDC is real US dollars."** USDC is a token whose value is backed by real US dollars (held by Circle in regulated accounts). It is not literally USD — it's an IOU that you can redeem for USD by going through Circle. In normal market conditions, this distinction doesn't matter; in a Circle-bank-failure scenario, it would.

❌ **"Stablecoins are always exactly $1."** USDC trades between roughly $0.999 and $1.001 in normal markets. During the Silicon Valley Bank failure (March 2023), USDC briefly traded at $0.88 because Circle had ~$3.3B exposure to SVB. Stablecoins are *designed* to be $1 but can deviate when their backing is in question.

❌ **"All ERC-20s are equally trustworthy."** Anyone can deploy an ERC-20. The contract code can do whatever its author wrote — including transferring tokens at the author's discretion, blocking certain addresses, or freezing transfers. Trust in a token is trust in (a) the issuer's intentions, (b) the contract's correctness, and (c) the backing model. USDC has high trust on all three; a random meme coin has low trust on all three.

## How This Appears in Aurix

Aurix's `dex/uniswap_v3.rs` hard-codes the assumption that token0 of the WETH/USDC pool is USDC (decimals: 6) and token1 is WETH (decimals: 18):

```rust
const TOKEN0_DECIMALS: u32 = 6;
const TOKEN1_DECIMALS: u32 = 18;
```

The price derivation then computes:

```rust
let numerator: BigUint = (BigUint::from(1u8) << 192) * BigUint::from(10u64).pow(TOKEN1_DECIMALS - TOKEN0_DECIMALS);
let denominator: BigUint = sqrt_price_x96.pow(2u32);
```

The `(TOKEN1_DECIMALS - TOKEN0_DECIMALS)` term is the `10^12` scaling that bridges the decimal asymmetry. This is the load-bearing reason Gap 3 (hard-coded WETH/USDC) exists in the codebase: the moment you support a different pair (say USDC/WBTC, with decimals 6 and 8), you need to make these constants pair-dependent rather than hard-coded.

V2's `dex/uniswap_v2.rs` does it differently — it reads `token0()` from the pair contract at runtime and conditionally inverts:

```rust
let token0_is_usdc = token0_address.eq_ignore_ascii_case(USDC_ADDRESS);

if token0_is_usdc {
    Ok((reserve0 / reserve1) * 10_f64.powi(12))
} else {
    Ok((reserve1 / reserve0) * 10_f64.powi(12))
}
```

The `10^12` is still there, just discoverable via the conditional rather than hard-coded.

## Related Files

- `concepts/foundations/markets-and-prices.md` — what a market is, fundamentally
- `concepts/foundations/exchanges-orderbook-vs-amm.md` — the two paradigms
- `concepts/core/amm-mechanics-v2-and-v3.md` — what these tokens do inside an AMM
- `GLOSSARY.md` — entries for `ERC-20`, `WETH`, `USDC`, `Stablecoin`
- `materials/ethereum-internals-resources.md` — for going deeper on Ethereum's account model
