//! Canonical mainnet token registry.
//!
//! Each token here is the unique on-chain ERC-20 used by all venues for a
//! given asset (e.g. `WETH` is always `0xC02aaA…6Cc2`). Adding a new token is
//! a one-line addition: define a constructor function and call it from
//! `pairs.rs` when registering a pair that uses the token.

use serde::Serialize;

/// A single ERC-20 asset.
///
/// `address` is stored in mixed-case hex but compared case-insensitively when
/// determining token0/token1 ordering (Uniswap pools order tokens by ascending
/// address — see [`Token::is_lower_than`]).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
}

impl Token {
    pub fn new(symbol: impl Into<String>, address: impl Into<String>, decimals: u8) -> Self {
        Self {
            symbol: symbol.into(),
            address: address.into(),
            decimals,
        }
    }

    /// Returns true if `self` would be `token0` of a Uniswap-style pool when
    /// paired with `other`. Pools order tokens by ascending hex address.
    pub fn is_lower_than(&self, other: &Token) -> bool {
        self.address.to_lowercase() < other.address.to_lowercase()
    }
}

/// Wrapped Ether — `0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2`, 18 decimals.
pub fn weth() -> Token {
    Token::new("WETH", "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2", 18)
}

/// Circle USD — `0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48`, 6 decimals.
pub fn usdc() -> Token {
    Token::new("USDC", "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", 6)
}

/// Tether USD — `0xdAC17F958D2ee523a2206206994597C13D831ec7`, 6 decimals.
#[allow(dead_code)] // reserved for future pair definitions
pub fn usdt() -> Token {
    Token::new("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7", 6)
}

/// Wrapped Bitcoin — `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599`, 8 decimals.
pub fn wbtc() -> Token {
    Token::new("WBTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", 8)
}

/// MakerDAO DAI — `0x6B175474E89094C44Da98b954EedeAC495271d0F`, 18 decimals.
#[allow(dead_code)] // reserved for future pair definitions
pub fn dai() -> Token {
    Token::new("DAI", "0x6B175474E89094C44Da98b954EedeAC495271d0F", 18)
}
