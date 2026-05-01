# Learning Map

`learning/` is the project's educational archive. Its job is to teach Aurix and the surrounding DeFi domain thoroughly enough that a motivated engineer can own the project, defend its decisions in interviews, and reason about extensions without having to reverse-engineer the same concepts repeatedly from primary sources.

This is not a sidecar README, a cheat sheet, or a doc summary. The archive is allowed to be large, verbose, and detailed because the underlying material is genuinely deep — automated market makers, cross-runtime systems, MEV economics, liquidity provision mathematics, and machine learning on financial time series all earn their own first-principles treatment here.

## How `learning/` Differs From `context/`

| Folder | Purpose | Audience | Style |
|---|---|---|---|
| `context/` | Implementation memory: what is currently in the code, how the systems are shaped, what decisions were made, what constraints apply | Future Aurix sessions, principal-engineering collaborator | Implementation-facing, terse, evidence-anchored, pinned to file paths and line numbers |
| `learning/` | Teaching archive: what the project IS, what the domain IS, why everything works the way it does, how to think about the field | The engineer learning DeFi, AMMs, MEV, statistics for risk modelling | First-principles, narrative, worked examples, comparisons, exercises |

`context/` answers *"what does the code do right now and why is it shaped this way?"*  
`learning/` answers *"how does any of this work, what are the underlying ideas, and how would I teach someone to build it from scratch?"*

The two folders cross-reference each other but should not duplicate. When `context/systems/arbitrage-analytics.md` says "the analytics layer uses a baseline window of 20 samples," `learning/` does not repeat that fact — `learning/` instead teaches *what a rolling baseline window IS, why we use one, and how it connects to time-series statistics generally*.

## Archive Structure

```text
learning/
├── LEARNING_MAP.md            (this file — usage guide and structural overview)
├── GLOSSARY.md                (alphabetical, comprehensive, cross-linked to deeper files)
├── STUDY_GUIDE.md             (route selector — which path to take given your goal)
│
├── paths/                     (focused learning paths through the archive)
│   ├── PATH_INDEX.md
│   ├── foundations-path.md
│   ├── domain-theory-path.md
│   ├── project-systems-path.md
│   ├── vector-prep-path.md
│   └── interview-fluency-path.md
│
├── concepts/                  (the theory layer — domain knowledge that underpins Aurix)
│   ├── foundations/           (entry-level: what tokens, markets, exchanges are)
│   ├── core/                  (AMM mechanics, LP math, slippage, arbitrage)
│   ├── domain-patterns/       (gas, MEV, mempool — the broader Ethereum economy)
│   └── advanced/              (V3 tick math, MEV detection, ML, statistics)
│
├── project/                   (Aurix-specific teaching — how the codebase realises the theory)
│   ├── architecture/          (the two-runtime model, IPC, data flow)
│   ├── systems/               (deep-dives on backend ingestion, frontend insights)
│   ├── decisions/             (the four core principles + technology choices, with reasoning)
│   ├── comparisons/           (V2 vs V3, public vs private mempool)
│   └── evolution/             (the five-tab vision, the three-vector roadmap, status)
│
├── exercises/                 (practice — code drills + design exercises)
│   ├── EXERCISE_GUIDE.md
│   ├── EXERCISE_ORDER.md
│   ├── foundations/           (paper exercises — work the AMM math by hand)
│   ├── core/                  (Rust drills — decode sqrtPriceX96, simulate V2 swap)
│   ├── project/               (extend Aurix — add an insight rule, design schema)
│   └── solutions/             (working implementations + index)
│
├── materials/                 (curated external resources, organised by topic)
│   ├── amm-foundational-resources.md
│   ├── mev-resources.md
│   ├── quant-finance-resources.md
│   ├── ethereum-internals-resources.md
│   └── ml-for-finance-resources.md
│
└── references/                (quick lookup — notation guides, status conventions)
    ├── notation-guide.md
    └── status-conventions.md
```

## Where To Start

Pick by goal, not by alphabetical order:

| You want to... | Start here |
|---|---|
| Get oriented before reading anything else | `STUDY_GUIDE.md` (the route selector) |
| Understand the domain from the bottom up | `paths/foundations-path.md` |
| Understand AMM mathematics deeply | `paths/domain-theory-path.md` |
| Understand the Aurix codebase | `paths/project-systems-path.md` |
| Prepare to ship one of the three vectors | `paths/vector-prep-path.md` |
| Develop talking points for a hiring interview | `paths/interview-fluency-path.md` |
| Look up a specific term | `GLOSSARY.md` |
| Practice with code | `exercises/EXERCISE_ORDER.md` |

## Status Labels

The archive contains material at different stages of project reality. Where ambiguity could mislead the learner, files include short status sections using these labels:

- **Current in the project runtime** — what the code does today
- **Foundational domain knowledge** — true regardless of project state (e.g. AMM math, statistics)
- **Planned project direction** — README-described or vector-plan-described, not yet implemented
- **Superseded** — older approach the project moved away from, retained because the contrast teaches something

When a file teaches a topic that's not yet implemented (e.g. ML signal layer in Vector C), it labels itself clearly so the learner doesn't conflate aspiration with reality.

## Progress Tracking

Files that contain learner checkboxes (`- [ ]`):

- `STUDY_GUIDE.md` — high-level route selection
- `paths/*.md` — every path file's recommended sequence
- `exercises/EXERCISE_ORDER.md` — the canonical exercise progression

These checkboxes are part of the learning system's state. They are preserved across upkeep passes whenever the underlying file structure permits a clean semantic mapping. If a topic moves or is renamed, the upkeep pass is required to record what happened to the corresponding checkbox.

Concept files, system files, decision files, glossary entries, and most exercise body content do NOT contain checkboxes — they're reference material, not progression markers.

## How To Read A Concept File

Most concept files follow a shape designed to teach from multiple angles:

1. **Why it matters here** — the project-specific motivation
2. **Prerequisites** — what to read first if you're cold on the topic
3. **Notation** (if applicable) — symbols and what they mean
4. **Core idea** — first-principles explanation
5. **Build-up** — step-by-step development of the concept
6. **Worked examples** — concrete numbers worked through
7. **How it appears in Aurix** — link back to specific code/systems
8. **Common misunderstandings** — what NOT to think
9. **Related files** — where to go next

You don't have to read in order, but if a section confuses you, scrolling back to the build-up almost always helps.

## Archive Scope

The archive is intentionally larger than "what's in the code." Per the project's CLAUDE.md and the upkeep-learning skill philosophy, it covers:

- Current implementation reality (Tab 1 partial)
- Project mission and roadmap (the five tabs the README describes)
- The three vectors authored in `context/plans/` (V3 backtester, MEV detector, ML signal)
- Foundational domain theory needed to understand any of the above
- Comparisons, alternatives, and the trade-offs that shaped Aurix's design

This means you'll find files teaching tick mathematics even though Tab 2 isn't built, and files on MEV even though Aurix doesn't watch the mempool yet. That's intentional — the archive teaches the project Aurix is trying to become, not only the project that exists at this exact commit.
