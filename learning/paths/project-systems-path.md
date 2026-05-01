# Project Systems Path

## Who This Path Is For

You want to understand the Aurix codebase as quickly as possible. You're comfortable with the broad strokes of DeFi (you know what a swap is, what an AMM is, what gas is) and want to see how Aurix is built — what each subsystem does, how they connect, and where the seams are.

By the end you should be able to read any file in the codebase and know roughly what role it plays.

## What This Path Assumes

- General DeFi familiarity (or completion of `foundations-path`)
- Comfort reading Rust and TypeScript

## Recommended Sequence

### Stage 1 — The 30,000-foot view

- [ ] `project/architecture/two-runtime-tauri-rust-react.md`
- [ ] `project/architecture/the-1hz-loadsnapshot-tick.md`

What Aurix is structurally — the two-runtime model (Rust backend + React frontend, mediated by Tauri IPC), and what happens every second when the polling loop fires. About 30 minutes.

### Stage 2 — The cross-runtime contract

- [ ] `project/architecture/cross-runtime-contract.md`

The shape of the IPC payloads, the Serde camelCase convention, and why the Rust ↔ TypeScript type mirror is fragile (no automated check). About 15 minutes.

### Stage 3 — The four backend subsystems

- [ ] `project/systems/what-aurix-observes.md` — synthesis: what the backend produces in domain terms
- [ ] `context/systems/runtime-foundation.md` — the shared runtime substrate (config, RPC client, types)
- [ ] `context/systems/arbitrage-market-data.md` — the DEX adapters and the orchestration command
- [ ] `context/systems/arbitrage-analytics.md` — the TypeScript insight engine
- [ ] `context/systems/arbitrage-gui.md` — the React presentation layer

Note: stages 3's deeper system files live in `context/systems/` rather than `learning/project/systems/`. The context files are already the maintained truth — `learning/` cross-links to them rather than duplicating. About 90 minutes total.

### Stage 4 — The insight engine deep-dive

- [ ] `project/systems/insight-engine-anatomy.md`

The 430-line `insights.ts` file is where Aurix's actual analytical interpretation happens. This file walks through the rolling-window structure, the persistence detection, the four severity levels, and how the "Positive setup holding" insight actually gets produced. About 30 minutes.

### Stage 5 — The decisions

- [ ] `project/decisions/read-only-by-design.md`
- [ ] `project/decisions/tauri-over-electron.md`
- [ ] `project/decisions/rust-backend-over-pure-typescript.md`
- [ ] `project/decisions/no-ethers-rs-handcrafted-abi.md`
- [ ] `project/decisions/plain-css-over-libraries.md`

Five decisions that shape Aurix's character. Each is a short file but each has been deliberately reasoned. About 30 minutes total.

### Stage 6 — Where the project is going

- [ ] `project/evolution/five-tab-vision-vs-current-reality.md`
- [ ] `project/evolution/vector-roadmap.md`

The gap between the README's five-tab vision and what's actually built, plus the three vectors authored in `context/plans/` as proposed paths forward. About 20 minutes.

## What You Should Understand By The End

- Aurix's two-runtime architecture and how data flows from Ethereum → Rust backend → React frontend → user
- Every `#[tauri::command]` (there's only one currently: `fetch_market_overview`)
- How each DEX adapter fetches and decodes prices (V3 via `slot0`/`sqrtPriceX96`, V2 via `getReserves`/reserve ratios)
- What the insight engine computes and how its severity levels work
- The four core design decisions (read-only, Tauri, Rust backend, plain CSS) and their reasoning
- The five-tab vision and the three-vector roadmap

## Estimated Time

3-5 hours of focused reading.

## What To Do Next

| Goal | Next path |
|---|---|
| Ship a vector | `vector-prep-path.md` |
| Deepen the math foundation | `domain-theory-path.md` |
| Practice with code | `exercises/EXERCISE_ORDER.md` |
| Prepare for interview | `interview-fluency-path.md` |
