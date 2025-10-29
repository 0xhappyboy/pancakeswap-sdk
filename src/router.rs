use crate::{
    EvmError,
    abi::{IPancakeRouter02, ISwapRouter},
};
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{Signer, Wallet},
    types::{Address, U256},
};
use evm_sdk::Evm;
use std::sync::Arc;

type SignerClient =
    SignerMiddleware<Arc<Provider<Http>>, Wallet<ethers::core::k256::ecdsa::SigningKey>>;

/// Router service for interacting with PancakeSwap V2 and V3 routers
pub struct RouterService {
    evm: Arc<Evm>,
}

impl RouterService {
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm: evm }
    }

    /// Get V2 router contract instance for read-only operations
    pub fn v2_router(&self, router_address: Address) -> IPancakeRouter02<Provider<Http>> {
        IPancakeRouter02::new(router_address, self.evm.client.provider.clone())
    }

    /// Get V3 router contract instance for read-only operations
    pub fn v3_router(&self, router_address: Address) -> ISwapRouter<Provider<Http>> {
        ISwapRouter::new(router_address, self.evm.client.provider.clone())
    }

    /// Get V2 router contract instance with signer for transaction operations
    pub fn v2_router_signer(
        &self,
        router_address: Address,
    ) -> Result<IPancakeRouter02<SignerClient>, EvmError> {
        let wallet = self
            .evm
            .client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let signer_middleware =
            SignerMiddleware::new(self.evm.client.provider.clone(), wallet.clone());
        Ok(IPancakeRouter02::new(
            router_address,
            Arc::new(signer_middleware),
        ))
    }

    /// Get V3 router contract instance with signer for transaction operations
    pub fn v3_router_signer(
        &self,
        router_address: Address,
    ) -> Result<ISwapRouter<SignerClient>, EvmError> {
        let wallet = self
            .evm.client
            .wallet
            .as_ref()
            .ok_or_else(|| EvmError::WalletError("No wallet configured".to_string()))?;
        let signer_middleware = SignerMiddleware::new(self.evm.client.provider.clone(), wallet.clone());
        Ok(ISwapRouter::new(
            router_address,
            Arc::new(signer_middleware),
        ))
    }

    /// Swap exact tokens for tokens supporting fee on transfer tokens
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    ///
    /// async fn example(router_service: RouterService) -> Result<(), EvmError> {
    /// let router_address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap();
    /// let amount_in = U256::from(1000000000000000000u64); // 1 token
    /// let amount_out_min = U256::from(500000000000000000u64); // 0.5 token
    /// let path = vec![
    ///     Address::from_str("0xTokenAAddress").unwrap(),
    ///     Address::from_str("0xTokenBAddress").unwrap(),
    /// ];
    /// let deadline = 1698765432; // Unix timestamp
    ///
    /// let tx_hash = router_service
    ///     .swap_exact_tokens_for_tokens_supporting_fee_on_transfer_tokens(
    ///         router_address,
    ///         amount_in,
    ///         amount_out_min,
    ///         path,
    ///         deadline,
    ///     )
    ///     .await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn swap_exact_tokens_for_tokens_supporting_fee_on_transfer_tokens(
        &self,
        router_address: Address,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        deadline: u64,
    ) -> Result<ethers::types::H256, EvmError> {
        let router = self.v2_router_signer(router_address)?;
        let wallet_address = self.evm.client.wallet.as_ref().unwrap().address();

        let tx = router.swap_exact_tokens_for_tokens_supporting_fee_on_transfer_tokens(
            amount_in,
            amount_out_min,
            path,
            wallet_address,
            deadline.into(),
        );

        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to swap tokens with fee on transfer: {}", e))
        })?;

        Ok(pending_tx.tx_hash())
    }

    /// Swap exact ETH for tokens supporting fee on transfer tokens
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    ///
    /// async fn example(router_service: RouterService) -> Result<(), EvmError> {
    /// let router_address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap();
    /// let amount_out_min = U256::from(1000000000000000000u64); // 1 token
    /// let path = vec![
    ///     Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c").unwrap(), // WBNB
    ///     Address::from_str("0xTokenAddress").unwrap(),
    /// ];
    /// let value = U256::from(100000000000000000u64); // 0.1 BNB
    /// let deadline = 1698765432;
    ///
    /// let tx_hash = router_service
    ///     .swap_exact_eth_for_tokens_supporting_fee_on_transfer_tokens(
    ///         router_address,
    ///         amount_out_min,
    ///         path,
    ///         value,
    ///         deadline,
    ///     )
    ///     .await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn swap_exact_eth_for_tokens_supporting_fee_on_transfer_tokens(
        &self,
        router_address: Address,
        amount_out_min: U256,
        path: Vec<Address>,
        value: U256,
        deadline: u64,
    ) -> Result<ethers::types::H256, EvmError> {
        let router = self.v2_router_signer(router_address)?;
        let wallet_address = self.evm.client.wallet.as_ref().unwrap().address();

        let tx = router
            .swap_exact_eth_for_tokens_supporting_fee_on_transfer_tokens(
                amount_out_min,
                path,
                wallet_address,
                deadline.into(),
            )
            .value(value);

        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!(
                "Failed to swap BNB for tokens with fee on transfer: {}",
                e
            ))
        })?;

        Ok(pending_tx.tx_hash())
    }

    /// Swap exact tokens for ETH supporting fee on transfer tokens
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    ///
    /// async fn example(router_service: RouterService) -> Result<(), EvmError> {
    /// let router_address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap();
    /// let amount_in = U256::from(1000000000000000000u64); // 1 token
    /// let amount_out_min = U256::from(500000000000000000u64); // 0.5 BNB
    /// let path = vec![
    ///     Address::from_str("0xTokenAddress").unwrap(),
    ///     Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c").unwrap(), // WBNB
    /// ];
    /// let deadline = 1698765432;
    ///
    /// let tx_hash = router_service
    ///     .swap_exact_tokens_for_eth_supporting_fee_on_transfer_tokens(
    ///         router_address,
    ///         amount_in,
    ///         amount_out_min,
    ///         path,
    ///         deadline,
    ///     )
    ///     .await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn swap_exact_tokens_for_eth_supporting_fee_on_transfer_tokens(
        &self,
        router_address: Address,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        deadline: u64,
    ) -> Result<ethers::types::H256, EvmError> {
        let router = self.v2_router_signer(router_address)?;
        let wallet_address = self.evm.client.wallet.as_ref().unwrap().address();

        let tx = router.swap_exact_tokens_for_eth_supporting_fee_on_transfer_tokens(
            amount_in,
            amount_out_min,
            path,
            wallet_address,
            deadline.into(),
        );

        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!(
                "Failed to swap tokens for BNB with fee on transfer: {}",
                e
            ))
        })?;

        Ok(pending_tx.tx_hash())
    }

    /// Get factory address from router
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    ///
    /// async fn example(router_service: RouterService) -> Result<(), EvmError> {
    /// let router_address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap();
    /// let factory_address = router_service.factory(router_address).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_factory_from_router(
        &self,
        router_address: Address,
    ) -> Result<Address, EvmError> {
        let router = self.v2_router(router_address);
        router
            .factory()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get factory: {}", e)))
    }

    /// Get WETH address from router
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    ///
    /// async fn example(router_service: RouterService) -> Result<(), EvmError> {
    /// let router_address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap();
    /// let weth_address = router_service.weth(router_address).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_weth_address_from_router(
        &self,
        router_address: Address,
    ) -> Result<Address, EvmError> {
        let router = self.v2_router(router_address);
        router
            .weth()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get WETH: {}", e)))
    }

    /// Get quote for token swap
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    ///
    /// async fn example(router_service: RouterService) -> Result<(), EvmError> {
    /// let router_address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap();
    /// let amount_a = U256::from(1000000000000000000u64); // 1 token
    /// let reserve_a = U256::from(1000000000000000000000u64); // 1000 tokens
    /// let reserve_b = U256::from(500000000000000000000u64); // 500 tokens
    ///
    /// let quote = router_service
    ///     .quote(router_address, amount_a, reserve_a, reserve_b)
    ///     .await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn quote(
        &self,
        router_address: Address,
        amount_a: U256,
        reserve_a: U256,
        reserve_b: U256,
    ) -> Result<U256, EvmError> {
        let router = self.v2_router(router_address);
        router
            .quote(amount_a, reserve_a, reserve_b)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get quote: {}", e)))
    }

    /// Get amount out for a given amount in
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    ///
    /// async fn example(router_service: RouterService) -> Result<(), EvmError> {
    /// let router_address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap();
    /// let amount_in = U256::from(1000000000000000000u64); // 1 token
    /// let reserve_in = U256::from(1000000000000000000000u64); // 1000 tokens
    /// let reserve_out = U256::from(500000000000000000000u64); // 500 tokens
    ///
    /// let amount_out = router_service
    ///     .get_amount_out(router_address, amount_in, reserve_in, reserve_out)
    ///     .await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_amount_out(
        &self,
        router_address: Address,
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, EvmError> {
        let router = self.v2_router(router_address);
        router
            .get_amount_out(amount_in, reserve_in, reserve_out)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get amount out: {}", e)))
    }

    /// Get amount in for a desired amount out
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    ///
    /// async fn example(router_service: RouterService) -> Result<(), EvmError> {
    /// let router_address = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E").unwrap();
    /// let amount_out = U256::from(1000000000000000000u64); // 1 token
    /// let reserve_in = U256::from(1000000000000000000000u64); // 1000 tokens
    /// let reserve_out = U256::from(500000000000000000000u64); // 500 tokens
    ///
    /// let amount_in = router_service
    ///     .get_amount_in(router_address, amount_out, reserve_in, reserve_out)
    ///     .await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_amount_in(
        &self,
        router_address: Address,
        amount_out: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, EvmError> {
        let router = self.v2_router(router_address);
        router
            .get_amount_in(amount_out, reserve_in, reserve_out)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get amount in: {}", e)))
    }
}
