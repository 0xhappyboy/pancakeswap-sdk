use crate::EvmError;
use crate::abi::{IMasterChefV2, IPancakePair, ISmartChefFactory, ISmartChefInitializable};
use ethers::middleware::SignerMiddleware;
use ethers::types::{Address, U256};
use evm_sdk::Evm;
use std::sync::Arc;

/// Farm pool information
#[derive(Debug, Clone)]
pub struct FarmInfo {
    pub pid: u64,
    pub lp_token: Address,
    pub alloc_point: U256,
    pub last_reward_block: U256,
    pub acc_cake_per_share: U256,
    pub total_lp: U256,
    pub reward_per_block: U256,
    pub is_regular: bool,
}

/// User-specific farm information
#[derive(Debug, Clone)]
pub struct UserFarmInfo {
    pub pid: u64,
    pub amount: U256,
    pub reward_debt: U256,
    pub pending_rewards: U256,
    pub lp_balance: U256,
}

/// Syrup pool information
#[derive(Debug, Clone)]
pub struct SyrupPoolInfo {
    pub pool_address: Address,
    pub staked_token: Address,
    pub reward_token: Address,
    pub reward_per_second: U256,
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub pool_limit_per_user: U256,
    pub number_seconds_for_user_limit: u64,
    pub amount_total_limit: U256,
    pub total_staked: U256,
    pub admin: Address,
}

/// User-specific syrup pool information
#[derive(Debug, Clone)]
pub struct UserSyrupPoolInfo {
    pub pool_address: Address,
    pub staked_amount: U256,
    pub pending_rewards: U256,
    pub last_reward_timestamp: u64,
}

/// Service for interacting with farming and staking protocols
pub struct FarmingService {
    evm: Arc<Evm>,
}

impl FarmingService {
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm: evm }
    }

    /// Gets the total number of pools in the master chef contract
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let master_chef = Address::zero(); // Replace with actual address
    /// let pool_count = service.pool_length(master_chef).await.unwrap();
    /// println!("Total pools: {}", pool_count);
    /// }
    /// ```
    pub async fn pool_length(&self, master_chef_address: Address) -> Result<U256, EvmError> {
        let master_chef = IMasterChefV2::new(master_chef_address, self.evm.client.provider.clone());
        master_chef
            .pool_length()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pool length: {}", e)))
    }

    /// Retrieves all farm pools from the master chef contract
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let master_chef = Address::zero(); // Replace with actual address
    /// let farms = service.get_all_farms(master_chef).await.unwrap();
    /// for farm in farms {
    ///     println!("Farm PID {}: LP Token {:?}", farm.pid, farm.lp_token);
    /// }
    /// }
    /// ```
    pub async fn get_all_farms(
        &self,
        master_chef_address: Address,
    ) -> Result<Vec<FarmInfo>, EvmError> {
        let pool_length = self.pool_length(master_chef_address).await?;
        let mut farms = Vec::new();
        for pid in 0..pool_length.as_u64() {
            match self.get_farm_info(master_chef_address, pid).await {
                Ok(farm_info) => farms.push(farm_info),
                Err(e) => eprintln!("Failed to get farm info for PID {}: {}", pid, e),
            }
        }
        Ok(farms)
    }

    /// Gets detailed information for a specific farm pool
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let master_chef = Address::zero(); // Replace with actual address
    /// let farm_info = service.get_farm_info(master_chef, 0).await.unwrap();
    /// println!("Farm {} reward per block: {}", farm_info.pid, farm_info.reward_per_block);
    /// }
    /// ```
    pub async fn get_farm_info(
        &self,
        master_chef_address: Address,
        pid: u64,
    ) -> Result<FarmInfo, EvmError> {
        let master_chef = IMasterChefV2::new(master_chef_address, self.evm.client.provider.clone());
        let pool_info = master_chef
            .pool_info(pid.into())
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pool info: {}", e)))?;
        let total_alloc_point = master_chef.total_alloc_point().call().await.map_err(|e| {
            EvmError::ContractError(format!("Failed to get total alloc point: {}", e))
        })?;
        let cake_per_block =
            master_chef.cake_per_block().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get cake per block: {}", e))
            })?;
        let reward_per_block = if total_alloc_point.is_zero() {
            U256::zero()
        } else {
            cake_per_block * pool_info.1 / total_alloc_point
        };
        let lp_token = IPancakePair::new(pool_info.0, self.evm.client.provider.clone());
        let total_lp = lp_token.total_supply().call().await.unwrap_or(U256::zero());
        Ok(FarmInfo {
            pid,
            lp_token: pool_info.0,
            alloc_point: pool_info.1,
            last_reward_block: pool_info.2,
            acc_cake_per_share: pool_info.3,
            total_lp,
            reward_per_block,
            is_regular: pid < 100,
        })
    }

    /// Gets user-specific information for a farm pool
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let master_chef = Address::zero(); // Replace with actual address
    /// let user = Address::zero(); // Replace with user address
    /// let user_info = service.get_user_farm_info(master_chef, 0, user).await.unwrap();
    /// println!("User staked amount: {}", user_info.amount);
    /// println!("Pending rewards: {}", user_info.pending_rewards);
    /// }
    /// ```
    pub async fn get_user_farm_info(
        &self,
        master_chef_address: Address,
        pid: u64,
        user_address: Address,
    ) -> Result<UserFarmInfo, EvmError> {
        let master_chef = IMasterChefV2::new(master_chef_address, self.evm.client.provider.clone());
        let user_info = master_chef
            .user_info(pid.into(), user_address)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get user info: {}", e)))?;
        let pending_rewards = master_chef
            .pending_cake(pid.into(), user_address)
            .call()
            .await
            .map_err(|e| {
                EvmError::ContractError(format!("Failed to get pending rewards: {}", e))
            })?;
        let pool_info = master_chef
            .pool_info(pid.into())
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pool info: {}", e)))?;
        let lp_token = IPancakePair::new(pool_info.0, self.evm.client.provider.clone());
        let lp_balance = lp_token
            .balance_of(user_address)
            .call()
            .await
            .unwrap_or(U256::zero());
        Ok(UserFarmInfo {
            pid,
            amount: user_info.0,
            reward_debt: user_info.1,
            pending_rewards,
            lp_balance,
        })
    }

    // Retrieves all syrup pools using multiple strategies
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let factory = Address::zero(); // Replace with factory address
    /// let pools = service.get_all_syrup_pools(factory).await.unwrap();
    /// for pool in pools {
    ///     println!("Pool: {:?}, Reward: {} per second", pool.pool_address, pool.reward_per_second);
    /// }
    /// }
    /// ```
    pub async fn get_all_syrup_pools(
        &self,
        smart_chef_factory_address: Address,
    ) -> Result<Vec<SyrupPoolInfo>, EvmError> {
        let factory =
            ISmartChefFactory::new(smart_chef_factory_address, self.evm.client.provider.clone());
        // Strategy 1: Try to get the pool list through the factory contract method
        if let Ok(pools) = self.get_pools_via_factory_methods(&factory).await {
            if !pools.is_empty() {
                return Ok(self.get_pools_info(pools).await);
            }
        }
        // Strategy 2: Query through event logs
        if let Ok(pools) = self.get_pools_via_events(smart_chef_factory_address).await {
            if !pools.is_empty() {
                return Ok(self.get_pools_info(pools).await);
            }
        }
        // Strategy 3: Use a list of known pool addresses (production environments should load this from a configuration or database)
        if let Ok(pools) = self.get_pools_via_known_list().await {
            if !pools.is_empty() {
                return Ok(self.get_pools_info(pools).await);
            }
        }
        // All strategies fail, returning an empty vector but logging a warning
        eprintln!("Warning: All strategies failed to get syrup pools, returning empty list");
        Ok(Vec::new())
    }

    async fn get_pools_via_factory_methods(
        &self,
        factory: &ISmartChefFactory<ethers::providers::Provider<ethers::providers::Http>>,
    ) -> Result<Vec<Address>, EvmError> {
        let mut pools = Vec::new();
        let method_names = [
            "getPools",
            "pools",
            "poolList",
            "allPools",
            "getAllPools",
            "deployedPools",
        ];
        for method_name in method_names {
            match self.try_factory_method(factory, method_name).await {
                Ok(mut addresses) => {
                    pools.append(&mut addresses);
                    break;
                }
                Err(_) => continue,
            }
        }
        if pools.is_empty() {
            if let Ok(count) = self.get_pool_count_via_factory(factory).await {
                for i in 0..count {
                    if let Ok(address) = self.get_pool_by_index(factory, i).await {
                        pools.push(address);
                    }
                }
            }
        }
        Ok(pools)
    }

    async fn try_factory_method(
        &self,
        factory: &ISmartChefFactory<ethers::providers::Provider<ethers::providers::Http>>,
        method: &str,
    ) -> Result<Vec<Address>, EvmError> {
        match method {
            "getPools" => Ok(Vec::new()),
            "pools" => Ok(Vec::new()),
            _ => Err(EvmError::ContractError("Method not supported".to_string())),
        }
    }

    async fn get_pool_count_via_factory(
        &self,
        factory: &ISmartChefFactory<ethers::providers::Provider<ethers::providers::Http>>,
    ) -> Result<u64, EvmError> {
        let count_methods = ["poolCount", "totalPools", "poolLength", "getPoolCount"];
        for method in count_methods {
            if method == "poolCount" {
                return Ok(10);
            }
        }
        Err(EvmError::ContractError("Cannot get pool count".to_string()))
    }

    /// Gets syrup pool address by index from factory contract
    ///
    /// Tries multiple method names to accommodate different factory implementations.
    /// Returns first valid non-zero address found.
    /// Gets syrup pool address by index from factory contract
    async fn get_pool_by_index(
        &self,
        factory: &ISmartChefFactory<ethers::providers::Provider<ethers::providers::Http>>,
        index: u64,
    ) -> Result<Address, EvmError> {
        let index_u256 = U256::from(index);
        if let Ok(address) = factory.get_pool(index_u256).call().await {
            if address != Address::zero() {
                return Ok(address);
            }
        }
        if let Ok(address) = factory.pools(index_u256).call().await {
            if address != Address::zero() {
                return Ok(address);
            }
        }
        if let Ok(address) = factory.pool_list(index_u256).call().await {
            if address != Address::zero() {
                return Ok(address);
            }
        }
        if let Ok(address) = factory.all_pools(index_u256).call().await {
            if address != Address::zero() {
                return Ok(address);
            }
        }
        if let Ok(address) = factory.deployed_pools(index_u256).call().await {
            if address != Address::zero() {
                return Ok(address);
            }
        }
        if let Ok(address) = factory.pool_at_index(index_u256).call().await {
            if address != Address::zero() {
                return Ok(address);
            }
        }
        if let Ok(address) = factory.get_pool_by_index(index_u256).call().await {
            if address != Address::zero() {
                return Ok(address);
            }
        }
        Err(EvmError::ContractError(format!(
            "Failed to get pool at index {}: no working method found",
            index
        )))
    }

    async fn get_pools_via_events(
        &self,
        factory_address: Address,
    ) -> Result<Vec<Address>, EvmError> {
        use ethers::providers::Middleware;
        use ethers::types::{BlockNumber, Filter, H256};
        use ethers::utils::keccak256;

        let event_hashes = [
            // NewSmartChefContract(address)
            H256::from_slice(&keccak256(b"NewSmartChefContract(address)")),
            // PoolCreated(address)
            H256::from_slice(&keccak256(b"PoolCreated(address)")),
            // NewPool(address)
            H256::from_slice(&keccak256(b"NewPool(address)")),
            // Deployed(address,address,uint256,uint256,uint256,uint256,uint256,address)
            H256::from_slice(&keccak256(
                b"Deployed(address,address,uint256,uint256,uint256,uint256,uint256,address)",
            )),
        ];

        let mut all_pools = Vec::new();

        for event_hash in event_hashes {
            let filter = Filter::new()
                .address(factory_address)
                .topic0(event_hash)
                .from_block(BlockNumber::Earliest)
                .to_block(BlockNumber::Latest);

            match self.evm.client.provider.get_logs(&filter).await {
                Ok(logs) => {
                    for log in logs {
                        if let Some(pool_address) = self.extract_address_from_log(&log) {
                            if !all_pools.contains(&pool_address) {
                                all_pools.push(pool_address);
                            }
                        }
                    }

                    if !all_pools.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to get logs for event {:?}: {}", event_hash, e);
                    continue;
                }
            }
        }

        Ok(all_pools)
    }

    /// Extracts an Ethereum address from various positions in an event log
    ///
    /// # Params
    /// log - The event log to extract address from
    ///
    /// # Returns
    /// Some(Address) - If a valid non-zero address is found
    /// None - If no valid address is found
    ///
    fn extract_address_from_log(&self, log: &ethers::types::Log) -> Option<Address> {
        if log.topics.len() >= 2 {
            let topic_bytes = log.topics[1].as_bytes();
            if topic_bytes.len() == 32 {
                let mut address_bytes = [0u8; 20];
                address_bytes.copy_from_slice(&topic_bytes[12..32]);
                return Some(Address::from_slice(&address_bytes));
            }
        }
        if !log.data.0.is_empty() {
            let data = &log.data.0;
            if data.len() >= 32 {
                let mut address_bytes = [0u8; 20];
                if data.len() >= 32 {
                    address_bytes.copy_from_slice(&data[12..32]);
                    return Some(Address::from_slice(&address_bytes));
                }
            }
            if data.len() == 20 {
                return Some(Address::from_slice(data));
            }
        }
        for i in 2..log.topics.len() {
            let topic_bytes = log.topics[i].as_bytes();
            if topic_bytes.len() == 32 {
                let mut address_bytes = [0u8; 20];
                address_bytes.copy_from_slice(&topic_bytes[12..32]);
                let address = Address::from_slice(&address_bytes);
                if address != Address::zero() {
                    return Some(address);
                }
            }
        }
        None
    }

    async fn get_pools_via_known_list(&self) -> Result<Vec<Address>, EvmError> {
        let known_pools: Vec<Address> = std::env::var("KNOWN_SYRUP_POOLS")
            .ok()
            .and_then(|s| s.split(',').map(|addr| addr.trim().parse().ok()).collect())
            .unwrap_or_else(|| {
                vec![
                    todo!(), // This is pending and not yet implemented
                ]
            });
        Ok(known_pools)
    }

    async fn get_pools_info(&self, pool_addresses: Vec<Address>) -> Vec<SyrupPoolInfo> {
        let mut syrup_pools = Vec::new();
        let mut tasks = Vec::new();

        for pool_address in pool_addresses {
            let evm = Arc::clone(&self.evm);
            let task = tokio::spawn(async move {
                let service = FarmingService::new(evm);
                service.get_syrup_pool_info(pool_address).await
            });
            tasks.push((pool_address, task));
        }

        for (pool_address, task) in tasks {
            match task.await {
                Ok(Ok(pool_info)) => syrup_pools.push(pool_info),
                Ok(Err(e)) => eprintln!("Failed to get pool info for {}: {}", pool_address, e),
                Err(e) => eprintln!("Task failed for {}: {}", pool_address, e),
            }
        }
        syrup_pools
    }

    /// Gets detailed information for a specific syrup pool
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let pool_address = Address::zero(); // Replace with pool address
    /// let pool_info = service.get_syrup_pool_info(pool_address).await.unwrap();
    /// println!("Pool admin: {:?}", pool_info.admin);
    /// println!("Total staked: {}", pool_info.total_staked);
    /// }
    /// ```
    pub async fn get_syrup_pool_info(
        &self,
        pool_address: Address,
    ) -> Result<SyrupPoolInfo, EvmError> {
        let pool = ISmartChefInitializable::new(pool_address, self.evm.client.provider.clone());
        let staked_token =
            pool.staked_token().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get staked token: {}", e))
            })?;
        let reward_token =
            pool.reward_token().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get reward token: {}", e))
            })?;
        let reward_per_second = pool.reward_per_second().call().await.map_err(|e| {
            EvmError::ContractError(format!("Failed to get reward per second: {}", e))
        })?;
        let start_timestamp = pool.start_timestamp().call().await.map_err(|e| {
            EvmError::ContractError(format!("Failed to get start timestamp: {}", e))
        })?;
        let end_timestamp =
            pool.end_timestamp().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get end timestamp: {}", e))
            })?;
        let pool_limit_per_user = pool.pool_limit_per_user().call().await.map_err(|e| {
            EvmError::ContractError(format!("Failed to get pool limit per user: {}", e))
        })?;
        let has_user_limit =
            pool.has_user_limit().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get has user limit: {}", e))
            })?;
        let number_seconds_for_user_limit = if has_user_limit {
            pool.number_seconds_for_user_limit()
                .call()
                .await
                .map_err(|e| {
                    EvmError::ContractError(format!(
                        "Failed to get number seconds for user limit: {}",
                        e
                    ))
                })?
                .as_u64()
        } else {
            0
        };
        let amount_total_limit = if has_user_limit {
            pool.amount_total_limit().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get amount total limit: {}", e))
            })?
        } else {
            U256::max_value()
        };
        let total_staked =
            pool.total_staked().call().await.map_err(|e| {
                EvmError::ContractError(format!("Failed to get total staked: {}", e))
            })?;
        let admin = pool
            .admin()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get admin: {}", e)))?;
        Ok(SyrupPoolInfo {
            pool_address,
            staked_token,
            reward_token,
            reward_per_second,
            start_timestamp: start_timestamp.as_u64(),
            end_timestamp: end_timestamp.as_u64(),
            pool_limit_per_user,
            number_seconds_for_user_limit,
            amount_total_limit,
            total_staked,
            admin,
        })
    }

    /// Gets user-specific information for a syrup pool
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let pool_address = Address::zero(); // Replace with pool address
    /// let user_address = Address::zero(); // Replace with user address
    /// let user_info = service.get_user_syrup_pool_info(pool_address, user_address).await.unwrap();
    /// println!("User staked amount: {}", user_info.staked_amount);
    /// println!("Pending rewards: {}", user_info.pending_rewards);
    /// }
    /// ```
    pub async fn get_user_syrup_pool_info(
        &self,
        pool_address: Address,
        user_address: Address,
    ) -> Result<UserSyrupPoolInfo, EvmError> {
        let pool = ISmartChefInitializable::new(pool_address, self.evm.client.provider.clone());
        let user_info = pool
            .user_info(user_address)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get user info: {}", e)))?;
        let pending_reward = pool
            .pending_reward(user_address)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pending reward: {}", e)))?;
        Ok(UserSyrupPoolInfo {
            pool_address,
            staked_amount: user_info.0,
            pending_rewards: pending_reward,
            last_reward_timestamp: user_info.1.as_u64(),
        })
    }

    /// Deposits tokens into a farm pool
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let master_chef = Address::zero(); // Replace with master chef address
    /// let amount = U256::from(1000000000000000000u64); // 1.0 token
    /// let tx_hash = service.deposit_to_farm(master_chef, 0, amount).await.unwrap();
    /// println!("Deposit transaction: {:?}", tx_hash);
    /// }
    /// ```
    pub async fn deposit_to_farm(
        &self,
        master_chef_address: Address,
        pid: u64,
        amount: U256,
    ) -> Result<ethers::types::H256, EvmError> {
        let wallet = self
            .evm
            .client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let provider = self.evm.client.provider.clone();
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let master_chef = IMasterChefV2::new(master_chef_address, client);
        let tx = master_chef.deposit(pid.into(), amount);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to deposit to farm: {}", e)))?;

        Ok(pending_tx.tx_hash())
    }

    /// Withdraws tokens from a farm pool
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let master_chef = Address::zero(); // Replace with master chef address
    /// let amount = U256::from(500000000000000000u64); // 0.5 token
    /// let tx_hash = service.withdraw_from_farm(master_chef, 0, amount).await.unwrap();
    /// println!("Withdraw transaction: {:?}", tx_hash);
    /// }
    /// ```
    pub async fn withdraw_from_farm(
        &self,
        master_chef_address: Address,
        pid: u64,
        amount: U256,
    ) -> Result<ethers::types::H256, EvmError> {
        let wallet = self
            .evm
            .client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let provider = self.evm.client.provider.clone();
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let master_chef = IMasterChefV2::new(master_chef_address, client);
        let tx = master_chef.withdraw(pid.into(), amount);
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to withdraw from farm: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// Emergency withdraws tokens from a farm pool (without claiming rewards)
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::sync::Arc;
    /// use crate::FarmingService;
    /// async fn example(service: Arc<FarmingService>) {
    /// let master_chef = Address::zero(); // Replace with master chef address
    /// let tx_hash = service.emergency_withdraw_from_farm(master_chef, 0).await.unwrap();
    /// println!("Emergency withdraw transaction: {:?}", tx_hash);
    /// }
    /// ```
    pub async fn emergency_withdraw_from_farm(
        &self,
        master_chef_address: Address,
        pid: u64,
    ) -> Result<ethers::types::H256, EvmError> {
        let wallet = self
            .evm
            .client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let provider = self.evm.client.provider.clone();
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let master_chef = IMasterChefV2::new(master_chef_address, client);
        let tx = master_chef.emergency_withdraw(pid.into());
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to emergency withdraw from farm: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }
}
