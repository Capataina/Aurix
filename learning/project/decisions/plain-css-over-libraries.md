# Decision: Plain CSS over Component Libraries

## Decision Summary

Aurix's frontend uses plain CSS (in `src/styles/theme.css` and `src/styles/dashboard.css`) — no Tailwind, no shadcn/ui, no Chakra, no Material UI, no Radix. The visual system is hand-built.

## Alternatives Considered

| Alternative | Description | Why Rejected |
|---|---|---|
| **Tailwind CSS** | Utility-first framework; classes like `flex p-4 bg-slate-900` | Defaults to consumer-app aesthetics; would need significant override for trading-terminal feel; class-name spam in JSX |
| **shadcn/ui** | Pre-built React components with Tailwind | Same Tailwind concerns plus opinionated component API |
| **Chakra UI** | Component library with theming | Light, consumer-app default styling; bundle size; runtime theming overhead |
| **Material UI** | Google's design language in React form | Distinctly Google-shaped; not the trading-terminal aesthetic |
| **Plain CSS** ✓ | Hand-written stylesheets, BEM-like class naming | Chosen — see below |

## Why The Chosen Path Won

### 1. Aesthetic control

Aurix targets a **dark, dense, monitoring-app aesthetic** — think Bloomberg Terminal, trading dashboards, real-time observability tools. Not consumer apps.

Component libraries default to:
- Light backgrounds with rounded soft surfaces
- Generous padding (consumer-friendly tap targets)
- Sans-serif friendly fonts
- Subtle animations meant to feel "delightful"

The trading-terminal aesthetic wants:
- Dark backgrounds with high-contrast data
- Tight padding (information density)
- Monospace-friendly numbers
- Minimal animations (focus on data not motion)

Overriding a component library to achieve this is more work than just writing the CSS directly.

### 2. No abstraction overhead

Plain CSS is debuggable in dev tools without library knowledge. When you inspect an element and see:

```html
<div class="venue-card venue-card--active">
```

You search for `.venue-card` and find the styles. Done. No need to understand a library's class-generation strategy or its theming layer.

For a solo developer maintaining the project across multiple sessions, this matters. Library-specific conventions decay from memory; CSS conventions are universal.

### 3. Bundle size

Plain CSS adds ~11 KB total to the bundle (`theme.css` ~5 KB + `dashboard.css` ~6 KB). Tailwind's output is much larger even after purging unused classes; shadcn brings in a Radix dependency tree. Component libraries are typically 50-200 KB minified.

For a desktop app where the bundle ships once at install time, this matters less than for a web app. But it's still cleaner.

### 4. Cross-project consistency

Caner's other Tauri+React projects (NeuroDrive, Cernio, Flat Browser, Image Browser) all use plain CSS. Maintaining consistency across the portfolio means improvements transfer between projects (a clean dark theme pattern from one becomes reusable in others without reconciling library choices).

### 5. The frontend isn't the differentiator

Aurix's hiring signal is in the backend (Rust + ABI + tick math) and the analytics layer (insights engine, eventually ML). The frontend just needs to be readable and not embarrassing. Plain CSS is sufficient.

A more polished frontend wouldn't add proportional signal — most of the audience for Aurix (crypto-quant hiring managers, technical contributors) judges the Rust code, not the CSS.

## Trade-Offs Accepted

| What we give up | Why it's acceptable |
|---|---|
| Pre-built accessible components (modals, dropdowns, tooltips) | Aurix doesn't currently need these; if needed later, build minimally |
| Design-system enforcement (consistent spacing, colors) | Discipline via convention; theme.css defines the tokens |
| Speed of prototyping new screens | Aurix doesn't have many screens; the cost is small |
| Familiar component patterns for new contributors | New contributors are unlikely; Caner is the maintainer |
| Animation libraries (Framer Motion, etc.) | Trading dashboards don't need elaborate animations |

## Downstream Consequences

- **Each component manages its own classes** — no theming context, no style props
- **theme.css defines tokens**: colours, spacing, type scale, accent variants. Components reference these via CSS custom properties (`--color-fg-primary`, `--spacing-tight`)
- **dashboard.css holds layout** — grid configurations, responsive breakpoints, dashboard-specific composition
- **Adding a new component** means writing its CSS by hand. ~30-60 minutes for a substantial new component
- **No automatic dark mode** because Aurix is dark-mode-only by design — there's no light theme

## How To Tell If This Decision Is Being Compromised

If a PR adds:
- `tailwindcss` or `@tailwindcss/*` packages
- `shadcn/ui`, `@chakra-ui/*`, `@mui/*`
- `styled-components`, `emotion`, or any CSS-in-JS library
- A `theme provider` component

The decision is being violated. Discuss before merging — the framework might be justified for a specific feature, but the project's plain-CSS commitment should be reconsidered explicitly rather than drifted away from.

## When To Revisit

Reconsider this decision if:

- Aurix grows to many distinct screens (10+) where consistency becomes painful to maintain by hand
- A future tab needs complex interactive components (data tables with column resizing, drag-and-drop) where building from scratch is genuinely costly
- A second contributor joins and needs faster onboarding
- The visual brief moves toward consumer-app aesthetic (very unlikely given Aurix's positioning)

## Concrete Example

Aurix's venue card in `PriceCard.tsx`:

```tsx
<div className={`price-card ${snapshot ? 'price-card--ready' : 'price-card--loading'}`}>
  <header className="price-card-header">
    <span className="price-card-eyebrow">{snapshot?.dexName ?? "Uniswap V3"}</span>
    <span className="price-card-pair">{snapshot?.pairLabel ?? "WETH / USDC"}</span>
  </header>
  <div className="price-card-value">{formatUsd(snapshot?.priceUsd ?? 0)}</div>
  ...
</div>
```

Compare to Tailwind:

```tsx
<div className="bg-slate-900 border border-slate-700 rounded-md p-4 ...">
  <header className="flex justify-between items-center mb-2">
    <span className="text-xs uppercase text-slate-500">{snapshot?.dexName ?? "Uniswap V3"}</span>
    ...
  </header>
  ...
</div>
```

Both render. The plain CSS version keeps the JSX clean (one class per element); the Tailwind version puts the styling intent in the markup. Personal preference matters here, and Aurix's preference is plain CSS.

## Links

- `project/decisions/read-only-by-design.md` — the broader design principles
- `context/systems/arbitrage-gui.md` — implementation truth for the GUI layer
- `project/architecture/two-runtime-tauri-rust-react.md` — broader architecture
