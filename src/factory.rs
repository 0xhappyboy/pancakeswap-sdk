use crate::{
    EvmClient, EvmError,
    abi::IUniswapV3Factory,
    global::{
        BASE_FACTORY_V2, BASE_FACTORY_V3, BSC_FACTORY_V2, BSC_FACTORY_V3, ETHEREUM_FACTORY_V2,
        ETHEREUM_FACTORY_V3,
    },
};
use ethers::{
    middleware::SignerMiddleware,
    types::{Address, H256, U256},
};
use std::sync::Arc;

/// pancakeswap factory service
pub struct FactoryService {
    client: Arc<EvmClient>,
}

impl FactoryService {
    /// create a factory service
    pub fn new(client: Arc<EvmClient>) -> Self {
        Self { client }
    }

    /// Retrieves all liquidity pools (V2 and V3) for a given token address
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// let factory_service = FactoryService::new(Arc::clone(&client));
    /// let token_address = "0x...".parse::<Address>().unwrap();
    /// async {
    /// let pools = factory_service.get_pools_by_token(token_address).await?;
    /// Ok::<(), EvmError>(())
    /// };
    /// ```
    pub async fn get_pools_by_token(
        &self,
        token_address: Address,
    ) -> Result<Vec<Address>, EvmError> {
        let mut pools = Vec::new();
        if let Ok(v2_pools) = self.get_v2_pools_by_token(token_address).await {
            pools.extend(v2_pools);
        }
        if let Ok(v3_pools) = self.get_v3_pools_by_token(token_address).await {
            pools.extend(v3_pools);
        }
        Ok(pools)
    }

    /// Get the V2 liquidity pool address
    async fn get_v2_pools_by_token(
        &self,
        token_address: Address,
    ) -> Result<Vec<Address>, EvmError> {
        let factory_address = match self.client.chain {
            crate::EvmType::Bsc => BSC_FACTORY_V2.parse::<Address>().unwrap(),
            crate::EvmType::Ethereum => ETHEREUM_FACTORY_V2.parse::<Address>().unwrap(),
            crate::EvmType::Base => BASE_FACTORY_V2.parse::<Address>().unwrap(),
            _ => return Err(EvmError::ConfigError("Unsupported chain".to_string())),
        };
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, Arc::clone(&self.client.provider));
        let total_pairs =
            factory.all_pairs_length().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get total pairs: {}", e))
            })?;
        let mut pools = Vec::new();
        let max_check = 500u64;
        for i in 0..std::cmp::min(total_pairs.as_u64(), max_check) {
            if let Ok(pair_address) = factory.all_pairs(i.into()).call().await {
                let pair =
                    crate::abi::IPancakePair::new(pair_address, Arc::clone(&self.client.provider));
                if let Ok(token0) = pair.token_0().call().await {
                    if let Ok(token1) = pair.token_1().call().await {
                        if token0 == token_address || token1 == token_address {
                            pools.push(pair_address);
                        }
                    }
                }
            }
        }

        Ok(pools)
    }

    /// Get the V3 liquidity pool address
    async fn get_v3_pools_by_token(
        &self,
        token_address: Address,
    ) -> Result<Vec<Address>, EvmError> {
        let factory_address = match self.client.chain {
            crate::EvmType::Bsc => BSC_FACTORY_V3.parse::<Address>().unwrap(),
            crate::EvmType::Ethereum => ETHEREUM_FACTORY_V3.parse::<Address>().unwrap(),
            crate::EvmType::Base => BASE_FACTORY_V3.parse::<Address>().unwrap(),
            _ => return Err(EvmError::ConfigError("Unsupported chain".to_string())),
        };
        let factory = IUniswapV3Factory::new(factory_address, Arc::clone(&self.client.provider));
        let fee_tiers = vec![100, 500, 2500, 10000];
        let mut pools = Vec::new();
        let common_tokens = vec![match self.client.chain {
            crate::EvmType::Bsc => "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"
                .parse()
                .unwrap(), // WBNB
            crate::EvmType::Ethereum => "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                .parse()
                .unwrap(), // WETH
            _ => Address::zero(),
        }];
        for other_token in common_tokens {
            if other_token == Address::zero() || other_token == token_address {
                continue;
            }
            for &fee in &fee_tiers {
                if let Ok(pool_address) = factory
                    .get_pool(token_address, other_token, fee)
                    .call()
                    .await
                {
                    if pool_address != Address::zero() {
                        pools.push(pool_address);
                    }
                }
            }
        }
        Ok(pools)
    }

    /// Gets the pair address for two tokens
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// let factory_service = FactoryService::new(Arc::clone(&client));
    /// let factory_address = "0x...".parse::<Address>().unwrap();
    /// let token_a = "0x...".parse::<Address>().unwrap();
    /// let token_b = "0x...".parse::<Address>().unwrap();
    /// async {
    /// let pair = factory_service.get_pair(factory_address, token_a, token_b).await?;
    /// Ok::<(), EvmError>(())
    /// };
    /// ```
    pub async fn get_pair(
        &self,
        factory_address: Address,
        token_a: Address,
        token_b: Address,
    ) -> Result<Option<Address>, EvmError> {
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, self.client.provider.clone());
        let pair = factory
            .get_pair(token_a, token_b)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pair: {}", e)))?;
        Ok(if pair == Address::zero() {
            None
        } else {
            Some(pair)
        })
    }

    /// Creates a new pair for two tokens
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// let factory_service = FactoryService::new(Arc::clone(&client));
    /// let factory_address = "0x...".parse::<Address>().unwrap();
    /// let token_a = "0x...".parse::<Address>().unwrap();
    /// let token_b = "0x...".parse::<Address>().unwrap();
    /// async {
    /// let pair_address = factory_service.create_pair(factory_address, token_a, token_b).await?;
    /// Ok::<(), EvmError>(())
    /// };
    /// ```
    pub async fn create_pair(
        &self,
        factory_address: Address,
        token_a: Address,
        token_b: Address,
    ) -> Result<Address, EvmError> {
        if self.client.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }
        let wallet = self.client.wallet.as_ref().unwrap();
        let signer_middleware = SignerMiddleware::new(self.client.provider.clone(), wallet.clone());
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, Arc::new(signer_middleware));
        let tx = factory.create_pair(token_a, token_b);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to create pair: {}", e)))?;
        let receipt = pending_tx
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to get receipt: {}", e)))?
            .ok_or_else(|| EvmError::TransactionError("Transaction failed".to_string()))?;
        // Get the newly created transaction pair address from the event log
        let pair_created_topic = H256::from_slice(&ethers::utils::keccak256(
            b"PairCreated(address,address,address,uint256)",
        ));
        if let Some(log) = receipt.logs.iter().find(|log| log.topics.len() >= 3) {
            if log.topics[0] == pair_created_topic {
                let pair_address = Address::from_slice(&log.data[12..32]);
                return Ok(pair_address);
            }
        }
        Err(EvmError::TransactionError(
            "Failed to extract pair address from logs".to_string(),
        ))
    }

    /// Gets the total number of pairs in the factory
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// let factory_service = FactoryService::new(Arc::clone(&client));
    /// let factory_address = "0x...".parse::<Address>().unwrap();
    /// async {
    /// let total_pairs = factory_service.all_pairs_length(factory_address).await?;
    /// Ok::<(), EvmError>(())
    /// };
    /// ```
    pub async fn all_pairs_length(&self, factory_address: Address) -> Result<U256, EvmError> {
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, self.client.provider.clone());
        factory
            .all_pairs_length()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pairs length: {}", e)))
    }

    /// Gets the pair address at a specific index
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use std::sync::Arc;
    /// let factory_service = FactoryService::new(Arc::clone(&client));
    /// let factory_address = "0x...".parse::<Address>().unwrap();
    /// let index = U256::from(0);
    /// async {
    /// let pair_address = factory_service.all_pairs(factory_address, index).await?;
    /// Ok::<(), EvmError>(())
    /// };
    /// ```
    pub async fn all_pairs(
        &self,
        factory_address: Address,
        index: U256,
    ) -> Result<Address, EvmError> {
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, self.client.provider.clone());
        factory.all_pairs(index).call().await.map_err(|e| {
            EvmError::ContractError(format!("Failed to get pair at index {}: {}", index, e))
        })
    }

    /// Get the fee receiving address
    pub async fn fee_to(&self, factory_address: Address) -> Result<Address, EvmError> {
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, self.client.provider.clone());
        factory
            .fee_to()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get fee to: {}", e)))
    }

    /// Get the address of the person who set the fee
    pub async fn fee_to_setter(&self, factory_address: Address) -> Result<Address, EvmError> {
        let factory =
            crate::abi::IPancakeFactory::new(factory_address, self.client.provider.clone());
        factory
            .fee_to_setter()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get fee to setter: {}", e)))
    }

    /// Gets multiple pairs in batch
    ///
    /// # Example
    /// ```
    /// # use ethers::types::Address;
    /// # use std::sync::Arc;
    /// # let factory_service = FactoryService::new(Arc::clone(&client));
    /// # let factory_address = "0x...".parse::<Address>().unwrap();
    /// # async {
    /// let pairs = factory_service.get_all_pairs(factory_address, 0, 10).await?;
    /// # Ok::<(), EvmError>(())
    /// # };
    /// ```
    pub async fn get_all_pairs(
        &self,
        factory_address: Address,
        start_index: u64,
        count: u64,
    ) -> Result<Vec<Address>, EvmError> {
        let total_pairs = self.all_pairs_length(factory_address).await?;
        let end_index = std::cmp::min(start_index + count, total_pairs.as_u64());
        let mut pairs = Vec::new();

        for i in start_index..end_index {
            let pair_address = self.all_pairs(factory_address, i.into()).await?;
            pairs.push(pair_address);
        }

        Ok(pairs)
    }

    /// Checks if a pair exists for two tokens
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// let factory_service = FactoryService::new(Arc::clone(&client));
    /// let factory_address = "0x...".parse::<Address>().unwrap();
    /// let token_a = "0x...".parse::<Address>().unwrap();
    /// let token_b = "0x...".parse::<Address>().unwrap();
    /// async {
    /// let exists = factory_service.pair_exists(factory_address, token_a, token_b).await?;
    /// Ok::<(), EvmError>(())
    /// };
    /// ```
    pub async fn pair_exists(
        &self,
        factory_address: Address,
        token_a: Address,
        token_b: Address,
    ) -> Result<bool, EvmError> {
        let pair = self.get_pair(factory_address, token_a, token_b).await?;
        Ok(pair.is_some())
    }
}
