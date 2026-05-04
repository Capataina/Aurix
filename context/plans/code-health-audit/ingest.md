# Ingest Pipeline — Code Health Findings

**Systems covered:** `src-tauri/src/ingest/{alchemy,subgraph,decoder,pipeline,source,mock,mod,error}.rs` (~1,800 lines, 8 files)
**Finding count:** 2 (1 medium, 1 modularisation verdict)

The ingest layer translates Ethereum archive data (live RPC via Alchemy, hosted subgraph, or synthetic mock) into rows in the persistence layer. The composite hotspot score puts `ingest/mod.rs` at 0.95 (highest in the repo) and `ingest/decoder.rs` at top-decile by line count (515 lines). The decoder is well-tested (~10 unit tests covering each event shape with positive/negative ticks and malformed-log cases), but the orchestration in `mod.rs` has surface-level perf wins worth flagging.

## Performance Improvement

### Avoid per-event byte-by-byte allocation in `parse_int24_word` for V3 swap streams

- [x] In `src-tauri/src/ingest/decoder.rs:91-109`, `parse_int24_word` re-runs `hex::decode(word)` and allocates a fresh `Vec<u8>` per call. For a typical Swap event the function is called once (the trailing tick word). For a backfill of 100k swaps that's 100k transient `Vec<u8>` allocations of 32 bytes each. The arithmetic only needs the last 4 bytes of the hex string — pre-slice and parse without going through `hex::decode`'s general-purpose path. *(implemented 2026-05-04 in commit b2e6863 — `u32::from_str_radix` on trailing 6 hex chars)*

**Category:** Performance Improvement
**Severity:** Medium
**Effort:** Small
**Behavioural Impact:** None (verified — same input bytes, same int24 output, just a tighter parse path)

**Location:**
- `src-tauri/src/ingest/decoder.rs:91-109` — `parse_int24_word`
- `src-tauri/src/ingest/decoder.rs:135` — call site in `decode_swap`

**Current State:**
```rust
fn parse_int24_word(word: &str) -> Result<i32, IngestError> {
    let bytes = hex::decode(word)?;
    if bytes.len() != 32 {
        return Err(IngestError::MalformedLog(format!(
            "int24 word expects 32 bytes, got {}", bytes.len()
        )));
    }
    let negative = bytes[29] & 0x80 != 0;
    let raw = ((bytes[29] as u32) << 16) | ((bytes[30] as u32) << 8) | (bytes[31] as u32);
    if negative {
        Ok((raw | 0xFF00_0000) as i32)
    } else {
        Ok(raw as i32)
    }
}
```

`hex::decode(word)` allocates a `Vec<u8>` of 32 bytes. The function then reads only `bytes[29]`, `bytes[30]`, `bytes[31]` — three bytes out of the 32. The other 29 bytes are untouched.

Over a backfill of 100k swap events, that's 100k × 32 bytes = 3.2 MB of allocator churn purely on the decoder's tick path, and the same on the `parse_uint128_word` path (line 78-87, slightly different — it consumes all 32 bytes of `to_u64_digits()`'s output, so less wasteful).

**Proposed Change:**
Inline the hex-to-int24 conversion to avoid the full-word `hex::decode`:

```rust
fn parse_int24_word(word: &str) -> Result<i32, IngestError> {
    if word.len() != 64 {
        return Err(IngestError::MalformedLog(format!(
            "int24 word expects 64 hex chars, got {}", word.len()
        )));
    }
    // Last 6 hex chars = 3 bytes = the int24 value.
    let raw = u32::from_str_radix(&word[58..64], 16)
        .map_err(|e| IngestError::MalformedLog(e.to_string()))?;
    // Sign bit lives in the high bit of the first of the three bytes.
    let negative = raw & 0x0080_0000 != 0;
    if negative {
        Ok((raw | 0xFF00_0000) as i32)
    } else {
        Ok(raw as i32)
    }
}
```

Two `u32::from_str_radix` calls' worth of work, no `Vec<u8>` allocation. The semantics are identical — same sign-extension rule, same out-of-band length check.

**Justification:**
Direct analytical evidence — the function reads only 3 bytes of the 32-byte word, so allocating all 32 is 90% wasted work. `u32::from_str_radix` on a 6-char hex slice is the canonical zero-allocation parse for fits-in-u32 hex.

The existing tests (`parse_int24_zero_and_positive`, `parse_int24_negative_uses_sign_extension` — `decoder.rs:319-341`) provide bit-for-bit pin against any drift.

**Expected Benefit:**
~32-byte `Vec<u8>` allocation removed per int24 word parse — 100k allocations / ~3.2 MB allocator churn saved on a 100k-swap backfill. The win compounds across `parse_int24_word` calls in `decode_mint`, `decode_burn`, `decode_collect` (each calls it for tick_lower + tick_upper from the indexed topic words).

**Impact Assessment:**
Zero functional change. Both implementations produce the same `i32` for every valid 32-byte hex input, and both reject lengths != 64 hex chars / != 32 bytes. Verified by inspection: the bit ops in both forms are identical (sign-extend from bit 23 to bits 24-31), only the parse path differs.

---

## Modularisation

### Verdict for `src-tauri/src/ingest/subgraph.rs` (549 lines, top-decile candidate)

**Verdict:** `split-recommended`

**Justification:** The file mixes (a) the Uniswap V3 subgraph endpoint URL routing (per chain × per protocol — Uniswap/Sushi/Pancake), (b) the GraphQL query strings + result-shape types, (c) the `UniswapV3SubgraphSource` impl that maps subgraph swaps to `EthLog` per the `ArchiveSource` trait. Each of (a), (b), (c) has its own change cadence: (a) changes when a new chain or fork is added, (b) changes when the subgraph schema evolves or a new event type is queried, (c) changes when the `ArchiveSource` trait surface evolves.

**Recommended split:**
- `subgraph/urls.rs` — endpoint table per (Chain, Protocol). Currently embedded in the file's mid-section.
- `subgraph/queries.rs` — GraphQL strings + GraphQL response types.
- `subgraph/source.rs` (or stay as `subgraph.rs` / mod.rs) — the `UniswapV3SubgraphSource` impl.

The split serves the ongoing Tier-2 (cross-chain) and Tier-3 (V3 forks) work: adding a new chain becomes "edit `urls.rs`"; adding a new event type becomes "edit `queries.rs`"; refactoring the source becomes "edit `source.rs`". Today all three changes touch one 549-line file.

The audit did not deep-read this file (Pass 2 budget did not extend); the verdict is based on file size + the architecture.md description of subgraph responsibilities + the confirmation that Tier 2-3 added a substantial amount of conditional URL routing in this sprint. **Confidence: Moderate.** A future Pass 2 read of the file should confirm the recommendation before implementation.

---

### Verdict for `src-tauri/src/ingest/decoder.rs` (515 lines, top-decile candidate)

**Verdict:** `leave-as-is`

**Justification:** The 515 lines split as ~250 lines of decoder logic (lines 1-261) + ~250 lines of `#[cfg(test)]` tests (lines 263-516). The decoder code itself owns one cohesive concern: ABI decoding of V3 pool events (Swap, Mint, Burn, Collect) plus the shared word-parsing helpers (`parse_uint256`, `parse_int256`, `parse_int24_word`, `parse_uint160_word`, `parse_uint128_word`, `nth_word`, `topic_at`, `address_from_topic`, `strip_prefix`). Splitting decoder logic per event would force the parse helpers into a fourth file (`decoder/words.rs`), creating four files where the current single file already groups by event-shape via doc comments.

The test surface is large because each event has positive/negative-tick + malformed-input + happy-path cases. The inline test convention used throughout Aurix keeps the tests next to the code they exercise.

---

### Verdict for `src-tauri/src/ingest/mod.rs` (392 lines, top-decile candidate)

**Verdict:** `leave-as-is`

**Justification:** The file is the ingest module's orchestration layer — it owns `Ingester`, `IngestionReport`, the `ArchiveSource` trait definition, and the integration tests (~150 lines of `#[cfg(test)]` tests including the `live_alchemy_*` ignored tests). The `Ingester::backfill` orchestration is the natural home for the chunked-eth_getLogs + idempotent-persistence + checkpoint logic. Pulling the orchestration into its own file would require either (a) duplicating the trait + report types in the new file, or (b) leaving them in `mod.rs` and creating a `pipeline.rs` that depends on `mod.rs` — both add boilerplate without separating concern. The 392 lines are mostly tests (~150) + a 50-line trait + the ~190-line `Ingester` impl; the production surface is below the threshold.

---

## Coverage gaps

### Live-RPC tests are correctly gated `#[ignore]`; coverage gap is *intentional*, not a finding

The 3 `ingest::tests::live_alchemy_*` tests are `#[ignore]` because they need live network access. This is the project's correct convention for tests that depend on external state (per the broader Aurix philosophy of not gating CI on live infrastructure). The tests are valuable — they form the integration baseline against the real Alchemy archive RPC — but they are correctly out-of-band of the default `cargo test` run.

The audit does NOT recommend un-ignoring them. Recommendation: when the implementing engineer is touching ingest code, run `cargo test -- --ignored` after configuring `.env` so the live-RPC tests run; the existing tests + the ignored live-RPC tests together provide a comprehensive baseline.
