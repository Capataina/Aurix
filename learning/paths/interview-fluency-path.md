# Interview Fluency Path

## Who This Path Is For

You have a hiring conversation coming up where Aurix is going to come up. You need to be able to:

- Explain what the project is in 30 seconds
- Walk through any design decision and defend it
- Discuss extensions without freezing
- Answer the inevitable "did you actually build this or did you just fork something" question with concrete evidence

By the end of this path you should be able to speak comfortably about Aurix at a level that satisfies a senior engineer at a crypto-quant or trading firm.

## What This Path Assumes

- General DeFi familiarity (or completion of `foundations-path`)
- General awareness of the Aurix codebase (either you wrote it, or you've completed `project-systems-path`)

## Recommended Sequence

### Stage 1 — The 30-second pitch

- [ ] Read `project/systems/what-aurix-observes.md`
- [ ] Practice articulating: "Aurix is a local-first DeFi analytics desktop app — Tauri 2 backend in Rust, React 19 frontend, polling four DEX venues at 1 Hz to surface cross-venue arbitrage opportunities with gas-adjusted profitability estimates. Read-only by design. Tab 1 (arbitrage) is partial; the architecture supports a five-tab vision covering LP backtesting, wallet tracking, gas intelligence, and risk modelling."

If you can't say something close to that in one breath, re-read until you can.

### Stage 2 — Defend every decision

For each of these decisions, you should be able to articulate (a) the choice, (b) the alternative, (c) the trade-off, and (d) why the trade-off was acceptable. Read each file and rehearse out loud.

- [ ] `project/decisions/read-only-by-design.md` — *"Why doesn't Aurix execute trades?"*
- [ ] `project/decisions/tauri-over-electron.md` — *"Why Tauri instead of Electron?"*
- [ ] `project/decisions/rust-backend-over-pure-typescript.md` — *"Why is the backend Rust instead of Node?"*
- [ ] `project/decisions/no-ethers-rs-handcrafted-abi.md` — *"Why did you write your own ABI encoding instead of using a library?"*
- [ ] `project/decisions/plain-css-over-libraries.md` — *"Why no Tailwind or shadcn/ui?"*

For each: name the alternative, the win-condition for the chosen approach, and the cost you accepted. Don't try to defend choices as universally correct — defend them as deliberate trade-offs.

### Stage 3 — Talk about the math

You will be asked about AMM math. The interviewer is checking whether you understand what's actually happening or just wrote scaffold code.

- [ ] `concepts/core/amm-mechanics-v2-and-v3.md` — be able to walk through V2's `x * y = k` with concrete numbers
- [ ] `concepts/advanced/uniswap-v3-tick-mathematics.md` — at minimum, be able to say what `sqrtPriceX96` is and why V3 encodes price that way

Practice answering: *"Walk me through what happens in your code when you decode `sqrtPriceX96`."* The honest answer involves `BigUint::from_bytes_be` of the 32-byte word, then computing `(2^192 × 10^12) / sqrt^2` to get the WETH/USDC price as `f64`. If you can't do this from memory, you don't actually understand the V3 decode and you should re-read.

### Stage 4 — Talk about the failure modes

You will be asked what's wrong with the project. The right answer is to enumerate the gaps honestly — that demonstrates engineering judgement, not weakness.

- [ ] Read `Projects/Aurix/Gaps.md` from the LifeOS vault (via `gh api`) — every known gap is documented
- [ ] Be able to name the top 3 gaps from memory:
  - **Gap 1:** No persistence (blocks 4 of 5 tabs + Tab 1 M1.5 historical features)
  - **Gap 4:** Duplicated analytical primitives — `formatUsd` already drifted
  - **Gap 6:** Zero tests across ~1850 LOC

Saying "the project has no tests and that's a known issue I plan to fix in M2.0" is much stronger than letting an interviewer find this for you.

### Stage 5 — Talk about extensions

You will be asked "how would you extend this?" The answer is the three vectors.

- [ ] `project/evolution/vector-roadmap.md` — overview
- [ ] `context/plans/vector-a-v3-lp-backtester.md` — read in detail
- [ ] `context/plans/vector-b-mev-detector.md` — read in detail
- [ ] `context/plans/vector-c-ml-arbitrage-survival.md` — read in detail

For each vector, be able to answer:
- What's the engineering signal it adds?
- What's the audience that would care?
- What's the prerequisite (e.g. persistence for A and C)?
- What's the rough effort estimate?

### Stage 6 — Talk about MEV (interviewer-favourite question)

If the interviewer is from a crypto-quant or trading firm, MEV will come up. They want to see whether you understand the ecosystem.

- [ ] `concepts/domain-patterns/mev-and-transaction-ordering.md`
- [ ] `concepts/domain-patterns/the-mempool-public-vs-private.md`

You should be able to explain a sandwich attack in your own words, name Flashbots, and articulate why most "free money" arbitrage opportunities Aurix shows are actually unprofitable (gas + slippage + bot competition).

### Stage 7 — Cross-project context

For interviews where the conversation moves to your other projects, be able to anchor each one:

- **NeuroDrive** — biology-inspired RL with custom PPO + sparse graph network (the research-grade work)
- **Nyquestro** — from-scratch matching engine with lock-free order book (the HFT-systems signal)
- **Image Browser** — multi-encoder semantic search with RRF (the applied-research signal)
- **Cernio** — local-first job-search systems engineering with multi-agent grading

The point: Aurix is one of five — and you can articulate which audience each project targets and how they complement each other.

## Common Question Patterns

| Question shape | Where to find your answer |
|---|---|
| "What is Aurix?" | Stage 1 above |
| "Why X over Y?" (any tech choice) | Stage 2: decisions |
| "Walk me through what happens when..." | Stage 3: math + Stage 5: vectors |
| "What's wrong with it?" | Stage 4: failure modes |
| "How would you extend it?" | Stage 5: vectors |
| "What's MEV?" | Stage 6 |
| "Tell me about your other projects" | Stage 7 |
| "Did you really build this or fork something?" | Be ready to walk through `dex/uniswap_v3.rs` line by line; nothing in there is forkable scaffold |

## What You Should NOT Try To Defend

- Aurix as a money-making project (it isn't, by design — read-only)
- Aurix as production-ready (it's a portfolio piece, the gaps are documented)
- The 5-tab vision as currently complete (only Tab 1 is partial)
- Any decision you don't actually understand (admit "I'm not sure" rather than fabricating)

The strongest interview move is honesty about what's done, what's planned, and why each decision was made.

## Estimated Time

4-8 hours of reading + practice. The reading is fast; the rehearsal-out-loud is what takes time.

## What To Do Next

| Goal | Next path |
|---|---|
| Strengthen the project before interviewing | `vector-prep-path.md` (pick A or C) |
| Go deeper on theory if asked technical questions | `domain-theory-path.md` |
| Build muscle memory on the codebase | `exercises/EXERCISE_ORDER.md` |
