//! ABI decoders for V3 pool events. Each decoder takes a raw `EthLog`
//! (topics + data) and returns a typed event row ready for storage
//! insertion. Hex parsing is manual — we already do this in
//! `dex/uniswap_v3.rs:decode_sqrt_price_x96`, and the pattern is small.

use num_bigint::{BigInt, BigUint, Sign};

use crate::storage::pool_events::{PoolEventKind, PoolEventRow};
use crate::storage::swaps::SwapEventRow;

use super::error::IngestError;
use super::source::EthLog;

/// `keccak256("Swap(address,address,int256,int256,uint160,uint128,int24)")`
pub const SWAP_TOPIC0: &str =
    "c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

/// `keccak256("Mint(address,address,int24,int24,uint128,uint256,uint256)")`
pub const MINT_TOPIC0: &str =
    "7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde";

/// `keccak256("Burn(address,int24,int24,uint128,uint256,uint256)")`
pub const BURN_TOPIC0: &str =
    "0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c";

/// `keccak256("Collect(address,address,int24,int24,uint128,uint128)")`
pub const COLLECT_TOPIC0: &str =
    "70935338e69775456a85ddef226c395fb668b63fa0115f5f20610b388e6ca9c0";

const WORD_HEX_LEN: usize = 64;

fn strip_prefix(hex: &str) -> &str {
    hex.trim_start_matches("0x")
}

fn nth_word<'a>(hex: &'a str, n: usize) -> Result<&'a str, IngestError> {
    let stripped = strip_prefix(hex);
    let start = n * WORD_HEX_LEN;
    let end = start + WORD_HEX_LEN;
    if stripped.len() < end {
        return Err(IngestError::MalformedLog(format!(
            "data shorter than {end} hex chars (have {})",
            stripped.len()
        )));
    }
    Ok(&stripped[start..end])
}

fn parse_uint256(word: &str) -> Result<BigUint, IngestError> {
    let bytes = hex::decode(word)?;
    Ok(BigUint::from_bytes_be(&bytes))
}

fn parse_int256(word: &str) -> Result<BigInt, IngestError> {
    let bytes = hex::decode(word)?;
    if bytes.len() != 32 {
        return Err(IngestError::MalformedLog(format!(
            "int256 expects 32 bytes, got {}",
            bytes.len()
        )));
    }
    if bytes[0] & 0x80 == 0 {
        // Non-negative — parse as unsigned and convert.
        let u = BigUint::from_bytes_be(&bytes);
        Ok(BigInt::from_biguint(Sign::Plus, u))
    } else {
        // Negative — two's complement: invert + 1, then negate.
        let mut inv = bytes.clone();
        for b in &mut inv {
            *b = !*b;
        }
        let mut u = BigUint::from_bytes_be(&inv);
        u += 1u8;
        Ok(BigInt::from_biguint(Sign::Minus, u))
    }
}

fn parse_uint128_word(word: &str) -> Result<u128, IngestError> {
    let u = parse_uint256(word)?;
    let digits = u.to_u64_digits();
    match digits.len() {
        0 => Ok(0u128),
        1 => Ok(digits[0] as u128),
        2 => Ok((digits[1] as u128) << 64 | digits[0] as u128),
        _ => Err(IngestError::Overflow("uint128 word")),
    }
}

/// Parses a 32-byte word that holds a sign-extended int24. The value lives
/// in the low 3 bytes; bytes 0..29 are 0xff for negative values.
///
/// Reads only the last 6 hex chars (3 bytes) directly via
/// `u32::from_str_radix` rather than allocating a 32-byte `Vec<u8>` per
/// call (audit finding `ingest.md` §"Avoid per-event byte-by-byte
/// allocation").
fn parse_int24_word(word: &str) -> Result<i32, IngestError> {
    if word.len() != WORD_HEX_LEN {
        return Err(IngestError::MalformedLog(format!(
            "int24 word expects {WORD_HEX_LEN} hex chars, got {}",
            word.len()
        )));
    }
    let raw = u32::from_str_radix(&word[58..64], 16)
        .map_err(|e| IngestError::MalformedLog(format!("int24 parse: {e}")))?;
    // Sign bit lives in the high bit of the most-significant of the
    // three bytes (bit 23 of `raw`).
    let negative = raw & 0x0080_0000 != 0;
    if negative {
        // Sign-extend from 24 to 32 bits.
        Ok((raw | 0xFF00_0000) as i32)
    } else {
        Ok(raw as i32)
    }
}

fn parse_uint160_word(word: &str) -> Result<BigUint, IngestError> {
    parse_uint256(word)
}

fn topic_at<'a>(log: &'a EthLog, idx: usize) -> Result<&'a str, IngestError> {
    log.topics.get(idx).map(|s| s.as_str()).ok_or_else(|| {
        IngestError::MalformedLog(format!("missing topic[{idx}] (have {} topics)", log.topics.len()))
    })
}

/// Decodes a `Swap(address,address,int256,int256,uint160,uint128,int24)` log
/// into a `SwapEventRow` ready for storage.
pub fn decode_swap(log: &EthLog, block_gas_price_gwei: Option<f64>) -> Result<SwapEventRow, IngestError> {
    let t0 = strip_prefix(topic_at(log, 0)?);
    if !t0.eq_ignore_ascii_case(SWAP_TOPIC0) {
        return Err(IngestError::UnsupportedTopic(t0.to_string()));
    }
    let sender = address_from_topic(topic_at(log, 1)?);
    let recipient = address_from_topic(topic_at(log, 2)?);

    let amount0 = parse_int256(nth_word(&log.data, 0)?)?;
    let amount1 = parse_int256(nth_word(&log.data, 1)?)?;
    let sqrt_price_x96 = parse_uint160_word(nth_word(&log.data, 2)?)?;
    let liquidity = parse_uint128_word(nth_word(&log.data, 3)?)?;
    let tick = parse_int24_word(nth_word(&log.data, 4)?)?;

    Ok(SwapEventRow {
        pool_address: log.address.clone(),
        block_number: log.block_number as i64,
        log_index: log.log_index as i64,
        transaction_hash: log.transaction_hash.clone(),
        block_timestamp: log.block_timestamp,
        sender,
        recipient,
        amount0: amount0.to_string(),
        amount1: amount1.to_string(),
        sqrt_price_x96: sqrt_price_x96.to_string(),
        liquidity: liquidity.to_string(),
        tick,
        block_gas_price_gwei,
    })
}

/// `Mint(address sender, address indexed owner, int24 indexed tickLower,
///       int24 indexed tickUpper, uint128 amount, uint256 amount0,
///       uint256 amount1)`
pub fn decode_mint(log: &EthLog) -> Result<PoolEventRow, IngestError> {
    let t0 = strip_prefix(topic_at(log, 0)?);
    if !t0.eq_ignore_ascii_case(MINT_TOPIC0) {
        return Err(IngestError::UnsupportedTopic(t0.to_string()));
    }
    let owner = address_from_topic(topic_at(log, 1)?);
    let tick_lower = parse_int24_word(strip_prefix(topic_at(log, 2)?))?;
    let tick_upper = parse_int24_word(strip_prefix(topic_at(log, 3)?))?;

    // data = sender (32) + amount (32) + amount0 (32) + amount1 (32)
    let _sender = nth_word(&log.data, 0)?;
    let amount = parse_uint256(nth_word(&log.data, 1)?)?;
    let amount0 = parse_uint256(nth_word(&log.data, 2)?)?;
    let amount1 = parse_uint256(nth_word(&log.data, 3)?)?;

    Ok(PoolEventRow {
        pool_address: log.address.clone(),
        block_number: log.block_number as i64,
        log_index: log.log_index as i64,
        transaction_hash: log.transaction_hash.clone(),
        block_timestamp: log.block_timestamp,
        kind: PoolEventKind::Mint,
        owner,
        tick_lower,
        tick_upper,
        liquidity: amount.to_string(),
        amount0: amount0.to_string(),
        amount1: amount1.to_string(),
    })
}

/// `Burn(address indexed owner, int24 indexed tickLower,
///       int24 indexed tickUpper, uint128 amount, uint256 amount0,
///       uint256 amount1)`
pub fn decode_burn(log: &EthLog) -> Result<PoolEventRow, IngestError> {
    let t0 = strip_prefix(topic_at(log, 0)?);
    if !t0.eq_ignore_ascii_case(BURN_TOPIC0) {
        return Err(IngestError::UnsupportedTopic(t0.to_string()));
    }
    let owner = address_from_topic(topic_at(log, 1)?);
    let tick_lower = parse_int24_word(strip_prefix(topic_at(log, 2)?))?;
    let tick_upper = parse_int24_word(strip_prefix(topic_at(log, 3)?))?;

    // data = amount (32) + amount0 (32) + amount1 (32)
    let amount = parse_uint256(nth_word(&log.data, 0)?)?;
    let amount0 = parse_uint256(nth_word(&log.data, 1)?)?;
    let amount1 = parse_uint256(nth_word(&log.data, 2)?)?;

    Ok(PoolEventRow {
        pool_address: log.address.clone(),
        block_number: log.block_number as i64,
        log_index: log.log_index as i64,
        transaction_hash: log.transaction_hash.clone(),
        block_timestamp: log.block_timestamp,
        kind: PoolEventKind::Burn,
        owner,
        tick_lower,
        tick_upper,
        liquidity: amount.to_string(),
        amount0: amount0.to_string(),
        amount1: amount1.to_string(),
    })
}

/// `Collect(address indexed owner, address recipient,
///          int24 indexed tickLower, int24 indexed tickUpper,
///          uint128 amount0, uint128 amount1)`
pub fn decode_collect(log: &EthLog) -> Result<PoolEventRow, IngestError> {
    let t0 = strip_prefix(topic_at(log, 0)?);
    if !t0.eq_ignore_ascii_case(COLLECT_TOPIC0) {
        return Err(IngestError::UnsupportedTopic(t0.to_string()));
    }
    let owner = address_from_topic(topic_at(log, 1)?);
    let tick_lower = parse_int24_word(strip_prefix(topic_at(log, 2)?))?;
    let tick_upper = parse_int24_word(strip_prefix(topic_at(log, 3)?))?;

    // data = recipient (32) + amount0 (32) + amount1 (32)
    let _recipient = nth_word(&log.data, 0)?;
    let amount0 = parse_uint256(nth_word(&log.data, 1)?)?;
    let amount1 = parse_uint256(nth_word(&log.data, 2)?)?;

    Ok(PoolEventRow {
        pool_address: log.address.clone(),
        block_number: log.block_number as i64,
        log_index: log.log_index as i64,
        transaction_hash: log.transaction_hash.clone(),
        block_timestamp: log.block_timestamp,
        kind: PoolEventKind::Collect,
        owner,
        tick_lower,
        tick_upper,
        // collect has no liquidity delta
        liquidity: "0".to_string(),
        amount0: amount0.to_string(),
        amount1: amount1.to_string(),
    })
}

fn address_from_topic(topic: &str) -> String {
    let stripped = strip_prefix(topic);
    if stripped.len() < 40 {
        return format!("0x{stripped}");
    }
    format!("0x{}", &stripped[stripped.len() - 40..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn padded_hex(value: &str, total_chars: usize) -> String {
        let s = value.trim_start_matches("0x");
        format!("{:0>width$}", s, width = total_chars)
    }

    /// Two's-complement-encode an i32 into a 32-byte word.
    fn encode_int24(value: i32) -> String {
        let raw = (value as u32) & 0x00FF_FFFF;
        let high = if value < 0 { 0xFFFF_FF00u32 } else { 0u32 };
        let combined = high | raw;
        format!("{:0>56}{:08x}", "ffffff".repeat(if value < 0 { 4 } else { 0 }).chars().take(0).collect::<String>(), combined)
    }

    /// Encode signed 256-bit value into hex word.
    fn encode_int256(value: i64) -> String {
        if value >= 0 {
            format!("{:0>64x}", value)
        } else {
            // two's complement: encode as 2^256 + value
            let two_to_256 = BigUint::from(1u8) << 256;
            let big = two_to_256 - BigUint::from((-value) as u64);
            format!("{:0>64x}", big)
        }
    }

    fn encode_uint(value: &BigUint) -> String {
        format!("{:0>64}", value.to_str_radix(16))
    }

    #[test]
    fn parse_uint256_round_trip() {
        let word = padded_hex("16345785d8a0000", 64);  // 1e17
        let r = parse_uint256(&word).unwrap();
        assert_eq!(r, BigUint::parse_bytes(b"100000000000000000", 10).unwrap());
    }

    #[test]
    fn parse_int256_negative_uses_twos_complement() {
        // -1 = 0xfff..fff
        let word = "f".repeat(64);
        let r = parse_int256(&word).unwrap();
        assert_eq!(r, BigInt::from(-1));
    }

    #[test]
    fn parse_int256_positive() {
        let word = padded_hex("64", 64);  // 100
        let r = parse_int256(&word).unwrap();
        assert_eq!(r, BigInt::from(100));
    }

    #[test]
    fn parse_int24_zero_and_positive() {
        let word_zero = "0".repeat(64);
        assert_eq!(parse_int24_word(&word_zero).unwrap(), 0);

        // tick = 100 (0x64)
        let word_100 = padded_hex("64", 64);
        assert_eq!(parse_int24_word(&word_100).unwrap(), 100);

        // tick = 887272 (MAX_TICK)  = 0xd89e8
        let word_max = padded_hex("d89e8", 64);
        assert_eq!(parse_int24_word(&word_max).unwrap(), 887272);
    }

    #[test]
    fn parse_int24_negative_uses_sign_extension() {
        // tick = -1 → low 3 bytes = 0xffffff with high bytes = 0xff (sign-extended)
        let word_neg1 = "f".repeat(64);
        assert_eq!(parse_int24_word(&word_neg1).unwrap(), -1);

        // tick = -100 → low 3 bytes = 0xffff9c; rest = 0xff
        let word_neg100 = format!("{}ffff9c", "f".repeat(58));
        assert_eq!(parse_int24_word(&word_neg100).unwrap(), -100);
    }

    fn build_swap_log(
        pool: &str,
        block: u64,
        log_idx: u64,
        amount0: i64,
        amount1: i64,
        sqrt_price_x96: &str,
        liquidity: u128,
        tick: i32,
    ) -> EthLog {
        let topic0 = format!("0x{SWAP_TOPIC0}");
        let topic1 = format!("0x{:0>64}", "1234567890abcdef");
        let topic2 = format!("0x{:0>64}", "fedcba0987654321");
        let data = format!(
            "0x{}{}{}{}{}",
            encode_int256(amount0),
            encode_int256(amount1),
            encode_uint(&BigUint::parse_bytes(sqrt_price_x96.as_bytes(), 10).unwrap()),
            encode_uint(&BigUint::from(liquidity)),
            {
                let raw = (tick as u32) & 0x00FF_FFFF;
                if tick < 0 {
                    format!("{:0>58}{:06x}", "f".repeat(58), raw)
                } else {
                    format!("{:0>64x}", raw)
                }
            }
        );
        EthLog {
            address: pool.to_string(),
            block_number: block,
            log_index: log_idx,
            transaction_hash: format!("0x{:0>64}", "deadbeef"),
            block_timestamp: 1_700_000_000 + (block as i64) * 12,
            topics: vec![topic0, topic1, topic2],
            data,
        }
    }

    #[test]
    fn decode_swap_round_trip_synthetic_log() {
        // Synthetic swap: 1e18 token0 paid in (positive), 3e9 token1 received (negative),
        // sqrtPriceX96 ≈ 1.382e30, liquidity 1e18, tick 200500.
        let sqrt_str = "1382037470929380185091293796";
        let log = build_swap_log(
            "0xpool", 19000000, 200,
            1_000_000_000_000_000_000i64,
            -3_000_000_000i64,
            sqrt_str,
            1_000_000_000_000_000_000u128,
            200_500,
        );
        let row = decode_swap(&log, Some(20.0)).unwrap();
        assert_eq!(row.amount0, "1000000000000000000");
        assert_eq!(row.amount1, "-3000000000");
        assert_eq!(row.sqrt_price_x96, sqrt_str);
        assert_eq!(row.liquidity, "1000000000000000000");
        assert_eq!(row.tick, 200_500);
        assert_eq!(row.block_number, 19_000_000);
        assert_eq!(row.log_index, 200);
        assert_eq!(row.block_gas_price_gwei, Some(20.0));
    }

    #[test]
    fn decode_swap_negative_tick() {
        let log = build_swap_log("0xp", 1, 1, 1, -1, "100000", 1, -123);
        let row = decode_swap(&log, None).unwrap();
        assert_eq!(row.tick, -123);
    }

    #[test]
    fn decode_swap_unknown_topic_errors() {
        let mut log = build_swap_log("0xp", 1, 1, 1, -1, "1", 1, 0);
        log.topics[0] = format!("0x{}", "a".repeat(64));
        assert!(matches!(decode_swap(&log, None), Err(IngestError::UnsupportedTopic(_))));
    }

    #[test]
    fn decode_swap_short_data_errors() {
        let mut log = build_swap_log("0xp", 1, 1, 1, -1, "1", 1, 0);
        log.data = "0x".to_string();
        assert!(matches!(decode_swap(&log, None), Err(IngestError::MalformedLog(_))));
    }

    fn build_mint_log(
        pool: &str,
        block: u64,
        log_idx: u64,
        owner: &str,
        tick_lower: i32,
        tick_upper: i32,
        amount: u128,
        amount0: u64,
        amount1: u64,
    ) -> EthLog {
        let topic0 = format!("0x{MINT_TOPIC0}");
        let topic1 = format!("0x{:0>64}", owner.trim_start_matches("0x"));
        let encode_t = |t: i32| {
            let raw = (t as u32) & 0x00FF_FFFF;
            if t < 0 {
                format!("0x{:0>58}{:06x}", "f".repeat(58), raw)
            } else {
                format!("0x{:0>64x}", raw)
            }
        };
        let topic2 = encode_t(tick_lower);
        let topic3 = encode_t(tick_upper);

        let sender_word = format!("{:0>64}", "1234");
        let amount_word = format!("{:0>64x}", amount);
        let amount0_word = format!("{:0>64x}", amount0);
        let amount1_word = format!("{:0>64x}", amount1);
        let data = format!("0x{sender_word}{amount_word}{amount0_word}{amount1_word}");

        EthLog {
            address: pool.to_string(),
            block_number: block,
            log_index: log_idx,
            transaction_hash: format!("0x{:0>64}", "cafe"),
            block_timestamp: 1_700_000_000 + (block as i64) * 12,
            topics: vec![topic0, topic1, topic2, topic3],
            data,
        }
    }

    #[test]
    fn decode_mint_round_trip() {
        let log = build_mint_log(
            "0xpool",
            18_000_000,
            42,
            "0xabcdef",
            -100,
            100,
            500_000_000u128,
            1_000_000_000_000_000_000u64,
            3_000_000_000u64,
        );
        let row = decode_mint(&log).unwrap();
        assert_eq!(row.kind, PoolEventKind::Mint);
        assert_eq!(row.tick_lower, -100);
        assert_eq!(row.tick_upper, 100);
        assert_eq!(row.liquidity, "500000000");
        assert_eq!(row.amount0, "1000000000000000000");
        assert_eq!(row.amount1, "3000000000");
        assert!(row.owner.ends_with("abcdef"));
    }

    #[test]
    fn decode_burn_with_negative_ticks() {
        let mut log = build_mint_log(
            "0xp", 1, 1, "0xa", -887_000, -800_000, 1, 1, 1,
        );
        log.topics[0] = format!("0x{BURN_TOPIC0}");
        // burn data is amount + amount0 + amount1 (no sender prefix)
        let amount_word = format!("{:0>64x}", 1u64);
        log.data = format!("0x{amount_word}{amount_word}{amount_word}");
        let row = decode_burn(&log).unwrap();
        assert_eq!(row.kind, PoolEventKind::Burn);
        assert_eq!(row.tick_lower, -887_000);
        assert_eq!(row.tick_upper, -800_000);
    }

    #[test]
    fn address_from_topic_extracts_low_20_bytes() {
        let s = format!(
            "0x{:0>64}",
            "abcdef0123456789abcdef0123456789abcdef01"
        );
        let addr = address_from_topic(&s);
        assert_eq!(addr, "0xabcdef0123456789abcdef0123456789abcdef01");
    }
}
