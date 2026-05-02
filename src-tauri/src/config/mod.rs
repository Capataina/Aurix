//! Configuration layer for the Aurix backend.
//!
//! All values that used to be scattered as constants across DEX adapters and
//! the market command live here in one of five purpose-built submodules:
//!
//! | Submodule | Holds |
//! | --- | --- |
//! | `rpc`      | Process-environment RPC configuration (`MAINNET_RPC_URL` / `ALCHEMY_API_KEY`). |
//! | `tokens`   | Canonical mainnet `Token` definitions (address, decimals, symbol). |
//! | `venues`   | `VenueProtocol` enum and `VenueConfig` describing a single tradable surface. |
//! | `pairs`    | `PairConfig` plus the active pair catalog. **Edit this file to add a pair.** |
//! | `runtime`  | Cross-cutting runtime parameters (chain label, default gas-units estimate). |
//!
//! Adding a new pair is a one-file edit: append a new `PairConfig` to the
//! catalog in `pairs.rs`. Adding a new protocol is a two-file change: extend
//! `VenueProtocol` in `venues.rs` and add the corresponding adapter dispatch in
//! `commands/market.rs`.

pub mod pairs;
pub mod rpc;
pub mod runtime;
pub mod tokens;
pub mod venues;

pub use pairs::{find_pair, list_pairs, PairConfig, PairSummary};
pub use rpc::AppConfig;
pub use runtime::RuntimeConfig;
pub use tokens::Token;
pub use venues::VenueProtocol;

// Re-exported for crate-level API completeness; reserved for future callers.
#[allow(unused_imports)]
pub use rpc::ConfigError;
#[allow(unused_imports)]
pub use venues::VenueConfig;
