//! M2.8 — Capital allocation headline analysis.
//!
//! Reference: `vector-a-v3-lp-backtester.md` §M2.8.

#![allow(dead_code)]

pub mod error;
pub mod regime;
pub mod verdict;

pub use error::HeadlineError;
pub use regime::{classify_terciles, monthly_realized_vol, VolRegime};
pub use verdict::{HeadlineConfig, HeadlineMonthlyInput, HeadlineOutput, HeadlineRunner};
