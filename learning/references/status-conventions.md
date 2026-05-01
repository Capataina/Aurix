# Status Conventions

How the archive labels material at different stages of project reality.

## Why This Matters

The archive teaches Aurix as it is now (Tab 1 partial), as it's planned to be (the five-tab vision), as it would extend (the three vectors), and as it once was (any superseded approaches we still teach for the contrast). Without explicit labels, learners can confuse aspiration with reality.

## The Labels

### Current in the project runtime

The code does this NOW. You can run Aurix and see it.

- Used for: the polling loop, the V3 + V2 decoders, the insight engine, the React dashboard
- Examples in archive:
  - `project/architecture/the-1hz-loadsnapshot-tick.md` — current
  - `project/systems/insight-engine-anatomy.md` — current
  - `project/architecture/two-runtime-tauri-rust-react.md` — current

### Foundational domain knowledge

True regardless of whether Aurix exists. Pure theory.

- Used for: AMM math, market microstructure, MEV concepts, statistics
- Examples:
  - `concepts/foundations/markets-and-prices.md`
  - `concepts/core/amm-mechanics-v2-and-v3.md`
  - `concepts/advanced/statistical-primitives-for-risk-modelling.md`

These files don't need status labels because they're context-independent.

### Planned project direction

The README or vector plans describe this. Code does not yet exist.

- Used for: Tabs 2-5 (LP backtesting, wallet tracking, gas intelligence, risk modelling), the three vectors (A/B/C)
- Examples:
  - `concepts/advanced/uniswap-v3-tick-mathematics.md` — labeled "Status: Foundational domain knowledge. Not yet implemented in Aurix — Vector A's plan is to implement this."
  - `concepts/advanced/mempool-mev-detection-mechanics.md` — labeled "Status: ... Not yet implemented in Aurix — Vector B's plan is to implement this."
  - `project/evolution/vector-roadmap.md` — labeled at the level of individual vectors

### Superseded in implementation

The project moved away from this approach. Retained because the contrast teaches something or because understanding why we moved away is valuable.

- Used for: NONE currently. Aurix is too young to have superseded approaches.
- Would be used for example if: V2-only adapter was replaced with multi-DEX adapter (we'd retain the V2-only material to show the simpler model)

### Historical but still educationally useful

Material that's no longer current but still worth understanding.

- Used for: NONE currently in Aurix.
- Would be used for example if: an early polling architecture was replaced with WebSocket subscriptions; the polling material would be retained to teach the contrast.

## Where Status Labels Appear

- **Concept files**: in the front matter or first section if the topic isn't current in Aurix
- **Project files**: only when ambiguity could mislead the learner
- **Path files**: never (paths are navigation, not material)
- **Glossary entries**: only when the term refers to something not yet implemented (e.g. `Mempool` entry notes Vector B is the eventual implementation)
- **Materials**: never (materials are external resources)
- **Exercises**: never (exercises are practice; their relationship to project state is implicit)
- **References**: never (these files describe themselves)

## How to Apply Labels in New Files

### When NOT to label

- Foundational domain knowledge files don't need labels — they're true regardless
- Files that describe current Aurix state don't need labels — current is the default
- Files in `materials/` and `references/` describe themselves; no project-state label

### When to label

- File teaches something that's planned but not implemented (Vectors A/B/C, Tabs 2-5)
- File compares current vs planned approach
- File describes an implementation detail that has changed recently (and the prior version is still material)

### Format

Status sections appear early in the file, typically after the "Why This Matters" section:

```markdown
## Status

Foundational domain knowledge. Not yet implemented in Aurix — Vector A's plan is to implement this.
```

Keep it short. Two sentences max. The label + the relevant project state.

## Examples

Good label:

> "Foundational domain knowledge. Not yet implemented in Aurix — Vector C's plan is to implement this."

Bad label (too vague):

> "Status: Coming soon."

Good label (compares present and future):

> "Currently the dashboard shows venue prices via 1 Hz polling. Vector B's plan adds mempool subscription via WebSocket — this file teaches both."

Bad label (apologetic):

> "Note: this is aspirational and may be outdated. Verify before relying on this."

The latter is exactly the disclaimer language to avoid (per the orient skill's discipline). Either the file describes current reality, or it describes planned direction; both are valid; the disclaimer style is not.

## Related Files

- `LEARNING_MAP.md` — covers how status labels work at the archive level
- `references/notation-guide.md` — the other reference file
