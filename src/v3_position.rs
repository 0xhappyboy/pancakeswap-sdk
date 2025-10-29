use crate::{
    EvmError,
    abi::{INonfungiblePositionManager, i_nonfungible_position_manager},
};
use ethers::{
    middleware::SignerMiddleware,
    types::{Address, U256},
};
use evm_sdk::Evm;
use std::sync::Arc;

/// Represents a Uniswap V3 position
#[derive(Debug, Clone)]
pub struct V3Position {
    pub token_id: U256,
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: U256,
    pub tokens_owed0: U256,
    pub tokens_owed1: U256,
    pub fee_growth_inside0_last_x128: U256,
    pub fee_growth_inside1_last_x128: U256,
}

/// Service for managing Uniswap V3 positions
pub struct V3PositionService {
    evm: Arc<Evm>,
}

impl V3PositionService {
    /// Creates a new V3PositionService instance
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm: evm }
    }

    /// Retrieves all V3 positions for a given user
    ///
    /// # Params
    /// nft_position_manager - Address of the NonfungiblePositionManager contract
    /// user_address - Address of the user to query positions for
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use crate::{EvmClient, V3PositionService};
    /// #
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let service = V3PositionService::new(client);
    /// let nft_manager = Address::from_str("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")?;
    /// let user = Address::from_str("0x742d35Cc6634C0532925a3b8Dc9F1a37d3Dd5F9A")?;
    /// let positions = service.get_user_positions(nft_manager, user).await?;
    /// println!("Found {} positions", positions.len());
    /// Ok(())
    /// }
    /// ```
    pub async fn get_user_positions(
        &self,
        nft_position_manager: Address,
        user_address: Address,
    ) -> Result<Vec<V3Position>, EvmError> {
        let nft_manager = INonfungiblePositionManager::new(
            nft_position_manager,
            self.evm.client.provider.clone(),
        );
        let balance = nft_manager
            .balance_of(user_address)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get NFT balance: {}", e)))?;
        let mut positions = Vec::new();
        for i in 0..balance.as_u64() {
            let token_id = nft_manager
                .token_of_owner_by_index(user_address, i.into())
                .call()
                .await
                .map_err(|e| EvmError::ContractError(format!("Failed to get token ID: {}", e)))?;
            if let Ok(position) = self.get_position_info(nft_position_manager, token_id).await {
                positions.push(position);
            }
        }
        Ok(positions)
    }

    /// Retrieves detailed information for a specific position
    ///
    /// # Params
    /// nft_position_manager - Address of the NonfungiblePositionManager contract
    /// token_id - The NFT token ID representing the position
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use crate::{EvmClient, V3PositionService};
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let service = V3PositionService::new(client);
    /// let nft_manager = Address::from_str("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")?;
    /// let token_id = U256::from(12345u64);
    /// let position = service.get_position_info(nft_manager, token_id).await?;
    /// println!("Position liquidity: {}", position.liquidity);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_position_info(
        &self,
        nft_position_manager: Address,
        token_id: U256,
    ) -> Result<V3Position, EvmError> {
        let nft_manager = INonfungiblePositionManager::new(
            nft_position_manager,
            self.evm.client.provider.clone(),
        );
        let position = nft_manager
            .positions(token_id)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get position: {}", e)))?;
        Ok(V3Position {
            token_id,
            token0: position.2,
            token1: position.3,
            fee: position.4,
            tick_lower: position.5,
            tick_upper: position.6,
            liquidity: position.7.into(),
            tokens_owed0: position.8,
            tokens_owed1: position.9,
            fee_growth_inside0_last_x128: position.10.into(),
            fee_growth_inside1_last_x128: position.11.into(),
        })
    }

    /// Creates a new V3 position
    ///
    /// # Params
    /// nft_position_manager - Address of the NonfungiblePositionManager contract
    /// token0 - Address of the first token in the pair
    /// token1 - Address of the second token in the pair
    /// fee - The fee tier for the pool (e.g., 3000 for 0.3%)
    /// tick_lower - The lower tick of the position
    /// tick_upper - The upper tick of the position
    /// amount0_desired - The desired amount of token0 to add
    /// amount1_desired - The desired amount of token1 to add
    /// amount0_min - The minimum amount of token0 to add
    /// amount1_min - The minimum amount of token1 to add
    /// recipient - The address that will receive the position NFT
    /// deadline - Unix timestamp after which the transaction will revert
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use crate::{EvmClient, V3PositionService};
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let service = V3PositionService::new(client);
    /// let nft_manager = Address::from_str("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")?;
    /// let token0 = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?; // USDC
    /// let token1 = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?; // WETH
    /// let tx_hash = service.create_position(
    ///     nft_manager,
    ///     token0,
    ///     token1,
    ///     3000, // 0.3% fee
    ///     -887220,
    ///     887220,
    ///     U256::from(1000000u64), // 1 USDC
    ///     U256::from(1000000000000000u64), // 0.001 ETH
    ///     U256::from(900000u64), // min 0.9 USDC
    ///     U256::from(900000000000000u64), // min 0.0009 ETH
    ///     Address::zero(), // recipient
    ///     1698765432, // deadline
    /// ).await?;
    /// println!("Position created with tx: {:?}", tx_hash);
    /// Ok(())
    /// }
    /// ```
    pub async fn create_position(
        &self,
        nft_position_manager: Address,
        token0: Address,
        token1: Address,
        fee: u32,
        tick_lower: i32,
        tick_upper: i32,
        amount0_desired: U256,
        amount1_desired: U256,
        amount0_min: U256,
        amount1_min: U256,
        recipient: Address,
        deadline: u64,
    ) -> Result<ethers::types::H256, EvmError> {
        let wallet = self
            .evm
            .client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let provider = self.evm.client.provider.clone();
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let nft_manager = INonfungiblePositionManager::new(nft_position_manager, client);
        let params = i_nonfungible_position_manager::MintParams {
            token_0: token0,
            token_1: token1,
            fee: fee,
            tick_lower: tick_lower,
            tick_upper: tick_upper,
            amount_0_desired: amount0_desired,
            amount_1_desired: amount1_desired,
            amount_0_min: amount0_min,
            amount_1_min: amount1_min,
            recipient: recipient,
            deadline: deadline.into(),
        };
        let tx = nft_manager.mint(params);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to create position: {}", e)))?;

        Ok(pending_tx.tx_hash())
    }

    /// Increases liquidity for an existing position
    ///
    /// # Params
    /// nft_position_manager - Address of the NonfungiblePositionManager contract
    /// token_id - The NFT token ID representing the position
    /// amount0_desired - The desired amount of token0 to add
    /// amount1_desired - The desired amount of token1 to add
    /// amount0_min - The minimum amount of token0 to add
    /// amount1_min - The minimum amount of token1 to add
    /// deadline - Unix timestamp after which the transaction will revert
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use crate::{EvmClient, V3PositionService};
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let service = V3PositionService::new(client);
    /// let nft_manager = Address::from_str("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")?;
    /// let token_id = U256::from(12345u64);
    /// let tx_hash = service.increase_liquidity(
    ///     nft_manager,
    ///     token_id,
    ///     U256::from(500000u64), // 0.5 USDC
    ///     U256::from(500000000000000u64), // 0.0005 ETH
    ///     U256::from(450000u64), // min 0.45 USDC
    ///     U256::from(450000000000000u64), // min 0.00045 ETH
    ///     1698765432, // deadline
    /// ).await?;
    /// println!("Liquidity increased with tx: {:?}", tx_hash);
    /// Ok(())
    /// }
    /// ```
    pub async fn increase_liquidity(
        &self,
        nft_position_manager: Address,
        token_id: U256,
        amount0_desired: U256,
        amount1_desired: U256,
        amount0_min: U256,
        amount1_min: U256,
        deadline: u64,
    ) -> Result<ethers::types::H256, EvmError> {
        let wallet = self
            .evm
            .client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let provider = self.evm.client.provider.clone();
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let nft_manager = INonfungiblePositionManager::new(nft_position_manager, client);
        let params = i_nonfungible_position_manager::IncreaseLiquidityParams {
            token_id,
            amount_0_desired: amount0_desired,
            amount_1_desired: amount1_desired,
            amount_0_min: amount0_min,
            amount_1_min: amount1_min,
            deadline: deadline.into(),
        };
        let tx = nft_manager.increase_liquidity(params);
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to increase liquidity: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// Decreases liquidity for an existing position
    ///
    /// # Params
    /// nft_position_manager - Address of the NonfungiblePositionManager contract
    /// token_id - The NFT token ID representing the position
    /// liquidity - The amount of liquidity to remove
    /// amount0_min - The minimum amount of token0 to receive
    /// amount1_min - The minimum amount of token1 to receive
    /// deadline - Unix timestamp after which the transaction will revert
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use crate::{EvmClient, V3PositionService};
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let service = V3PositionService::new(client);
    /// let nft_manager = Address::from_str("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")?;
    /// let token_id = U256::from(12345u64);
    /// let tx_hash = service.decrease_liquidity(
    ///     nft_manager,
    ///     token_id,
    ///     U256::from(1000000u64), // Remove 1M liquidity
    ///     U256::from(900000u64), // min 0.9 USDC
    ///     U256::from(900000000000000u64), // min 0.0009 ETH
    ///     1698765432, // deadline
    /// ).await?;
    /// println!("Liquidity decreased with tx: {:?}", tx_hash);
    /// Ok(())
    /// }
    /// ```
    pub async fn decrease_liquidity(
        &self,
        nft_position_manager: Address,
        token_id: U256,
        liquidity: U256,
        amount0_min: U256,
        amount1_min: U256,
        deadline: u64,
    ) -> Result<ethers::types::H256, EvmError> {
        let wallet = self
            .evm
            .client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let provider = self.evm.client.provider.clone();
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let nft_manager = INonfungiblePositionManager::new(nft_position_manager, client);
        let params = i_nonfungible_position_manager::DecreaseLiquidityParams {
            token_id: token_id,
            liquidity: liquidity.as_u128(),
            amount_0_min: amount0_min,
            amount_1_min: amount1_min,
            deadline: deadline.into(),
        };
        let tx = nft_manager.decrease_liquidity(params);
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to decrease liquidity: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// Collects accumulated fees from a position
    ///
    /// # Params
    /// nft_position_manager - Address of the NonfungiblePositionManager contract
    /// token_id - The NFT token ID representing the position
    /// recipient - The address that will receive the collected fees
    /// amount0_max - The maximum amount of token0 to collect
    /// amount1_max - The maximum amount of token1 to collect
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    /// use std::sync::Arc;
    /// use crate::{EvmClient, V3PositionService};
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let service = V3PositionService::new(client);
    /// let nft_manager = Address::from_str("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")?;
    /// let token_id = U256::from(12345u64);
    /// let recipient = Address::from_str("0x742d35Cc6634C0532925a3b8Dc9F1a37d3Dd5F9A")?;
    /// let tx_hash = service.collect_fees(
    ///     nft_manager,
    ///     token_id,
    ///     recipient,
    ///     U256::max_value(), // Collect all available token0
    ///     U256::max_value(), // Collect all available token1
    /// ).await?;
    /// println!("Fees collected with tx: {:?}", tx_hash);
    /// Ok(())
    /// }
    /// ```
    pub async fn collect_fees(
        &self,
        nft_position_manager: Address,
        token_id: U256,
        recipient: Address,
        amount0_max: U256,
        amount1_max: U256,
    ) -> Result<ethers::types::H256, EvmError> {
        let wallet = self
            .evm
            .client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let provider = self.evm.client.provider.clone();
        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let nft_manager = INonfungiblePositionManager::new(nft_position_manager, client);
        let params = i_nonfungible_position_manager::CollectParams {
            token_id,
            recipient,
            amount_0_max: amount0_max.as_u128(),
            amount_1_max: amount1_max.as_u128(),
        };
        let tx = nft_manager.collect(params);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to collect fees: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }
}
