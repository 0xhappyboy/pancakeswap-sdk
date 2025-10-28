use crate::{EvmClient, EvmError, PancakeSwapConfig};
use ethers::types::{Address, U256};
use std::sync::Arc;

/// Liquidity management service for DEX operations
pub struct LiquidityService {
    client: Arc<EvmClient>,
}

impl LiquidityService {

    /// create liquidity service
    pub fn new(client: Arc<EvmClient>) -> Self {
        Self { client }
    }

    /// Retrieves the pair address for two tokens from a DEX factory
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    /// async fn example(service: LiquidityService) -> Result<(), EvmError> {
    /// let factory = Address::from_str("0x1234...").unwrap();
    /// let token_a = Address::from_str("0x5678...").unwrap();
    /// let token_b = Address::from_str("0x9abc...").unwrap();
    ///
    /// match service.get_pair_info(factory, token_a, token_b).await? {
    ///     Some(pair) => println!("Pair address: {:?}", pair),
    ///     None => println!("No pair exists for these tokens"),
    /// }
    /// Ok(())
    /// }
    /// ```
    pub async fn get_pair_info(
        &self,
        factory_address: Address,
        token_a: Address,
        token_b: Address,
    ) -> Result<Option<Address>, EvmError> {
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, self.client.provider.clone());

        factory
            .get_pair(token_a, token_b)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pair info: {}", e)))
            .map(|pair| {
                if pair == Address::zero() {
                    None
                } else {
                    Some(pair)
                }
            })
    }

    /// Gets the reserves of a liquidity pool
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    /// async fn example(service: LiquidityService) -> Result<(), EvmError> {
    /// let pair_address = Address::from_str("0x1234...").unwrap();
    /// let (reserve0, reserve1, timestamp) = service.get_reserves(pair_address).await?;
    /// println!("Reserves: {} and {}", reserve0, reserve1);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_reserves(&self, pair_address: Address) -> Result<(U256, U256, u32), EvmError> {
        let pair = crate::abi::IPancakePair::new(pair_address, self.client.provider.clone());

        let (reserve0, reserve1, block_timestamp_last) = pair
            .get_reserves()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get reserves: {}", e)))?;

        Ok((reserve0.into(), reserve1.into(), block_timestamp_last))
    }

    /// Retrieves the token addresses of a liquidity pool
    pub async fn get_pair_tokens(
        &self,
        pair_address: Address,
    ) -> Result<(Address, Address), EvmError> {
        let pair = crate::abi::IPancakePair::new(pair_address, self.client.provider.clone());

        let token0 = pair
            .token_0()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get token0: {}", e)))?;

        let token1 = pair
            .token_1()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get token1: {}", e)))?;

        Ok((token0, token1))
    }

    /// Gets a user liquidity balance in a pool
    pub async fn get_user_liquidity(
        &self,
        pair_address: Address,
        user_address: Address,
    ) -> Result<U256, EvmError> {
        let pair = crate::abi::IPancakePair::new(pair_address, self.client.provider.clone());
        pair.balance_of(user_address)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get user liquidity: {}", e)))
    }

    /// Gets the total supply of LP tokens for a pool
    pub async fn get_total_supply(&self, pair_address: Address) -> Result<U256, EvmError> {
        let pair = crate::abi::IPancakePair::new(pair_address, self.client.provider.clone());

        pair.total_supply()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get total supply: {}", e)))
    }

    /// Calculates the value of liquidity position
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    /// async fn example(service: LiquidityService) -> Result<(), EvmError> {
    /// let pair_address = Address::from_str("0x1234...").unwrap();
    /// let liquidity = U256::from(1000u64);
    /// let token_a_price = 1.5;
    /// let token_b_price = 2.0;
    ///
    /// let (value_a, value_b, total) = service
    ///     .cal_liquidity_value(pair_address, liquidity, token_a_price, token_b_price)
    ///     .await?;
    ///
    /// println!("Token A value: ${}, Token B value: ${}, Total: ${}", value_a, value_b, total);
    /// Ok(())
    /// }
    /// ```
    pub async fn cal_liquidity_value(
        &self,
        pair_address: Address,
        liquidity_amount: U256,
        token_a_price: f64,
        token_b_price: f64,
    ) -> Result<(f64, f64, f64), EvmError> {
        let total_supply = self.get_total_supply(pair_address).await?;
        let (reserve_a, reserve_b, _) = self.get_reserves(pair_address).await?;

        if total_supply.is_zero() {
            return Ok((0.0, 0.0, 0.0));
        }

        let user_token_a = (liquidity_amount * reserve_a) / total_supply;
        let user_token_b = (liquidity_amount * reserve_b) / total_supply;

        let value_a = user_token_a.as_u128() as f64 * token_a_price;
        let value_b = user_token_b.as_u128() as f64 * token_b_price;
        let total_value = value_a + value_b;

        Ok((value_a, value_b, total_value))
    }

    /// Retrieves multiple pairs from a factory contract
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    /// async fn example(service: LiquidityService) -> Result<(), EvmError> {
    /// let factory = Address::from_str("0x1234...").unwrap();
    /// let pairs = service.get_all_pairs(factory, 0, 10).await?;
    /// println!("Found {} pairs", pairs.len());
    /// Ok(())
    /// }
    /// ```
    pub async fn get_all_pairs(
        &self,
        factory_address: Address,
        start_index: u64,
        count: u64,
    ) -> Result<Vec<Address>, EvmError> {
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, self.client.provider.clone());

        let total_pairs =
            factory.all_pairs_length().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get total pairs: {}", e))
            })?;

        let end_index = std::cmp::min(start_index + count, total_pairs.as_u64());
        let mut pairs = Vec::new();

        for i in start_index..end_index {
            let pair_address = factory.all_pairs(i.into()).call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get pair at index {}: {}", i, e))
            })?;

            pairs.push(pair_address);
        }

        Ok(pairs)
    }

    /// Gets comprehensive information about a liquidity pool
    pub async fn get_pool_info(&self, pair_address: Address) -> Result<PoolInfo, EvmError> {
        let (token0, token1) = self.get_pair_tokens(pair_address).await?;
        let (reserve0, reserve1, block_timestamp_last) = self.get_reserves(pair_address).await?;
        let total_supply = self.get_total_supply(pair_address).await?;
        Ok(PoolInfo {
            pair_address,
            token0,
            token1,
            reserve0,
            reserve1,
            block_timestamp_last,
            total_supply,
        })
    }
}

/// Comprehensive liquidity pool information
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pair_address: Address,
    pub token0: Address,
    pub token1: Address,
    pub reserve0: U256,
    pub reserve1: U256,
    pub block_timestamp_last: u32,
    pub total_supply: U256,
}

impl PoolInfo {
    /// Calculates the price of one token relative to another in the pool
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    /// fn example(pool: PoolInfo) -> Result<(), EvmError> {
    /// let base_token = Address::from_str("0x1234...").unwrap();
    /// let price = pool.cal_price(base_token)?;
    /// println!("Price: {}", price);
    /// Ok(())
    /// }
    /// ```
    pub fn cal_price(&self, base_token: Address) -> Result<f64, EvmError> {
        if self.reserve0.is_zero() || self.reserve1.is_zero() {
            return Err(EvmError::CalculationError("Reserves are zero".to_string()));
        }

        if base_token == self.token0 {
            Ok(self.reserve1.as_u128() as f64 / self.reserve0.as_u128() as f64)
        } else if base_token == self.token1 {
            Ok(self.reserve0.as_u128() as f64 / self.reserve1.as_u128() as f64)
        } else {
            Err(EvmError::CalculationError("Invalid base token".to_string()))
        }
    }
}
