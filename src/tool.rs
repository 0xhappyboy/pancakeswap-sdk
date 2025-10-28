use crate::types::{
    BurnEvent, MintEvent, PairCreatedEvent, SwapEvent, V3BurnEvent, V3MintEvent, V3SwapEvent,
};
use ethers::types::{H160, U256};

pub mod event_parsers {
    use super::*;
    use ethers::types::Log;

    pub fn parse_swap_log(log: &Log) -> Result<SwapEvent, Box<dyn std::error::Error>> {
        if log.topics.len() < 3 {
            return Err("Invalid swap log: insufficient topics".into());
        }

        let sender = H160::from_slice(&log.topics[1].as_bytes()[12..]);
        let to = H160::from_slice(&log.topics[2].as_bytes()[12..]);

        let data = log.data.clone().to_vec();
        if data.len() < 192 {
            return Err("Invalid swap log: insufficient data".into());
        }

        let amount0_in = U256::from_big_endian(&data[0..32]);
        let amount1_in = U256::from_big_endian(&data[32..64]);
        let amount0_out = U256::from_big_endian(&data[64..96]);
        let amount1_out = U256::from_big_endian(&data[96..128]);

        Ok(SwapEvent {
            sender,
            to,
            amount0_in,
            amount1_in,
            amount0_out,
            amount1_out,
        })
    }

    pub fn parse_mint_log(log: &Log) -> Result<MintEvent, Box<dyn std::error::Error>> {
        if log.topics.len() < 2 {
            return Err("Invalid mint log: insufficient topics".into());
        }

        let sender = H160::from_slice(&log.topics[1].as_bytes()[12..]);

        let data = log.data.clone().to_vec();
        if data.len() < 64 {
            return Err("Invalid mint log: insufficient data".into());
        }

        let amount0 = U256::from_big_endian(&data[0..32]);
        let amount1 = U256::from_big_endian(&data[32..64]);

        Ok(MintEvent {
            sender,
            amount0,
            amount1,
        })
    }

    pub fn parse_burn_log(log: &Log) -> Result<BurnEvent, Box<dyn std::error::Error>> {
        if log.topics.len() < 3 {
            return Err("Invalid burn log: insufficient topics".into());
        }

        let sender = H160::from_slice(&log.topics[1].as_bytes()[12..]);
        let to = H160::from_slice(&log.topics[2].as_bytes()[12..]);

        let data = log.data.clone().to_vec();
        if data.len() < 64 {
            return Err("Invalid burn log: insufficient data".into());
        }

        let amount0 = U256::from_big_endian(&data[0..32]);
        let amount1 = U256::from_big_endian(&data[32..64]);

        Ok(BurnEvent {
            sender,
            to,
            amount0,
            amount1,
        })
    }

    pub fn parse_pair_created_log(
        log: &Log,
    ) -> Result<PairCreatedEvent, Box<dyn std::error::Error>> {
        if log.topics.len() < 3 {
            return Err("Invalid pair created log: insufficient topics".into());
        }

        let token0 = H160::from_slice(&log.topics[1].as_bytes()[12..]);
        let token1 = H160::from_slice(&log.topics[2].as_bytes()[12..]);

        let data = log.data.clone().to_vec();
        if data.len() < 32 {
            return Err("Invalid pair created log: insufficient data".into());
        }

        let pair = H160::from_slice(&data[12..32]);

        Ok(PairCreatedEvent {
            token0,
            token1,
            pair,
        })
    }

    pub fn parse_v3_swap_log(log: &Log) -> Result<V3SwapEvent, Box<dyn std::error::Error>> {
        if log.topics.len() < 4 {
            return Err("Invalid V3 swap log: insufficient topics".into());
        }

        let sender = H160::from_slice(&log.topics[1].as_bytes()[12..]);
        let recipient = H160::from_slice(&log.topics[2].as_bytes()[12..]);

        let data = log.data.clone().to_vec();
        if data.len() < 128 {
            return Err("Invalid V3 swap log: insufficient data".into());
        }

        let amount0 = U256::from_big_endian(&data[0..32]);
        let amount1 = U256::from_big_endian(&data[32..64]);
        let sqrt_price_x96 = U256::from_big_endian(&data[64..96]);
        let liquidity = U256::from_big_endian(&data[96..128]);
        let tick = i32::from_be_bytes(data[128..132].try_into().unwrap_or([0; 4]));

        Ok(V3SwapEvent {
            sender,
            recipient,
            amount0,
            amount1,
            sqrt_price_x96,
            liquidity,
            tick,
        })
    }

    pub fn parse_v3_mint_log(log: &Log) -> Result<V3MintEvent, Box<dyn std::error::Error>> {
        if log.topics.len() < 4 {
            return Err("Invalid V3 mint log: insufficient topics".into());
        }
        let sender = H160::from_slice(&log.topics[1].as_bytes()[12..]);
        let owner = H160::from_slice(&log.topics[2].as_bytes()[12..]);
        let data = log.data.clone().to_vec();
        if data.len() < 128 {
            return Err("Invalid V3 mint log: insufficient data".into());
        }

        let tick_lower = bytes_to_i24(&data[0..3]);
        let tick_upper = bytes_to_i24(&data[3..6]);
        let amount = U256::from_big_endian(&data[6..38]);
        let amount0 = U256::from_big_endian(&data[38..70]);
        let amount1 = U256::from_big_endian(&data[70..102]);
        Ok(V3MintEvent {
            sender,
            owner,
            tick_lower: tick_lower as i32,
            tick_upper: tick_upper as i32,
            amount,
            amount0,
            amount1,
        })
    }

    pub fn parse_v3_burn_log(log: &Log) -> Result<V3BurnEvent, Box<dyn std::error::Error>> {
        if log.topics.len() < 4 {
            return Err("Invalid V3 burn log: insufficient topics".into());
        }
        let owner = H160::from_slice(&log.topics[1].as_bytes()[12..]);
        let data = log.data.clone().to_vec();
        if data.len() < 96 {
            return Err("Invalid V3 burn log: insufficient data".into());
        }

        let tick_lower = bytes_to_i24(&data[0..3]);
        let tick_upper = bytes_to_i24(&data[3..6]);
        let amount = U256::from_big_endian(&data[6..38]);
        let amount0 = U256::from_big_endian(&data[38..70]);
        let amount1 = U256::from_big_endian(&data[70..102]);
        Ok(V3BurnEvent {
            owner,
            tick_lower: tick_lower as i32,
            tick_upper: tick_upper as i32,
            amount,
            amount0,
            amount1,
        })
    }

    fn bytes_to_i24(bytes: &[u8]) -> i32 {
        if bytes.len() != 3 {
            return 0;
        }

        let mut extended = [0u8; 4];
        extended[1..4].copy_from_slice(bytes);

        if bytes[0] & 0x80 != 0 {
            extended[0] = 0xFF;
        }
        i32::from_be_bytes(extended)
    }
}

pub mod math_utils {
    use super::*;

    pub fn calculate_amount_out(
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, Box<dyn std::error::Error>> {
        if amount_in.is_zero() {
            return Ok(U256::zero());
        }
        if reserve_in.is_zero() || reserve_out.is_zero() {
            return Err("Reserves cannot be zero".into());
        }

        let amount_in_with_fee = amount_in * U256::from(997);
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * U256::from(1000) + amount_in_with_fee;

        if denominator.is_zero() {
            return Err("Denominator is zero".into());
        }

        Ok(numerator / denominator)
    }

    pub fn calculate_amount_in(
        amount_out: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, Box<dyn std::error::Error>> {
        if amount_out.is_zero() {
            return Ok(U256::zero());
        }
        if reserve_in.is_zero() || reserve_out.is_zero() {
            return Err("Reserves cannot be zero".into());
        }
        if amount_out >= reserve_out {
            return Err("Insufficient reserve out".into());
        }

        let numerator = reserve_in * amount_out * U256::from(1000);
        let denominator = (reserve_out - amount_out) * U256::from(997);

        if denominator.is_zero() {
            return Err("Denominator is zero".into());
        }

        Ok((numerator / denominator) + U256::one())
    }

    pub fn calculate_v3_price(sqrt_price_x96: U256) -> f64 {
        let price = (sqrt_price_x96.as_u128() as f64).powi(2) / (2.0_f64.powi(192));
        price
    }

    pub fn calculate_v3_tick_price(tick: i32) -> f64 {
        1.0001_f64.powi(tick)
    }

    pub fn calculate_slippage(expected_amount: U256, actual_amount: U256) -> f64 {
        if expected_amount.is_zero() {
            return 0.0;
        }

        let expected = expected_amount.as_u128() as f64;
        let actual = actual_amount.as_u128() as f64;

        ((expected - actual) / expected * 100.0).abs()
    }
}

pub mod address_utils {
    use std::str::FromStr;

    use super::*;

    pub fn is_zero_address(address: &H160) -> bool {
        address == &H160::zero()
    }

    pub fn to_checksum(address: &H160) -> String {
        let addr_str = format!("{:?}", address);
        let hash = ethers::utils::keccak256(addr_str.to_lowercase().as_bytes());
        let mut checksum = String::with_capacity(42);

        checksum.push_str("0x");

        for (i, char) in addr_str[2..].chars().enumerate() {
            let byte = hash[i / 2];
            if i % 2 == 0 {
                if (byte >> 4) >= 8 {
                    checksum.push(char.to_ascii_uppercase());
                } else {
                    checksum.push(char.to_ascii_lowercase());
                }
            } else {
                if (byte & 0x0f) >= 8 {
                    checksum.push(char.to_ascii_uppercase());
                } else {
                    checksum.push(char.to_ascii_lowercase());
                }
            }
        }

        checksum
    }

    pub fn is_valid_address(address: &str) -> bool {
        if !address.starts_with("0x") || address.len() != 42 {
            return false;
        }

        H160::from_str(address).is_ok()
    }
}

pub mod time_utils {
    use super::*;

    pub fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn calculate_deadline(minutes_from_now: u64) -> u64 {
        current_timestamp() + minutes_from_now * 60
    }

    pub fn is_expired(deadline: u64) -> bool {
        current_timestamp() > deadline
    }
}
