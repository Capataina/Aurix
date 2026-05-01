# Path Index

The archive supports five focused paths, each oriented around a different goal. Pick by what you're trying to accomplish, not by alphabetical order. See `STUDY_GUIDE.md` for the high-level route selector.

## Available Paths

| Path | For learners who | Length |
|---|---|---|
| `foundations-path.md` | Are new to DeFi/crypto and want to build understanding from the ground up | ~6-10 hours |
| `domain-theory-path.md` | Want rigorous AMM math, market microstructure, and MEV mechanics | ~8-15 hours |
| `project-systems-path.md` | Want to understand the Aurix codebase as quickly as possible | ~3-5 hours |
| `vector-prep-path.md` | Are about to ship one of the three vectors (A: V3 backtester, B: MEV detector, C: ML signal) | ~2-4 hours per vector |
| `interview-fluency-path.md` | Need to talk about Aurix in hiring conversations | ~4-8 hours |

## Path Overlap

The paths intentionally share files. If you complete `foundations-path` and then start `domain-theory-path`, you'll see some overlap in the early files — skim or skip if you've already absorbed them.

```
                       ┌──────────────────┐
                       │  foundations     │
                       └────────┬─────────┘
                                │
              ┌─────────────────┼─────────────────┐
              v                 v                 v
   ┌──────────────────┐  ┌──────────────┐  ┌──────────────┐
   │  domain-theory   │  │  project     │  │  interview   │
   │                  │  │  systems     │  │  fluency     │
   └────────┬─────────┘  └──────┬───────┘  └──────────────┘
            │                   │
            └────────┬──────────┘
                     v
            ┌──────────────────┐
            │  vector-prep     │
            └──────────────────┘
```

`foundations-path` is the prerequisite for everything else. After that, paths branch by goal.

## How To Choose

- **If you don't know what an AMM is** → start with `foundations-path`
- **If you can describe an AMM but want to know the math** → `domain-theory-path`
- **If you want to ship code in Aurix** → `project-systems-path`, then `vector-prep-path`
- **If you have a hiring conversation next week** → `interview-fluency-path`

## Recommended Pairings

- [ ] `foundations-path` + `project-systems-path` — understand the domain enough to navigate the code
- [ ] `domain-theory-path` + `vector-prep-path` (Vector A) — math foundation for the LP backtester
- [ ] `project-systems-path` + `interview-fluency-path` — defend the codebase in conversation
- [ ] `foundations-path` + `interview-fluency-path` — fastest route to comfortable Aurix discussion when starting cold
