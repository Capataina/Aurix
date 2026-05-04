# Potential Issues — Suspicions Without Certain-Bar Evidence

These are observations grounded in concrete code reading that did not meet the certain-bar required for `findings.md` files. Each is suspicious enough to flag but requires the implementing engineer's domain knowledge or out-of-process investigation to resolve. Per the audit's Pass 2.5 protocol — distinct file, distinct bar.

If a potential-issue could be resolved by a test the audit can write, it would have been a certain-finding instead. The items below all fail the resolvability criteria — either because they depend on production data shape (LP-on-mainnet fee distribution under realistic active-liquidity), Tauri runtime behaviour (StrictMode + tokio interaction), or live-data side effects (Alchemy 400 root cause).

---

### 1. Mock active-liquidity calibration may misrepresent fee economics

**Locations to inspect:**
- `src-tauri/src/commands/lp.rs:317-321` — `liq_hex = format!("{:0>64x}", 100_000_000_000_000_000u128)` (the synthetic 1e17 active-liquidity)
- `src-tauri/src/math/fees.rs:73-95` — `fee_share` clamps `position_L ≤ active_L`

**Observation:** The `synthetic_mock` function in `commands/lp.rs:321` uses `1e17` as the active liquidity for synthetic swaps. The accompanying comment claims this puts a typical `$10k` position at "~5.6% of pool" after a previous calibration moved from `1e12` to `1e17`. The previous (1e12) calibration produced fees that "over-attributed by ~5000×" — the comment's words.

**Reasoning:** Active liquidity in real V3 mainnet pools (like the ETH/USDC 5bps pool the synthetic data emulates) is typically `1e15` to `1e18` raw units depending on pool depth and concentration. The `1e17` choice is in the right order of magnitude, but whether it accurately reflects the realised fee share for typical positions on the live mainnet pool is something only an implementing engineer with access to live-pool inspection can verify. The synthetic generator's purpose is the demo dashboard, not the validation harness — but the demo's fee numbers are what the user sees first, and they shape the user's expectation about what the LP backtester is computing.

**Suggested investigation:** During a live-data pickup session (one with `cargo test -- --ignored` access), run the live-Alchemy ingest path on the actual ETH/USDC 5bps pool for ~1000 blocks. Compare the realised `active_liquidity` distribution against the synthetic `1e17` constant. If the mainnet distribution centres around a meaningfully different value (say, `5e17` or `2e16`), update the synthetic constant + add a doc comment explaining the calibration source.

**Why not a certain finding:** The audit cannot verify the calibration without live mainnet data, which the test environment cannot reproduce. The observation is grounded in concrete code reading (the comment confesses a previous mis-calibration; the new value is asserted but not tested) but requires out-of-process verification. This is the no-test-physically-possible deferral applied narrowly.

---

### 2. Alchemy 400 carry-forward — possible truncated key in `.env`

**Locations to inspect:**
- `src-tauri/src/config/mod.rs` — env-resolution path for `ALCHEMY_API_KEY` / `MAINNET_RPC_URL`
- `.env` (the user's actual file)
- The 2026-05-04 commit body of `391eadd` which flagged this specifically: "Alchemy 400 (key URL looks short — possibly truncated in .env)"

**Observation:** The 2026-05-04 session-wrap commit explicitly noted that the Alchemy live ingest path was returning HTTP 400, and conjectured the key URL is truncated in the user's `.env`. The audit cannot inspect `.env` (project preference + secrets-handling boundary). A 400 response from Alchemy on `eth_getLogs` is typically either malformed JSON-RPC body, malformed URL (truncated key would fit), or hitting a tier-specific range-cap.

**Reasoning:** A truncated key produces a valid-looking URL with a too-short final segment that Alchemy's gateway rejects with a 400 (path-not-found-style) instead of 401 (unauthorised). If the key was wholly missing, the `from_environment` constructor would have errored out in a different way. The "looks short" signal in the commit body suggests the user already noticed; the diagnosis is unresolved at audit time.

**Suggested investigation:** During the next session (per the orient run's #2 today's-likely-focus item), the implementing engineer should compare the `.env` value against an expected Alchemy key length (typically ~32 chars for the URL path-segment after `/v2/`), and run a single `eth_blockNumber` request via curl to confirm the URL responds. If the key is truncated, fix `.env` and the 4-tier extension's Tier 2 (cross-chain) + Tier 4 (non-USD via DefiLlama with chain-specific routing) become testable.

**Why not a certain finding:** The audit cannot read `.env` (project preferences + the orient skill's earlier secrets-respect convention) and cannot run live HTTP from this session. The carry-forward in the commit body is the strongest evidence; resolving it requires either (a) the implementing engineer running curl against the URL, or (b) the audit reading `.env` (out of scope). This is the "out-of-process state" no-test deferral — the resolution depends on the user's actual `.env` file shape.

---

### 3. The `prev_sqrt = cur_sqrt` clone at the end of every backtest loop iteration

**Locations to inspect:**
- `src-tauri/src/backtest/engine.rs:148, 278` — `let mut prev_sqrt = entry_sqrt.clone();` + `prev_sqrt = cur_sqrt;`

**Observation:** The LVR computation at lines 189-202 needs the previous swap's `sqrt_price_x96` to compute `delta = cur_f - prev_f`. The current implementation moves `cur_sqrt` (a freshly-parsed BigUint per iteration) into `prev_sqrt` at end of loop — no clone here, but the parse cost on the next iteration means the *next* `cur_sqrt` is paid for fresh, not via reuse.

**Reasoning:** If the parsing finding (backtest.md §"Pre-parse swap rows once instead of per-loop-iteration") is implemented, this loop becomes a walk over already-parsed BigUints — the prev_sqrt carry becomes a borrow rather than a clone, since the parsed vector outlives the loop. Whether that gives a measurable benefit depends on the typical BigUint size for sqrtPriceX96 values (~25 digits, so ~2 u64 words). The audit's intuition is that the win is tiny, but a benchmark would resolve it.

**Suggested investigation:** When the backtest.md pre-parse finding is implemented, write a criterion benchmark for `Engine::simulate` over a 10k-swap synthetic stream, measuring before/after. If the speedup is meaningful (>5%), the audit's analytical claim is corroborated; if it's lost in noise, the per-iteration parse cost was already amortised by something the audit didn't see.

**Why not a certain finding:** The audit's analytical claim (parsing once is faster than parsing N times) is unfalsifiable in principle but not quantifiable without a benchmark, and the audit didn't write a criterion benchmark for this run. A benchmark would be a load-bearing piece of evidence; the audit defers it to the implementing engineer per "no diagnostic test needed when finding confidence is already moderate-to-high without it." This entry exists to flag the dependency between findings — the prev_sqrt cleanup is contingent on the parse-once finding landing first.

---

### 4. `f64` casts in LVR computation when `cur_f`/`prev_f` are very large

**Locations to inspect:**
- `src-tauri/src/backtest/engine.rs:190-201` — LVR computation

**Observation:** The LVR computation casts `cur_sqrt.to_f64().unwrap_or(0.0)` and `prev_sqrt.to_f64().unwrap_or(0.0)`. For sqrtPriceX96 values close to MAX_SQRT_RATIO (which is ~1.5e48), the f64 representation loses ~25-29 bits of precision (f64 has 52 bits of mantissa; the values are ~160 bits). The `delta = cur_f - prev_f` then suffers catastrophic cancellation when the two sqrt values differ by a small amount on the high-magnitude end of the range.

**Reasoning:** For typical mainnet pools (ETH/USDC, WBTC/USDC), sqrtPriceX96 lives in a relatively narrow band — ~1e25 to ~1e35 — well within f64's representable range. The LVR formula `0.5 * delta * delta * L_f / (cur_f * q96)` is dimensionally correct, but for extreme price moves (during a flash event, for example) the f64 precision loss could produce noise-level LVR readings.

**Suggested investigation:** Replace the f64 LVR computation with a BigInt/BigUint version that maintains full precision until the final USD conversion. Or: scale-down sqrtPriceX96 to a more f64-friendly range before subtracting. Or: document the precision floor in the engine module's doc comments so users understand the LVR readings have a noise floor proportional to the sqrtPrice magnitude. The audit's preference is the documented-precision-floor option — full BigInt LVR is a meaningful refactor with its own correctness risks.

**Why not a certain finding:** Quantifying the precision loss in practice requires running the engine on real mainnet data and measuring the LVR noise floor. The audit's claim is analytical (f64 has 52 mantissa bits, sqrtPriceX96 has up to 160 bits, therefore precision is lost) but the *magnitude* of impact in realistic scenarios is unknown without measurement. This is suspicious enough to flag for the implementing engineer's attention; resolving it requires either a measurement campaign or a documented design decision to accept the precision floor.

---

### 5. Storage `wal_checkpoint(TRUNCATE)` is callable but not scheduled

**Locations to inspect:**
- `src-tauri/src/storage/mod.rs:113-127` — the `checkpoint` method
- `src-tauri/src/lib.rs:42-75` — the Tauri `run` body, where periodic checkpointing would be scheduled

**Observation:** `Storage::checkpoint` runs `PRAGMA wal_checkpoint(TRUNCATE);` on the writer. The doc comment says "Should be called periodically (e.g. every N writes or every M seconds) to bound WAL growth." But the audit's reading of `lib.rs` and the IPC commands shows no caller currently invokes it. The WAL file grows unbounded under sustained ingest workloads.

**Reasoning:** SQLite's WAL grows during writes and truncates only on explicit `wal_checkpoint(TRUNCATE)` or when the WAL hits a size threshold (default 1000 pages). For long-lived processes doing continuous ingest (a backtest of a long block range, or a live-watching dashboard mode that accumulates over hours), the WAL can grow into hundreds of MB without a checkpoint trigger. The reference doc the engineer cited (`sqlite-rust-production-patterns.md`) presumably covers the recommended cadence — the audit didn't read the reference but the doc-comment hint suggests it does.

**Suggested investigation:** Add a periodic checkpoint task to the Tauri runtime startup — e.g. a tokio interval timer that calls `storage.checkpoint().await` every N seconds (say 60). The trigger condition can be either time-based, write-count-based (instrument the writer to count completed transactions), or both. The right cadence depends on production WAL growth rate, which the implementing engineer can measure.

**Why not a certain finding:** The audit cannot quantify "how big does the WAL get?" without running the backend long enough to see growth. The risk is real but the magnitude depends on usage pattern. The implementing engineer should either (a) add the periodic task as a defence-in-depth measure, or (b) document the expected WAL growth bounds and confirm SQLite's default 1000-page auto-truncate threshold is adequate. Either choice closes the issue.
