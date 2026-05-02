//! Venue-level configuration.
//!
//! A `VenueConfig` describes a single tradable surface (one DEX × one pool)
//! for one pair. The `protocol` field carries the protocol-specific data the
//! adapter needs (pool address for V3, factory address for V2).

/// Protocol-specific venue configuration.
///
/// Adding a new protocol means: (a) extend this enum with a new variant
/// carrying its required fields, (b) add a new `dex/<protocol>.rs` adapter,
/// and (c) extend the dispatch table in `commands/market.rs`.
#[derive(Debug, Clone)]
pub enum VenueProtocol {
    /// Uniswap V3 — concentrated-liquidity pool with `slot0()` price feed.
    UniswapV3 {
        /// 20-byte hex pool address.
        pool_address: String,
        /// Fee tier in basis points (5, 30, 100, 10000).
        fee_tier_bps: u16,
    },
    /// Uniswap V2 / V2-fork — reserve-ratio pool resolved via factory lookup.
    UniswapV2 {
        /// 20-byte hex factory address. The factory's `getPair(base, quote)`
        /// is called per fetch to resolve the active pair address.
        factory_address: String,
        /// Display fee tier in basis points (V2 fork fees are typically 30 bps).
        fee_tier_bps: u16,
    },
}

/// One row in a pair's venue list. The `dex_name` is the cross-system
/// identity key used by the frontend `dexName` map (see
/// `context/notes/dex-name-contract.md`). It must remain stable across
/// releases of a given venue or the GUI venue lookup will silently break.
#[derive(Debug, Clone)]
pub struct VenueConfig {
    pub dex_name: String,
    pub protocol: VenueProtocol,
}

impl VenueConfig {
    #[allow(dead_code)] // reserved for future analytics that need the venue fee tier directly
    pub fn fee_tier_bps(&self) -> u16 {
        match &self.protocol {
            VenueProtocol::UniswapV3 { fee_tier_bps, .. } => *fee_tier_bps,
            VenueProtocol::UniswapV2 { fee_tier_bps, .. } => *fee_tier_bps,
        }
    }
}
