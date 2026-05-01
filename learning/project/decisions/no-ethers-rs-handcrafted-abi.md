# Decision: No ethers-rs — Handcrafted ABI

## Decision Summary

Aurix does not use `ethers-rs`, `alloy`, or any other Ethereum-specific library for ABI encoding/decoding. The backend hand-crafts the JSON-RPC requests, encodes calldata as raw byte sequences, and decodes responses by parsing the hex strings directly.

This is a deliberate choice with both engineering and signalling rationale.

## Alternatives Considered

| Alternative | Description | Why Rejected |
|---|---|---|
| **`ethers-rs`** | The dominant Rust Ethereum library; auto-generates type-safe contract bindings | Heavy dependency; abstracts away the wire-format learning; doesn't signal byte-level competence |
| **`alloy`** | Newer, modular replacement for ethers-rs; smaller surface | Same fundamental concern: abstracts away the layer we want to demonstrate competence in |
| **`web3-rs`** | Older library, less maintained | Same concerns plus poor maintenance signal |
| **Hand-crafted ABI** ✓ | Manual byte-level encoding/decoding | Chosen — see below |

## Why The Chosen Path Won

### 1. Resume signal

The Aurix resume bullet explicitly names this choice:

> "The arbitrage scanner uses raw JSON-RPC with hand-crafted ABI encoding (no ethers-rs), decoding Uniswap V3 sqrtPriceX96 and V2 reserve ratios via BigUint across multiple DEXs."

This is a deliberate signal to crypto-aware hiring managers: the candidate didn't reach for the convenience library; they understand the layer below it. Most "DeFi devs" never read the wire format — they call `contract.method().send()` and trust the abstraction. Hand-crafting the calldata signals depth.

### 2. Educational value

Building the ABI layer by hand teaches:

- The ABI specification (function selector = first 4 bytes of `keccak256(signature)`)
- Padding rules (everything is 32-byte aligned)
- Type encoding (uint256 is right-padded zeros; addresses are 20 bytes left-padded with 12 zeros to 32)
- Response decoding (returns are concatenated 32-byte words)

This material is directly transferable to any other low-level Ethereum work — building a custom indexer, writing a Solidity-to-Rust transpiler, debugging cross-contract calls in production.

### 3. Dependency hygiene

`ethers-rs` is a large dependency tree. Aurix doesn't need 90% of it (no transaction signing, no wallet management, no contract deployment). Hand-crafting saves ~50 transitive dependencies.

### 4. Read-only fit

Aurix's read-only design (see `project/decisions/read-only-by-design.md`) means we only need:
- `eth_call` for contract reads
- `eth_gasPrice` for gas
- Maybe `eth_getLogs` later for Vector A's swap event ingestion

That's a tiny RPC surface. Wrapping it in a 100k-line library is overkill.

### 5. Surface area is small

The current Aurix backend uses 4 selectors:

| Selector | Function | File |
|---|---|---|
| `0x3850c7bd` | V3 `slot0()` | `dex/uniswap_v3.rs` |
| `0xe6a43905` | V2 factory `getPair(address,address)` | `dex/uniswap_v2.rs` |
| `0x0902f1ac` | V2 pair `getReserves()` | `dex/uniswap_v2.rs` |
| `0x0dfe1681` | V2 pair `token0()` | `dex/uniswap_v2.rs` |

Four selectors. Hand-crafting them is ~5 lines of code each. The cost is low; the signal is high.

## Trade-Offs Accepted

| What we give up | Why it's acceptable |
|---|---|
| Type-safe contract bindings | Tested by hand; the four selectors haven't changed since Uniswap V2 deployed in 2020 |
| Auto-generated event filters | Not needed for current Aurix; will need this for Vector A's swap event ingestion |
| Convenient gas estimation | We compute gas costs analytically rather than via `estimateGas` |
| Easy contract deployment | Out of scope (read-only) |
| Extensive ABI library covering all standard types | Aurix only uses uint256, address, and tuple — easy to encode by hand |

## Downstream Consequences

- **Adding a new DEX adapter** requires understanding its function signatures and encoding the calldata by hand. ~30 minutes per adapter.
- **Vector A's swap event ingestion** will need event topic encoding — slightly more complex than function calls, but still well within hand-crafting territory
- **Vector B's calldata decoder** will need to handle many more function signatures (V2/V3 routers, Universal Router, 1inch). At that point, an ABI library starts looking more attractive — but for matching against well-known signatures, hand-rolling is still tractable
- **Future contributors** need to understand the ABI spec, which raises the bar for contributors but produces stronger contributors

## When To Revisit

Consider adding `alloy` (the cleaner ethers-rs successor) if:

- Vector B's calldata decoder grows beyond ~10 router types (the maintenance cost of hand-rolling exceeds the signal value)
- The project ever moves toward writing transactions (then we genuinely need a signing-capable library)
- A specific feature requires complex ABI encoding (e.g. nested structs with dynamic-length arrays)

The decision is "no library FOR NOW given Aurix's current scope." It's not "no library ever."

## How To Tell If This Decision Is Being Compromised

If a PR adds `ethers-rs`, `alloy`, `web3-rs`, or similar as a dependency, the original signalling intent is dropped. Discuss before merging — the dependency might be justified, but the resume bullet would need to be rewritten and the educational material would need updating.

## Concrete Example

Here's what hand-crafted ABI looks like in Aurix (from `dex/uniswap_v2.rs`):

```rust
async fn resolve_pair_address(
    rpc_client: &EthereumRpcClient,
    factory_address: &str,
) -> Result<String, UniswapV2Error> {
    let calldata = format!(
        "{selector}{token0}{token1}",
        selector = GET_PAIR_SELECTOR.trim_start_matches("0x"),  // "e6a43905"
        token0 = encode_address(USDC_ADDRESS),                   // 32-byte left-padded address
        token1 = encode_address(WETH_ADDRESS),                   // 32-byte left-padded address
    );
    let response = rpc_client.eth_call(factory_address, &format!("0x{calldata}")).await?;
    let pair_address = decode_address_word(&response)?;
    // ...
}
```

`encode_address` is a one-liner:

```rust
fn encode_address(address: &str) -> String {
    format!("{:0>64}", address.trim_start_matches("0x"))
}
```

Pad the 40-character hex address to 64 characters (32 bytes) with leading zeros. That's it.

Compare to the equivalent ethers-rs code:

```rust
let factory = IUniswapV2Factory::new(factory_address.parse()?, provider);
let pair = factory.get_pair(usdc, weth).call().await?;
```

The ethers-rs version is more concise but hides everything that's interesting. The hand-crafted version is more verbose but the entire wire format is visible in the source.

For Aurix's signalling and educational goals, the more verbose version wins.

## Links

- `project/decisions/read-only-by-design.md` — why we don't need transaction-signing features
- `concepts/foundations/markets-and-prices.md` — covers ABI as a calling convention
- `context/notes/wire-convention.md` — the project's wire-format conventions
- `materials/ethereum-internals-resources.md` — Ethereum ABI spec
