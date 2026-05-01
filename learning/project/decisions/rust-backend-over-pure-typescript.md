# Decision: Rust Backend over Pure TypeScript

## Decision Summary

Aurix's backend is Rust, not Node.js or pure-TypeScript-with-WebSocket. All on-chain interaction, big-integer math, and async orchestration happens in Rust; TypeScript is purely the frontend presentation and analytical-interpretation layer.

## Alternatives Considered

| Alternative | Description | Why Rejected |
|---|---|---|
| **Pure TypeScript via Node.js** | Backend in Node + frontend in browser, communicating via WebSocket or HTTP | Worse big-integer math (BigInt is slower and less ergonomic), Node async is harder to reason about than Rust tokio, doesn't get Tauri's binary-size benefits |
| **Pure TypeScript via Browser** | Everything in the browser, IPC contracts removed | No serious DEX work possible — browsers can't easily handle 256-bit math and CORS would block direct RPC calls |
| **Backend in Go** | Rust-like performance, simpler concurrency | Less ergonomic for big-integer work, weaker async story, doesn't dovetail with Tauri's preferred Rust integration |
| **Backend in Rust** ✓ | Chosen | Below |

## Why The Chosen Path Won

### 1. Big-integer math performance

The single most consequential operation in Aurix's backend is decoding `sqrtPriceX96` from Uniswap V3. This requires 256-bit unsigned integer arithmetic — outside the range of any native Rust integer (`u128` maxes at ~3.4 × 10³⁸).

Options for handling 256-bit math:

| Language | Big-integer library | Performance | Ergonomics |
|---|---|---|---|
| Rust | `num-bigint` | Fast (native) | Good (operator overloading via traits) |
| TypeScript | `BigInt` (built-in) | Moderate (interpreter overhead) | Awkward (must use BigInt() literal everywhere) |
| Go | `math/big` | Fast | Verbose (no operator overloading) |

`num-bigint` lets Aurix write:

```rust
let numerator: BigUint = (BigUint::from(1u8) << 192) * BigUint::from(10u64).pow(12);
let price = numerator / sqrt_price_x96.pow(2);
```

The TypeScript equivalent is more cluttered. The performance difference is small for one decode but compounds across millions of swaps when Vector A's backtester is replaying historical data.

### 2. Async orchestration clarity

Aurix fans out 5 concurrent RPC calls per tick. The Rust implementation:

```rust
let (v3_5bps, v3_30bps, v2, sushi, gas) = tokio::join!(
    fetch_v3_5bps(&rpc),
    fetch_v3_30bps(&rpc),
    fetch_v2(&rpc),
    fetch_sushi(&rpc),
    rpc.gas_price_gwei(),
);
```

Crystal clear: 5 futures, all start in parallel, completion when all done. Errors are propagated via `Result`.

The Node.js equivalent (`Promise.all`) works similarly but has worse error semantics — `Promise.all` rejects on the first error (similar to Rust's `try_join!`); for `tokio::join!` semantics (drive all to completion regardless of errors), you'd use `Promise.allSettled`. The choice between these is annoying to keep straight in JavaScript; in Rust, the macro names are explicit (`tokio::join!` vs `tokio::try_join!`).

### 3. Type system

Rust's type system catches at compile time:
- Field renames in `MarketOverview` (any consumer that uses the old name fails to compile)
- Missing fields in pattern matches
- Unhandled `Result` returns
- Lifetime errors in shared references

TypeScript catches some of this (with strict settings) but its type system has more escape hatches (`any`, `as` casts, runtime `JSON.parse` returning `any`). For backend code that must be correct, Rust's stronger guarantees are worth the steeper learning curve.

### 4. Future-tab fit

Looking ahead at the planned tabs:
- **Tab 2 (LP Backtester)**: needs exact Q64.96 tick math over millions of historical swaps. Rust performance and `num-bigint` are essential.
- **Tab 4 (Gas Predictor)**: ML model inference. Rust has `tract-onnx` and `ort` for ONNX runtime; both production-ready.
- **Tab 5 (Risk Modelling)**: numerical work over time series — covariance matrices, VaR computation. `nalgebra`, `ndarray`, `polars` are mature in Rust.

These tabs would all be harder in Node.js. Choosing Rust upfront saves a future "we should rewrite this in Rust" pain.

### 5. Tauri's preferred shape

Tauri is Rust-first. The IPC machinery, plugin ecosystem, and security model are all designed around Rust backends. Using Tauri with a non-Rust backend (yes, Tauri supports this) loses much of the framework's value.

### 6. Resume signal

The Aurix resume bullet specifically calls out:
- "Hand-crafted ABI encoding (no ethers-rs)" — Rust enables this
- "BigUint decoding of sqrtPriceX96" — Rust language choice surfaces in the bullet directly

A pure-TypeScript Aurix wouldn't carry these signals.

## Trade-Offs Accepted

| What we give up | Why it's acceptable |
|---|---|
| Steeper learning curve for the backend | Already absorbed; pays off across multiple Rust+Tauri projects |
| Two languages to maintain | Manageable for a single-developer project; cross-runtime contract documented |
| Slower iteration vs pure TypeScript (compile times) | Rust compile times are bearable; rust-analyzer makes the editor experience fast |
| No backend ↔ frontend code sharing | The cross-runtime types must be manually mirrored; treated as a known risk (Gap 11) |

## Downstream Consequences

- **Hand-crafted ABI is possible**: Rust gives us the byte-level control to encode calldata directly without an `ethers-rs` dependency
- **Async patterns are explicit**: every async boundary in the backend uses Rust's explicit `await` + `tokio` primitives, easier to debug than Node's implicit event loop
- **Error handling is structured**: `thiserror::Error` enums per module, explicit `Result` returns everywhere — no swallowed promise rejections
- **Adding a backend feature requires Rust competence**: not all hires can extend Aurix; smaller pool but more committed contributors

## When To Revisit

Reconsider this decision if:

- Rust becomes a hiring bottleneck for the project (currently fine for solo development)
- A specific tab requires a JS library with no Rust equivalent (hasn't happened yet)
- Tauri itself migrates away from Rust-first (very unlikely)

## Links

- `project/decisions/tauri-over-electron.md` — the related framework decision
- `project/decisions/no-ethers-rs-handcrafted-abi.md` — a downstream consequence
- `project/architecture/two-runtime-tauri-rust-react.md` — the broader architecture
- `concepts/advanced/uniswap-v3-tick-mathematics.md` — the math that Rust enables
