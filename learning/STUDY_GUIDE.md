# Study Guide

The archive supports several different ways through the material depending on your starting point and your goal. This file is the route selector — read it before committing to a path.

There is no single correct order. The same content gets approached differently if your goal is "understand the domain" vs "understand the codebase" vs "ship Vector A" vs "explain Aurix in an interview." Pick the route that matches your goal; you can always switch later.

## Route Selection

### Route A — Foundations First (Bottom-Up)

For: someone genuinely new to DeFi, Ethereum, AMMs, and crypto markets. You may know software engineering broadly but the domain itself is unfamiliar.

- [ ] `paths/foundations-path.md` — start here, work through it linearly
- [ ] Then `paths/domain-theory-path.md` — go deeper on AMM math and market microstructure
- [ ] Then `paths/project-systems-path.md` — see how Aurix realises the theory
- [ ] Optional: `paths/vector-prep-path.md` if you want to ship one of the three vectors next

Time to reach "I can hold a conversation about Aurix": ~6-10 hours of focused reading.

### Route B — Project First (Top-Down)

For: someone with general crypto/DeFi familiarity who wants to understand the Aurix codebase as quickly as possible. You're comfortable with the broad strokes (you know what a swap is, you know what an AMM is) and want to see how the project is built.

- [ ] `paths/project-systems-path.md` — start here, get the architecture overview
- [ ] Backfill specific concept files when you hit something unfamiliar (use the cross-links inside each system file)
- [ ] `project/decisions/*.md` — read the design rationale for each major choice
- [ ] Optional: `paths/domain-theory-path.md` for any concept you skimmed

Time to working knowledge of the codebase: ~3-5 hours.

### Route C — Domain Theory Deep Dive

For: someone who wants to understand AMMs, MEV, and DeFi market microstructure regardless of Aurix specifically. You're treating Aurix as a vehicle for learning the domain.

- [ ] `paths/domain-theory-path.md` — the main route
- [ ] Cross-reference `materials/amm-foundational-resources.md` for primary sources
- [ ] `concepts/advanced/*.md` for the rigorous treatments
- [ ] Use the glossary aggressively — `GLOSSARY.md`

Time to graduate-level fluency: 20-40 hours including reading the Uniswap V3 whitepaper end-to-end.

### Route D — Vector Preparation

For: someone who has decided to revive Aurix and ship one of the three vectors (A: V3 LP backtester / B: MEV detector / C: ML signal). You're trying to load the right context before starting the work.

- [ ] `paths/vector-prep-path.md` — prerequisites for each vector, branched
- [ ] The corresponding plan file in `context/plans/vector-{a,b,c}-*.md`
- [ ] Domain theory files relevant to your vector (e.g. tick math for Vector A, mempool mechanics for Vector B)

Time to "ready to start coding": 2-4 hours of focused prep per vector.

### Route E — Interview Fluency

For: someone preparing to talk about Aurix in a hiring conversation. You need to be able to explain the design choices, defend the architecture, and discuss extensions without freezing.

- [ ] `paths/interview-fluency-path.md` — curated for interview talking points
- [ ] `project/decisions/*.md` — every "why did you do X" answer is here
- [ ] `project/comparisons/*.md` — the trade-offs you'll be asked about
- [ ] Practice articulating each design decision out loud

Time to comfortable interview discussion: 4-8 hours.

## Suggested Combinations

- [ ] **Foundations + Project** — if you want both to land properly
- [ ] **Project + Vector Prep** — if you're deciding which vector to commit to
- [ ] **Foundations + Interview Fluency** — if you're new to crypto AND need to interview soon
- [ ] **Domain Theory + Vector A Prep** — if you're committing to the V3 LP backtester (the math overlap is large)
- [ ] **Project Systems + Domain Patterns** — if you want to be able to extend Aurix beyond the README's scope

## What To Do If You're Stuck

If a concept isn't landing despite reading the relevant file:

1. Check the **prerequisites** section — there's almost always an upstream concept that needs to land first
2. Look at the **worked examples** — abstract definitions often click only after concrete numbers
3. Check the **glossary** — the term might mean something more specific than you assumed
4. Look at **how it appears in Aurix** — sometimes the project-grounding makes the abstract idea click
5. If still stuck, reach for `materials/*.md` — primary sources go deeper than the archive can

## Assumptions About The Reader

The archive assumes:

- Solid software engineering background (you can read Rust + TypeScript, you understand async, you've worked with web stacks)
- Comfort with mathematical notation when needed (you don't recoil at Σ or √)
- No prior crypto knowledge required (foundations files start from "what is a token")
- Willingness to read whitepapers when the archive points to them
- A motivated learning attitude — the archive teaches thoroughly, but it doesn't drill

Where these assumptions fail you (e.g. the math gets dense), the archive will say so and point to a remediation path.
