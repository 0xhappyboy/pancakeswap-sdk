use crate::{
    EvmClient, EvmError,
    abi::{IERC20, IMulticall, IPancakePair, IPancakeRouter02, i_multicall},
};
use ethers::{
    abi::AbiDecode,
    types::{Address, U256},
};
use std::collections::HashMap;
use std::sync::Arc;

/// Represents the result of a multicall operation
#[derive(Debug, Clone)]
pub struct MulticallResult {
    pub success: bool,
    pub data: Vec<u8>,
    pub gas_used: U256,
}

/// Service for executing multiple Ethereum calls in a single transaction
pub struct MulticallService {
    client: Arc<EvmClient>,
}

impl MulticallService {
    /// Creates a new MulticallService instance
    pub fn new(client: Arc<EvmClient>) -> Self {
        Self { client }
    }

    /// Executes a batch of calls using the multicall contract
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use multicall::MulticallService;
    ///
    /// async fn example(service: MulticallService, multicall_addr: Address) -> Result<(), Box<dyn std::error::Error>> {
    /// let calls = vec![
    ///     Call::new(token_address, balance_of_calldata),
    ///     Call::new(pair_address, get_reserves_calldata),
    /// ];
    /// let results = service.aggregate(multicall_addr, calls).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn aggregate(
        &self,
        multicall_address: Address,
        calls: Vec<Call>,
    ) -> Result<Vec<MulticallResult>, EvmError> {
        let multicall = IMulticall::new(multicall_address, self.client.provider.clone());
        let call_data: Vec<i_multicall::Call> = calls
            .into_iter()
            .map(|call| i_multicall::Call {
                target: call.target,
                call_data: call.data.into(),
            })
            .collect();
        let (block_number, return_data) = multicall
            .aggregate(call_data)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Multicall failed: {}", e)))?;
        Ok(return_data
            .into_iter()
            .map(|data| MulticallResult {
                success: true,
                data: data.to_vec(),
                gas_used: U256::zero(),
            })
            .collect())
    }

    /// Batch fetches token balances for multiple tokens for a single user
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use std::collections::HashMap;
    /// use multicall::MulticallService;
    ///
    /// async fn example(service: MulticallService, multicall_addr: Address, user: Address) -> Result<(), Box<dyn std::error::Error>> {
    /// let tokens = vec![token1, token2, token3];
    /// let balances: HashMap<Address, U256> = service.get_token_balances(multicall_addr, tokens, user).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_token_balances(
        &self,
        multicall_address: Address,
        token_addresses: Vec<Address>,
        user_address: Address,
    ) -> Result<HashMap<Address, U256>, EvmError> {
        let mut calls = Vec::new();
        for token_address in &token_addresses {
            let erc20 = IERC20::new(*token_address, self.client.provider.clone());
            let call_data = erc20.balance_of(user_address).calldata().ok_or_else(|| {
                EvmError::ContractError("Failed to encode balanceOf call".to_string())
            })?;
            calls.push(Call {
                target: *token_address,
                data: call_data.to_vec(),
            });
        }
        let results = self.aggregate(multicall_address, calls).await?;
        let mut balances = HashMap::new();
        for (i, result) in results.into_iter().enumerate() {
            if result.success && !result.data.is_empty() {
                match U256::decode(&result.data) {
                    Ok(balance) => {
                        balances.insert(token_addresses[i], balance);
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to decode balance for token {}: {}",
                            token_addresses[i], e
                        );
                    }
                }
            }
        }
        Ok(balances)
    }

    /// Batch fetches reserves for multiple liquidity pairs
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use std::collections::HashMap;
    /// use multicall::MulticallService;
    ///
    /// async fn example(service: MulticallService, multicall_addr: Address) -> Result<(), Box<dyn std::error::Error>> {
    /// let pairs = vec![pair1, pair2, pair3];
    /// let reserves: HashMap<Address, (U256, U256, u32)> = service.get_reserves_batch(multicall_addr, pairs).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_reserves_batch(
        &self,
        multicall_address: Address,
        pair_addresses: Vec<Address>,
    ) -> Result<HashMap<Address, (U256, U256, u32)>, EvmError> {
        let mut calls = Vec::new();
        for pair_address in &pair_addresses {
            let pair = IPancakePair::new(*pair_address, self.client.provider.clone());
            let call_data = pair.get_reserves().calldata().ok_or_else(|| {
                EvmError::ContractError("Failed to encode getReserves call".to_string())
            })?;
            calls.push(Call {
                target: *pair_address,
                data: call_data.to_vec(),
            });
        }
        let results = self.aggregate(multicall_address, calls).await?;
        let mut reserves = HashMap::new();
        for (i, result) in results.into_iter().enumerate() {
            if result.success && result.data.len() >= 96 {
                let reserve0 = U256::from_big_endian(&result.data[0..32]);
                let reserve1 = U256::from_big_endian(&result.data[32..64]);
                let block_timestamp_last =
                    u32::from_be_bytes(result.data[64..68].try_into().unwrap());
                reserves.insert(
                    pair_addresses[i],
                    (reserve0, reserve1, block_timestamp_last),
                );
            }
        }
        Ok(reserves)
    }

    /// Batch fetches prices for multiple token pairs using a router
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use std::collections::HashMap;
    /// use multicall::MulticallService;
    ///
    /// async fn example(service: MulticallService, multicall_addr: Address, router_addr: Address) -> Result<(), Box<dyn std::error::Error>> {
    /// let token_pairs = vec![(token_in1, token_out1), (token_in2, token_out2)];
    /// let amount_in = U256::from(10).pow(18); // 1 token
    /// let prices: HashMap<(Address, Address), U256> = service.get_prices_batch(multicall_addr, router_addr, token_pairs, amount_in).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_prices_batch(
        &self,
        multicall_address: Address,
        router_address: Address,
        token_pairs: Vec<(Address, Address)>,
        amount_in: U256,
    ) -> Result<HashMap<(Address, Address), U256>, EvmError> {
        let mut calls = Vec::new();
        for (token_in, token_out) in &token_pairs {
            let router = IPancakeRouter02::new(router_address, self.client.provider.clone());
            let path = vec![*token_in, *token_out];
            let call_data = router
                .get_amounts_out(amount_in, path.clone())
                .calldata()
                .ok_or_else(|| {
                    EvmError::ContractError("Failed to encode getAmountsOut call".to_string())
                })?;
            calls.push(Call {
                target: router_address,
                data: call_data.to_vec(),
            });
        }
        let results = self.aggregate(multicall_address, calls).await?;
        let mut prices = HashMap::new();
        for (i, result) in results.into_iter().enumerate() {
            if result.success && result.data.len() >= 64 {
                if result.data.len() >= 96 {
                    let amount_out = U256::from_big_endian(&result.data[64..96]);
                    prices.insert(token_pairs[i].clone(), amount_out);
                }
            }
        }
        Ok(prices)
    }

    /// Batch fetches balances for multiple tokens and multiple users
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use std::collections::HashMap;
    /// use multicall::MulticallService;
    ///
    /// async fn example(service: MulticallService, multicall_addr: Address) -> Result<(), Box<dyn std::error::Error>> {
    /// let tokens = vec![token1, token2];
    /// let users = vec![user1, user2, user3];
    /// let balances: HashMap<(Address, Address), U256> = service.get_multiple_token_balances(multicall_addr, tokens, users).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_multiple_token_balances(
        &self,
        multicall_address: Address,
        token_addresses: Vec<Address>,
        user_addresses: Vec<Address>,
    ) -> Result<HashMap<(Address, Address), U256>, EvmError> {
        let mut calls = Vec::new();
        for token_address in &token_addresses {
            for user_address in &user_addresses {
                let erc20 = IERC20::new(*token_address, self.client.provider.clone());
                let call_data = erc20.balance_of(*user_address).calldata().ok_or_else(|| {
                    EvmError::ContractError("Failed to encode balanceOf call".to_string())
                })?;
                calls.push(Call {
                    target: *token_address,
                    data: call_data.to_vec(),
                });
            }
        }
        let results = self.aggregate(multicall_address, calls).await?;
        let mut balances = HashMap::new();
        let mut call_index = 0;
        for token_address in &token_addresses {
            for user_address in &user_addresses {
                if let Some(result) = results.get(call_index) {
                    if result.success && !result.data.is_empty() {
                        match U256::decode(&result.data) {
                            Ok(balance) => {
                                balances.insert((*token_address, *user_address), balance);
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to decode balance for token {} user {}: {}",
                                    token_address, user_address, e
                                );
                            }
                        }
                    }
                }
                call_index += 1;
            }
        }
        Ok(balances)
    }
}

#[derive(Debug, Clone)]
pub struct Call {
    pub target: Address,
    pub data: Vec<u8>,
}

impl Call {
    pub fn new(target: Address, data: Vec<u8>) -> Self {
        Self { target, data }
    }
}
