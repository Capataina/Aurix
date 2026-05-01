# Decision: Read-Only by Design

## Decision Summary

Aurix never submits transactions, never holds private keys, never requests wallet connection. The application can only read on-chain state via `eth_call` (read-only RPC method); it has no code path that writes to the chain.

This is a **non-negotiable architectural commitment** documented in `context/architecture.md` as one of the project's four core principles.

## Alternatives Considered

| Alternative | Description | Why Rejected |
|---|---|---|
| **Write-capable execution layer** | Add a Solidity-aware transaction builder, wallet integration, signing flow | Changes Aurix's identity from "analytics" to "trading tool" — a fundamentally different product with different security implications |
| **Read-only by default with opt-in writes** | Allow advanced users to enable execution via a setting | Creates a code path that must be defended against accidental activation; security/clarity trade-off doesn't justify the optionality |
| **Watch-only wallet integration** | Read positions for a connected wallet but never sign | This is allowed (Tab 3 will do this), but is conceptually still read-only — the wallet provides the address, not the signing key |
| **Status quo: pure observation** ✓ | Aurix never has a code path that submits transactions | Chosen |

## Why The Chosen Path Won

### 1. Safety

A tool that cannot write to the chain cannot drain funds. The blast radius of any Aurix bug is bounded at "wrong information displayed" — not "transactions submitted in error" or "funds at risk." For a personal-use analytics tool, this is the right safety profile.

### 2. Trust

Users should be able to run Aurix on the same machine that holds their wallet keys (e.g. MetaMask in the same browser, hardware wallet plugged in nearby) without concern. The read-only guarantee makes this trust explicit. If Aurix could submit transactions, a clever exploit could potentially trigger a transaction without user awareness.

### 3. Scope clarity

Analytics and execution are different products with different requirements:
- Analytics: latency-tolerant, observation-focused, can run on free public RPC, doesn't need MEV protection
- Execution: latency-sensitive, action-focused, needs Flashbots or similar, needs gas optimisation, needs signing infrastructure

Building both into one product creates ambiguity about what the tool is for and significantly increases attack surface. Aurix is the analytics product. A separate "Aurix Execute" tool (if it ever existed) would be a different project with its own discipline.

### 4. The five-tab vision is consistent

All five planned tabs are read-only by nature:
- Tab 1 (Arbitrage Scanner) — observes; never executes
- Tab 2 (LP Backtester) — replays historical data; never deploys capital
- Tab 3 (Wallet Tracker) — reads on-chain positions for a given address; never signs
- Tab 4 (Gas Monitor) — reads gas prices; never spends gas
- Tab 5 (Risk Dashboard) — computes risk metrics; never executes hedges

The read-only commitment isn't a limitation that constrains the vision — it's consistent with the vision.

## Trade-Offs Accepted

| What we give up | Why it's acceptable |
|---|---|
| Cannot ever extend Aurix into a trading tool without violating the principle | If we want a trading tool later, fork — don't mutate Aurix's identity |
| Can't capture arbitrage opportunities that would be profitable | Aurix is an analytics tool; capturing profit isn't the goal |
| Can't act on the ML signals Vector C produces | The ML signal is portfolio signal — the building, not the trading |
| Lose users who want a one-stop "monitor + trade" tool | Different product audience; not Aurix's target |

## Downstream Consequences

This decision shapes many smaller decisions:

- **No ethers-rs** (decision file): we don't need a transaction builder, just a JSON-RPC client. Hand-crafted ABI for `eth_call` suffices.
- **No wallet integration** in the React layer: no MetaMask connect button, no WalletConnect, no signature requests
- **Gas modelling is observational**: we estimate gas costs for the dashboard but never actually spend gas
- **MEV exposure is zero**: Aurix never submits to the mempool; it doesn't need MEV protection
- **The tab shell** (when built) won't have a "wallet" or "settings" surface for execution

## How To Tell If This Decision Is Being Violated

Concrete signals:

1. Adding a dependency that includes transaction signing (e.g. `ethers-rs` with signer features, `alloy-signer`, hardware wallet integration libraries)
2. Adding a Tauri command that takes a private key, mnemonic, or signing material as input
3. Calling any RPC method other than `eth_call`, `eth_gasPrice`, `eth_blockNumber`, or other read-only methods
4. Adding a UI element that prompts for wallet connection
5. Importing any library whose primary purpose is transaction submission (e.g. Flashbots SDK)

If you see any of these in a PR, the read-only principle is being violated. Discuss before merging.

## Links

- `context/architecture.md` — implementation-facing version with the principle stated
- `concepts/domain-patterns/mev-and-transaction-ordering.md` — what we're avoiding by not executing
- `project/decisions/no-ethers-rs-handcrafted-abi.md` — a derived decision
- `project/systems/what-aurix-observes.md` — the resulting product surface
