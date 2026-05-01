# Decision: Tauri over Electron

## Decision Summary

Aurix is built on Tauri 2 rather than Electron. The frontend (React 19 + TypeScript) runs in the OS's native webview rather than a bundled Chromium instance. The backend is Rust rather than Node.js.

## Alternatives Considered

| Alternative | Description | Why Rejected |
|---|---|---|
| **Electron** | Bundled Chromium + Node.js backend | Larger binary (100-200 MB), higher memory footprint, weaker security model, Node.js backend insufficient for big-integer math performance |
| **Tauri 1** | Older version, less mature | v2 is GA, supports more platforms, has better APIs |
| **Web app (no desktop)** | Pure browser-based, no native shell | Loses the "local-first" identity — would need a hosted backend for any non-trivial work |
| **Native (egui, gtk, qt)** | No HTML/CSS/JS at all, fully native UI | Loses access to the rich web frontend ecosystem (charts, animations, dev tooling) |
| **Tauri 2 + Rust + React** ✓ | Chosen | Below |

## Why The Chosen Path Won

### 1. Binary size and footprint

| Concern | Electron | Tauri |
|---|---|---|
| Binary size | 100-200 MB | 5-15 MB |
| Memory footprint | High (bundled Chromium) | Low (native webview) |
| Cold-start latency | ~1-2 seconds | <500ms |

For a desktop analytics tool that should "feel native," Tauri's footprint matters. A 5 MB binary is the right size for a tool you might recommend a friend install.

### 2. Backend language

Aurix's backend does:
- Big-integer arithmetic (256-bit `sqrtPriceX96` decoding)
- Concurrent network I/O (5 RPC calls per second)
- Hand-rolled ABI encoding (byte manipulation)

All three are areas where Rust outperforms Node.js significantly:
- Rust's `num-bigint` is faster than JavaScript's `BigInt` and ergonomically clearer
- Rust's `tokio` provides explicit, debuggable async; Node's event loop hides scheduling
- Rust's type system catches encoding bugs at compile time; JS catches them at runtime

Future tabs will lean even more on Rust's strengths. Tab 2 (LP backtester) needs exact tick math over millions of swap events — well-suited to Rust. Tab 5 (risk modelling) involves heavy numerical computation. Tab 4 (gas prediction) might involve ML inference. All are easier in Rust than in Node.

### 3. Frontend ecosystem

We still want the rich frontend ecosystem — React for UI, modern build tooling (Vite), TypeScript for type safety, the ability to use any npm package for charts/animations/utilities.

Tauri gives us this: Rust backend + React frontend. The JS ecosystem dominates frontend tooling; Rust dominates systems work. We get both.

### 4. Security model

Tauri uses a capability-based security model. The frontend has access only to specific Tauri commands the backend explicitly exposes. No filesystem access, no shell execution, no arbitrary native APIs unless the backend opts into them.

Electron's default model is "Node.js access from the renderer" — strictly looser. Securing Electron requires explicit context isolation, content security policies, and node integration disabling. Tauri starts secure and you opt into permissions.

### 5. Cross-platform story

Tauri compiles to native binaries for macOS (Intel + ARM), Windows, Linux. Same source, three targets. Electron does similarly but with much larger binaries per target.

### 6. Caner's existing stack

The user has multiple Tauri projects (Aurix, NeuroDrive, Flat Browser, Image Browser). Mastering one stack across multiple projects is more valuable than spreading across stacks. Tauri being shared across the portfolio also means improvements (build patterns, IPC conventions, error handling discipline) transfer between projects.

## Trade-Offs Accepted

| What we give up | Why it's acceptable |
|---|---|
| Cross-runtime maintenance burden (Rust ↔ TypeScript type mirror) | Manageable for current scope; will need codegen at scale |
| Learning curve for Rust async + Tauri specifics | Already absorbed; pays off across multiple Tauri projects |
| Some npm packages don't have Rust equivalents | We've avoided those situations so far; `num-bigint` covers the critical math |
| Native webview behaviour varies by OS (Safari WebView on macOS, Edge WebView on Windows) | Browser-style differences but minor for a charting + table app |
| Less hire-able pool for "Aurix maintainers" — Rust + Tauri is rarer than Electron + Node | Acceptable for a personal-portfolio project |

## Downstream Consequences

- **The cross-runtime contract** becomes a thing that needs maintenance (covered in `project/architecture/cross-runtime-contract.md`)
- **Backend logic stays in Rust** — there's no escape hatch for "let me write this in TypeScript" because Tauri's frontend is webview-only
- **IPC commands are the integration surface** — every new feature needs a new `#[tauri::command]`
- **Build tooling diverges** — `cargo` for backend, `pnpm` for frontend, Tauri CLI orchestrates both

## When To Revisit

Reconsider this decision if:

- Tauri 2 becomes deprecated or unmaintained (currently active)
- The team needs to expand and Rust+Tauri becomes a hiring blocker (not currently a concern)
- A specific feature requires a Node.js library with no Rust equivalent (hasn't happened yet)
- Native UI becomes preferable for a specific platform integration (Mac menu bar app, Windows tray app — these are Tauri-supported but might justify Native for some scenarios)

## Links

- `project/architecture/two-runtime-tauri-rust-react.md` — the broader architecture this decision shapes
- `project/decisions/rust-backend-over-pure-typescript.md` — the related backend-language decision
- `context/architecture.md` — implementation-facing reference
