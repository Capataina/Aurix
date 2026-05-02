//! Active pair catalog.
//!
//! ## To add a new pair
//!
//! 1. Pick or add a `Token` constructor in `tokens.rs`.
//! 2. Identify the venue addresses for that pair: V3 pool addresses, V2 / V2-fork
//!    factory addresses.
//! 3. Add a `fn <pair_name>() -> PairConfig` builder below.
//! 4. Append the new builder to `list_pairs()`.
//!
//! That is the entire change required. The dispatch in `commands/market.rs`
//! and the frontend pair selector both pick up the new pair automatically.

use serde::Serialize;

use super::tokens::{usdc, wbtc, weth, Token};
use super::venues::{VenueConfig, VenueProtocol};

const UNISWAP_V2_FACTORY: &str = "0x5C69bEe701ef814A2B6a3EDD4B1652CB9cc5aA6f";
const SUSHISWAP_V2_FACTORY: &str = "0xC0AEe478e3658e2610c5F7A4A2E1777Ce9e4f2Ac";

/// A tradable token pair plus the venues we read prices from.
#[derive(Debug, Clone)]
pub struct PairConfig {
    /// Stable identifier used by the frontend (e.g. "weth-usdc").
    pub id: String,
    /// Display label (e.g. "WETH / USDC").
    pub label: String,
    /// Numerator token (the asset whose price we report).
    pub base: Token,
    /// Denominator token (USD-like quote in this catalog).
    pub quote: Token,
    /// Ordered venue list. `venues[0]` is the hero / primary venue.
    pub venues: Vec<VenueConfig>,
}

/// Public summary of a pair, for the frontend's pair selector.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairSummary {
    pub id: String,
    pub label: String,
    pub base_symbol: String,
    pub quote_symbol: String,
    pub venue_count: usize,
}

impl From<&PairConfig> for PairSummary {
    fn from(pair: &PairConfig) -> Self {
        Self {
            id: pair.id.clone(),
            label: pair.label.clone(),
            base_symbol: pair.base.symbol.clone(),
            quote_symbol: pair.quote.symbol.clone(),
            venue_count: pair.venues.len(),
        }
    }
}

/// Returns every registered pair, in display order.
pub fn list_pairs() -> Vec<PairConfig> {
    vec![weth_usdc(), wbtc_usdc()]
}

/// Looks up a pair by `id`. Returns `None` for unknown ids.
pub fn find_pair(id: &str) -> Option<PairConfig> {
    list_pairs().into_iter().find(|pair| pair.id == id)
}

fn weth_usdc() -> PairConfig {
    PairConfig {
        id: "weth-usdc".to_string(),
        label: "WETH / USDC".to_string(),
        base: weth(),
        quote: usdc(),
        venues: vec![
            VenueConfig {
                dex_name: "Uniswap V3 5bps".to_string(),
                protocol: VenueProtocol::UniswapV3 {
                    pool_address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(),
                    fee_tier_bps: 5,
                },
            },
            VenueConfig {
                dex_name: "Uniswap V3 30bps".to_string(),
                protocol: VenueProtocol::UniswapV3 {
                    pool_address: "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8".to_string(),
                    fee_tier_bps: 30,
                },
            },
            VenueConfig {
                dex_name: "Uniswap V2".to_string(),
                protocol: VenueProtocol::UniswapV2 {
                    factory_address: UNISWAP_V2_FACTORY.to_string(),
                    fee_tier_bps: 30,
                },
            },
            VenueConfig {
                dex_name: "SushiSwap".to_string(),
                protocol: VenueProtocol::UniswapV2 {
                    factory_address: SUSHISWAP_V2_FACTORY.to_string(),
                    fee_tier_bps: 30,
                },
            },
        ],
    }
}

fn wbtc_usdc() -> PairConfig {
    PairConfig {
        id: "wbtc-usdc".to_string(),
        label: "WBTC / USDC".to_string(),
        base: wbtc(),
        quote: usdc(),
        venues: vec![
            VenueConfig {
                dex_name: "Uniswap V3 5bps".to_string(),
                protocol: VenueProtocol::UniswapV3 {
                    pool_address: "0x9a772018FbD77fcD2d25657e5C547BAfF3Fd7D16".to_string(),
                    fee_tier_bps: 5,
                },
            },
            VenueConfig {
                dex_name: "Uniswap V3 30bps".to_string(),
                protocol: VenueProtocol::UniswapV3 {
                    pool_address: "0x99ac8cA7087fA4A2A1FB6357269965A2014ABc35".to_string(),
                    fee_tier_bps: 30,
                },
            },
            VenueConfig {
                dex_name: "Uniswap V2".to_string(),
                protocol: VenueProtocol::UniswapV2 {
                    factory_address: UNISWAP_V2_FACTORY.to_string(),
                    fee_tier_bps: 30,
                },
            },
            VenueConfig {
                dex_name: "SushiSwap".to_string(),
                protocol: VenueProtocol::UniswapV2 {
                    factory_address: SUSHISWAP_V2_FACTORY.to_string(),
                    fee_tier_bps: 30,
                },
            },
        ],
    }
}
