# MEV Resources

Curated resources on Maximal Extractable Value — the field that explains why most arbitrage Aurix observes isn't actionable, why pro traders use private orderflow, and what Vector B (mempool MEV detector) would actually be detecting.

## Why It Matters For This Repo

Vector B's plan is built on understanding the MEV ecosystem. Even if you don't ship Vector B, knowing this material is essential for interpreting Aurix's observations — particularly why "Positive setup holding" insights persist for many seconds without being captured.

## Primary Sources

### Flashbots Documentation

- URL: `https://docs.flashbots.net`
- Format: technical docs site
- What you'll learn: how Flashbots-style private orderflow works, how to submit bundles, builder economics, MEV-Boost protocol details

Start with "What is Flashbots?" then "MEV-Boost Architecture" then the Bundles API reference. The depth of coverage is excellent; spend 1-2 hours here for a working understanding.

### Flashbots Research Papers

- Index: `https://github.com/flashbots/mev-research`
- Format: academic-flavoured papers and technical write-ups
- What you'll learn: the theoretical foundations of MEV, sandwich attack analysis, builder competition models, encrypted mempool proposals

The research repo contains both production-ready ideas and speculative work. "Quantifying MEV" papers are particularly useful for understanding the empirical scale.

### "Flash Boys 2.0" — Daian et al. (2020)

- URL: arXiv 1904.05234
- Length: ~25 pages
- Difficulty: Moderate (academic style but readable)
- The paper that named MEV as a phenomenon, with empirical analysis of the early ecosystem

Worth reading for context — it's the academic origin of the term. The conclusions are now somewhat dated (the ecosystem has evolved significantly since 2019) but the framing is still useful.

## Empirical Tools

### Eigenphi

- URL: `https://eigenphi.io`
- Format: web dashboard
- What you'll learn: real-time MEV statistics, recent sandwich attacks, JIT liquidity events, arbitrage flows

Browse the recent sandwiches section to see exactly what MEV looks like in practice. Each row shows the victim tx, the bot's front-run and back-run, and the extracted value. Excellent for calibrating expectations about scale.

### libMEV

- URL: `https://libmev.com`
- Format: web dashboard with MEV-Boost block analysis
- What you'll learn: per-block MEV, builder market share, validator-side economics

Useful for understanding the "back of the supply chain" — what builders extract and how much flows back to validators.

### MEV-Inspect-rs

- Repo: `https://github.com/flashbots/mev-inspect-rs`
- Language: Rust
- What you'll learn: a reference implementation of MEV classification — the tool Vector B is essentially trying to be a real-time variant of

Read the source code. Even if you don't use it directly, the patterns for detecting sandwiches, JIT liquidity, and arbitrage are highly transferable.

### MEV-Inspect-py

- Repo: `https://github.com/flashbots/mev-inspect-py`
- Language: Python
- Older Python version of the same idea — good for cross-checking

## Books / Long-Form

### "MEV: A Survey" — Heimbach & Wattenhofer (2023)

- URL: arXiv 2301.13779
- Length: ~30 pages
- Difficulty: Moderate
- Comprehensive survey of MEV strategies, mitigations, and ecosystem participants

Best single resource for getting fully up to speed on the field. Newer than "Flash Boys 2.0" and reflects the post-merge MEV-Boost world.

### "The Order Flow Auction Design Space" — Quintus, Robinson, et al.

- URL: Flashbots blog
- Length: ~10 pages
- What you'll learn: the design tensions in private orderflow auctions, including MEV-Share's philosophy

## Domain-Specific

### Sandwich Detection Methodology

- Search: "sandwich attack detection ethereum" papers
- Several academic papers; methodology is converging
- Aurix's Vector B classifier should match standard methodology

### JIT Liquidity Analysis

- Original Flashbots blog post on JIT
- Several follow-up papers on JIT economics
- Vector B's plan covers JIT detection

### Liquidation MEV

- Aave, Compound, MakerDAO documentation on liquidator economics
- Each protocol has its own liquidation function ABI; the Vector B classifier needs all of them

## When To Read What

**For Aurix's current state interpretation**: Flashbots docs (1-2 hours) + Eigenphi browsing (30 min) is sufficient.

**For Vector B preparation**: add MEV-Inspect-rs source review (2-3 hours), the Heimbach survey (2-3 hours), Flashbots research repo (selective deep-dives, 4-8 hours).

**For interview-level fluency**: Flashbots docs + sandwich detection methodology + Eigenphi familiarity is enough to discuss MEV credibly.

## Related Files

- `concepts/domain-patterns/mev-and-transaction-ordering.md` — concept treatment
- `concepts/domain-patterns/the-mempool-public-vs-private.md` — where MEV happens
- `concepts/advanced/mempool-mev-detection-mechanics.md` — Vector B's technical depth
- `project/comparisons/public-vs-private-mempool-flow.md` — comparative analysis
- `context/plans/vector-b-mev-detector.md` — implementation plan
