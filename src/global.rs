use ethers::types::Address;
use std::str::FromStr;

// BSC V2
pub const BSC_FACTORY_V2: &str = "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73";
pub const BSC_ROUTER_V2: &str = "0x10ED43C718714eb63d5aA57B78B54704E256024E";
// BSC V3
pub const BSC_FACTORY_V3: &str = "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865";
pub const BSC_ROUTER_V3: &str = "0x13f4EA83D0bd40E75C8222255bc855a974568Dd4";
// Ethereum Mainnet V3
pub const ETHEREUM_FACTORY_V3: &str = "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865";
pub const ETHEREUM_ROUTER_V3: &str = "0x13f4EA83D0bd40E75C8222255bc855a974568Dd4";
// Ethereum Mainnet V2
pub const ETHEREUM_FACTORY_V2: &str = "0x1097053Fd2ea711dad45caCcc45EfF7548fCB362";
pub const ETHEREUM_ROUTER_V2: &str = "0xEfF92A263d31888d860bD50809A8D171709b7b1c";
// Base V2
pub const BASE_FACTORY_V2: &str = "0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E";
pub const BASE_ROUTER_V2: &str = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506";
// Base V3
pub const BASE_FACTORY_V3: &str = "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865";
pub const BASE_ROUTER_V3: &str = "0x13f4EA83D0bd40E75C8222255bc855a974568Dd4";
// Polygon Mainnet
pub const POLYGON_FACTORY: &str = "0xc35DADB65012eC5796536bD9864eD8773aBc74C4";
pub const POLYGON_ROUTER: &str = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506";
// Arbitrum One
pub const ARBITRUM_FACTORY: &str = "0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E";
pub const ARBITRUM_ROUTER: &str = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506";
// BSC Quoter
pub const BSC_QUOTER: &str = "0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997";
// Ethereum Quoter
pub const ETHEREUM_QUOTER: &str = "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6";
// Base Quoter
pub const BASE_QUOTER: &str = "0x672b7Be0bC5334b342F306Aaa6D812E6f39c353B";
pub const BSC_STABLE_SWAP_FACTORY: &str = "0x36bBb66e7E7Ef21b42608C17Ef7D68A6c6dFB3b7";
pub const BSC_STABLE_SWAP_ROUTER: &str = "0x1698a2220f472A2d18e8D0f268F8e277B21c8F68";
pub const BSC_MASTERCHEF_V2: &str = "0xa5f8C5Dbd5F286960b9d90548680aE5ebFf07652";
pub const BSC_POSITION_MANAGER: &str = "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364";
pub const FOUR_MEME_ADDRESS: &str = "0x5c952063c7fc8610FFDB798152D69F0B9550762b";

pub fn parse_address(address_str: &str) -> Result<Address, Box<dyn std::error::Error>> {
    Ok(Address::from_str(address_str)?)
}

pub struct PrecomputedAddresses;

impl PrecomputedAddresses {
    pub fn v2_ethereum_addresses_provider() -> Address {
        Address::from_str("0xB53C1a33016B2DC2fF3653530bfF1848a515c8c5")
            .expect("Invalid V2 Ethereum addresses provider address")
    }

    pub fn v3_ethereum_addresses_provider() -> Address {
        Address::from_str("0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e")
            .expect("Invalid V3 Ethereum addresses provider address")
    }

    pub fn weth() -> Address {
        Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
            .expect("Invalid WETH address")
    }

    pub fn usdc() -> Address {
        Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
            .expect("Invalid USDC address")
    }
}
