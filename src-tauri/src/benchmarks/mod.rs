//! M2.7 — Multi-asset benchmark module.
//!
//! Reference: `vector-a-v3-lp-backtester.md` §M2.7.

#![allow(dead_code, unused_imports)]

pub mod alpha;
pub mod beaconchain;
pub mod defi;
pub mod error;
pub mod http;
pub mod tradfi;
pub mod v2lp;

pub use alpha::{alpha_summary, AlphaSummary};
pub use beaconchain::BeaconChainProvider;
pub use defi::{
    DefiLlamaProvider, AAVE_V3_USDC_SUPPLY_POOL, COMPOUND_V3_USDC_SUPPLY_POOL, LIDO_STETH_POOL,
};
pub use error::BenchmarkError;
pub use http::{HttpFetcher, MockHttpFetcher, ReqwestFetcher};
pub use tradfi::{
    TradFiProvider, FRED_DGS1_URL, FRED_DGS3MO_URL, FRED_GOLD_LBMA_URL, STOOQ_VOO_URL,
    STOOQ_XAUUSD_URL,
};
pub use v2lp::{hodl_equity_series, v2_lp_equity_series};
