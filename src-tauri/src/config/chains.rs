//! Per-chain configuration for V3 LP backtesting.
//!
//! Each chain has its own Uniswap V3 subgraph, free public RPC, and
//! block-time conventions. Centralised here so adapters resolve URLs
//! by `ChainId` instead of hardcoding mainnet everywhere.

use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainId {
    Ethereum,
    Arbitrum,
    Optimism,
    Base,
    Polygon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    UniswapV3,
    SushiswapV3,
    PancakeswapV3,
}

#[allow(dead_code)]
impl Protocol {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "uniswap-v3" => Some(Self::UniswapV3),
            "sushiswap-v3" => Some(Self::SushiswapV3),
            "pancakeswap-v3" => Some(Self::PancakeswapV3),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::UniswapV3 => "uniswap-v3",
            Self::SushiswapV3 => "sushiswap-v3",
            Self::PancakeswapV3 => "pancakeswap-v3",
        }
    }
}

#[allow(dead_code)] // tier 4 wires native-token pricing
impl ChainId {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ethereum" => Some(Self::Ethereum),
            "arbitrum" => Some(Self::Arbitrum),
            "optimism" => Some(Self::Optimism),
            "base" => Some(Self::Base),
            "polygon" => Some(Self::Polygon),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Arbitrum => "arbitrum",
            Self::Optimism => "optimism",
            Self::Base => "base",
            Self::Polygon => "polygon",
        }
    }

    /// Public RPC default — used when the user has no Alchemy key.
    /// All five are free and require no auth.
    pub fn public_rpc_url(&self) -> &'static str {
        match self {
            Self::Ethereum => "https://eth.llamarpc.com",
            Self::Arbitrum => "https://arb1.arbitrum.io/rpc",
            Self::Optimism => "https://mainnet.optimism.io",
            Self::Base => "https://mainnet.base.org",
            Self::Polygon => "https://polygon-rpc.com",
        }
    }

    /// Subgraph ID on The Graph's decentralized gateway. Used when
    /// `THE_GRAPH_API_KEY` is set.
    pub fn subgraph_id(&self) -> &'static str {
        match self {
            // Uniswap V3 official deployments.
            Self::Ethereum => "5zvR82QoaXYFyDEKLZ9t6v9adgnptxYpKpSbxtgVENFV",
            Self::Arbitrum => "FbCGRftH4a3yZugY7TnbYgPJVEv2LvMT6oF1fxPe9aJM",
            Self::Optimism => "Cghf4LfVqPiFw6fp6Y5X5Ubc8UpmUhSfJL82zwiBFLaj",
            Self::Base => "HMuAwufqZ1YCRmzL2SfHTVkzZovC9VL2UAKhjvRqKiR1",
            Self::Polygon => "3hCPRGf4z88VC5rsBKU5AA9FBBq5nF3jbKJG7VZCbhjm",
        }
    }

    /// Legacy hosted-service URL — many remained alive after the 2024
    /// deprecation. Tried as a no-key fallback when the gateway key
    /// isn't configured.
    pub fn legacy_subgraph_url(&self) -> &'static str {
        match self {
            Self::Ethereum => "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3",
            Self::Arbitrum => "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3-arbitrum-ii",
            Self::Optimism => "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3-optimism",
            Self::Base => "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3-base",
            Self::Polygon => "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3-polygon",
        }
    }

    /// Resolves the best available subgraph URL given env state.
    /// Prefers the gateway when a Graph API key is configured;
    /// otherwise falls back to the legacy hosted URL. Defaults to
    /// Uniswap V3.
    pub fn subgraph_url(&self) -> String {
        self.subgraph_url_for(Protocol::UniswapV3)
    }

    /// Subgraph URL for a specific (chain, protocol) pair. V3 forks
    /// (Sushi, Pancake) share the Uniswap V3 schema so the same
    /// GraphQL queries work — only the URL differs.
    pub fn subgraph_url_for(&self, protocol: Protocol) -> String {
        // Gateway path with API key — we don't currently track Sushi/
        // Pancake subgraph IDs on the gateway; fall through to the
        // legacy URL for forks. Users with The Graph keys still get
        // the gateway for Uniswap V3 itself.
        if let Ok(key) = env::var("THE_GRAPH_API_KEY") {
            if matches!(protocol, Protocol::UniswapV3) {
                return format!(
                    "https://gateway.thegraph.com/api/{key}/subgraphs/id/{}",
                    self.subgraph_id()
                );
            }
        }
        self.legacy_subgraph_url_for(protocol).to_string()
    }

    /// Per-(chain, protocol) legacy hosted URL. For protocol/chain
    /// combinations we don't support, returns the closest viable
    /// URL — caller should expect it to fail gracefully.
    pub fn legacy_subgraph_url_for(&self, protocol: Protocol) -> &'static str {
        match (self, protocol) {
            // ---- Uniswap V3 (existing) ----
            (Self::Ethereum, Protocol::UniswapV3) => self.legacy_subgraph_url(),
            (Self::Arbitrum, Protocol::UniswapV3) => self.legacy_subgraph_url(),
            (Self::Optimism, Protocol::UniswapV3) => self.legacy_subgraph_url(),
            (Self::Base, Protocol::UniswapV3) => self.legacy_subgraph_url(),
            (Self::Polygon, Protocol::UniswapV3) => self.legacy_subgraph_url(),
            // ---- Sushi V3 ----
            (Self::Ethereum, Protocol::SushiswapV3) => {
                "https://api.thegraph.com/subgraphs/name/sushi-v3/v3-ethereum"
            }
            (Self::Arbitrum, Protocol::SushiswapV3) => {
                "https://api.thegraph.com/subgraphs/name/sushi-v3/v3-arbitrum"
            }
            (Self::Optimism, Protocol::SushiswapV3) => {
                "https://api.thegraph.com/subgraphs/name/sushi-v3/v3-optimism"
            }
            (Self::Base, Protocol::SushiswapV3) => {
                "https://api.thegraph.com/subgraphs/name/sushi-v3/v3-base"
            }
            (Self::Polygon, Protocol::SushiswapV3) => {
                "https://api.thegraph.com/subgraphs/name/sushi-v3/v3-polygon"
            }
            // ---- Pancake V3 ----
            (Self::Ethereum, Protocol::PancakeswapV3) => {
                "https://api.thegraph.com/subgraphs/name/pancakeswap/exchange-v3-eth"
            }
            (Self::Arbitrum, Protocol::PancakeswapV3) => {
                "https://api.thegraph.com/subgraphs/name/pancakeswap/exchange-v3-arb"
            }
            // Pancake V3 isn't deployed on Optimism/Base/Polygon —
            // fall back to Ethereum URL; subgraph will return empty
            // for any pool not in scope. Caller surfaces as "no swaps
            // in range" via empty ingestion.
            _ => "https://api.thegraph.com/subgraphs/name/pancakeswap/exchange-v3-eth",
        }
    }

    /// Native gas-paying token — drives gas-cost USD valuation for the
    /// backtest engine. Polygon is the only non-ETH chain in our
    /// supported set.
    pub fn native_token_symbol(&self) -> &'static str {
        match self {
            Self::Polygon => "MATIC",
            _ => "ETH",
        }
    }

    /// DefiLlama coin id for native-token USD price lookup. Used by
    /// the backend's `lp_native_token_usd` IPC (tier 4 leverages the
    /// same plumbing for arbitrary token USD prices).
    pub fn native_coingecko_id(&self) -> &'static str {
        match self {
            Self::Polygon => "coingecko:matic-network",
            _ => "coingecko:ethereum",
        }
    }

    /// Approximate seconds-per-block — used for block↔time conversions
    /// and the grid runner's blocks_per_day default.
    pub fn block_time_seconds(&self) -> f64 {
        match self {
            Self::Ethereum => 12.0,
            Self::Arbitrum => 0.25,
            Self::Optimism => 2.0,
            Self::Base => 2.0,
            Self::Polygon => 2.2,
        }
    }
}
