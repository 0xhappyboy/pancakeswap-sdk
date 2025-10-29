/// This module is the pancakeswap service entry module.
pub mod abi;
pub mod analytics;
pub mod events;
pub mod factory;
pub mod farm;
pub mod global;
pub mod limit_order;
pub mod liquidity;
pub mod multicall;
pub mod price;
pub mod router;
pub mod tool;
pub mod types;
pub mod v3_position;

use ethers::{
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, U256},
};
use evm_client::EvmType;
use evm_sdk::Evm;
use std::sync::Arc;

use crate::{
    abi::IQuoter,
    analytics::AnalyticsService,
    factory::FactoryService,
    global::{
        BASE_QUOTER, BASE_ROUTER_V3, BSC_QUOTER, BSC_ROUTER_V2, BSC_ROUTER_V3, ETHEREUM_QUOTER,
        ETHEREUM_ROUTER_V2, ETHEREUM_ROUTER_V3,
    },
    liquidity::LiquidityService,
    price::PriceService,
    router::RouterService,
    types::PriceInfo,
};
use evm_sdk::types::EvmError;
/// PancakeSwap Service for interacting with PancakeSwap protocols
pub struct PancakeSwapService {
    evm: Arc<Evm>,
    router: Arc<RouterService>,
    factory: Arc<FactoryService>,
    liquidity: Arc<LiquidityService>,
    price: Arc<PriceService>,
    analytics: Arc<AnalyticsService>,
}

impl PancakeSwapService {
    /// Create a new PancakeSwap service instance
    pub fn new(evm: Arc<Evm>) -> Self {
        Self {
            evm: evm.clone(),
            router: Arc::new(RouterService::new(evm.clone())),
            factory: Arc::new(FactoryService::new(evm.clone())),
            liquidity: Arc::new(LiquidityService::new(evm.clone())),
            price: Arc::new(PriceService::new(evm.clone())),
            analytics: Arc::new(AnalyticsService::new(evm.clone())),
        }
    }

    /// Get amounts out for a swap (V2)
    ///
    /// # Example
    /// ```
    /// use pancake_swap_sdk::{PancakeSwapService, EvmClient, EvmType};
    /// use ethers::types::{Address, U256};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(),()> {
    ///     let client = EvmClient::new(EvmType::Bsc).await?;
    ///     let service = PancakeSwapService::new(std::sync::Arc::new(client));
    ///     
    ///     let amount_in = U256::from(1000000000000000000u64); // 1 token
    ///     let path = vec![
    ///         "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse()?, // WBNB
    ///         "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56".parse()?, // BUSD
    ///     ];
    ///     
    ///     let amounts = service.get_amounts_out_v2(amount_in, path).await?;
    ///     println!("Output amounts: {:?}", amounts);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_amounts_out_v2(
        &self,
        amount_in: U256,
        path: Vec<Address>,
    ) -> Result<Vec<U256>, EvmError> {
        let router_address =
            PancakeSwapConfig::v2_router_address(self.evm.client.evm_type.unwrap())?;
        let router = self.router.v2_router(router_address);
        router
            .get_amounts_out(amount_in, path)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get amounts out: {}", e)))
    }

    /// Get amounts in for a swap (V2)
    pub async fn get_amounts_in_v2(
        &self,
        amount_out: U256,
        path: Vec<Address>,
    ) -> Result<Vec<U256>, EvmError> {
        let router_address =
            PancakeSwapConfig::v2_router_address(self.evm.client.evm_type.unwrap())?;
        let router = self.router.v2_router(router_address);
        router
            .get_amounts_in(amount_out, path)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get amounts in: {}", e)))
    }

    /// execute V2 swap
    ///
    /// # Example
    /// ```
    /// use pancake_swap_sdk::{PancakeSwapService, EvmClient, EvmType};
    /// use ethers::types::{Address, U256};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(),()> {
    ///     let private_key = "your_private_key_here";
    ///     let client = EvmClient::with_wallet(EvmType::Bsc, private_key).await?;
    ///     let service = PancakeSwapService::new(std::sync::Arc::new(client));
    ///     
    ///     let token_in: Address = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse()?; // WBNB
    ///     let token_out: Address = "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56".parse()?; // BUSD
    ///     let amount_in = U256::from(1000000000000000000u64); // 1 BNB
    ///     let slippage_percent = 1.0; // 1% slippage
    ///     
    ///     let tx_hash = service.swap_v2(token_in, token_out, amount_in, slippage_percent).await?;
    ///     println!("Transaction hash: {:?}", tx_hash);
    ///     Ok(())
    /// }
    /// ```
    pub async fn swap_v2(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        slippage_percent: f64,
    ) -> Result<ethers::types::H256, EvmError> {
        if self.evm.client.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }

        let router_address =
            PancakeSwapConfig::v2_router_address(self.evm.client.evm_type.unwrap())?;
        let deadline = crate::tool::time_utils::calculate_deadline(30); // 30 minutes

        // Get expected output
        let amounts = self
            .get_amounts_out_v2(amount_in, vec![token_in, token_out])
            .await?;
        let expected_out = amounts
            .last()
            .ok_or_else(|| EvmError::CalculationError("Invalid path".to_string()))?;

        // Calculate minimum output with slippage
        let amount_out_min = self.calculate_amount_with_slippage(*expected_out, slippage_percent);
        let wallet_address = self.evm.client.wallet.as_ref().unwrap().address();

        let router = self.router.v2_router(router_address);
        let tx = router.swap_exact_tokens_for_tokens(
            amount_in,
            amount_out_min,
            vec![token_in, token_out],
            wallet_address,
            deadline.into(),
        );

        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to swap tokens: {}", e)))?;

        Ok(pending_tx.tx_hash())
    }

    /// Execute V3 swap
    ///
    /// # Example
    /// ```
    /// use pancake_swap_sdk::{PancakeSwapService, EvmClient, EvmType};
    /// use ethers::types::{Address, U256};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(),()> {
    ///     let private_key = "your_private_key_here";
    ///     let client = EvmClient::with_wallet(EvmType::Bsc, private_key).await?;
    ///     let service = PancakeSwapService::new(std::sync::Arc::new(client));
    ///     
    ///     let token_in: Address = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse()?; // WBNB
    ///     let token_out: Address = "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56".parse()?; // BUSD
    ///     let amount_in = U256::from(1000000000000000000u64); // 1 BNB
    ///     let slippage_percent = 1.0; // 1% slippage
    ///     let fee_tier = Some(500); // 0.05% fee
    ///     
    ///     let tx_hash = service.swap_v3(token_in, token_out, amount_in, slippage_percent, fee_tier).await?;
    ///     println!("Transaction hash: {:?}", tx_hash);
    ///     Ok(())
    /// }
    /// ```
    pub async fn swap_v3(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        slippage_percent: f64,
        fee_tier: Option<u32>,
    ) -> Result<ethers::types::H256, EvmError> {
        if self.evm.client.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }

        let router_address =
            PancakeSwapConfig::v3_router_address(self.evm.client.evm_type.unwrap())?;
        let deadline = crate::tool::time_utils::calculate_deadline(30);

        let fee = fee_tier.unwrap_or_else(|| self.get_default_fee_tier(token_in, token_out));
        let expected_out = self
            .simulate_v3_swap(token_in, token_out, fee, amount_in)
            .await?;
        let amount_out_min = self.calculate_amount_with_slippage(expected_out, slippage_percent);
        let wallet_address = self.evm.client.wallet.as_ref().unwrap().address();

        let router = self.router.v3_router_signer(router_address)?;

        // 使用单独的参数调用 exactInputSingle
        let tx = router.exact_input_single(
            token_in,
            token_out,
            fee,
            wallet_address,
            deadline.into(),
            amount_in,
            amount_out_min,
            U256::zero(),
        );

        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to execute V3 swap: {}", e)))?;

        Ok(pending_tx.tx_hash())
    }

    /// Auto swap - find best price between V2 and V3 and execute
    ///
    /// # Example
    /// ```
    /// use pancake_swap_sdk::{PancakeSwapService, EvmClient, EvmType};
    /// use ethers::types::{Address, U256};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(),()> {
    ///     let private_key = "your_private_key_here";
    ///     let client = EvmClient::with_wallet(EvmType::Bsc, private_key).await?;
    ///     let service = PancakeSwapService::new(std::sync::Arc::new(client));
    ///     
    ///     let token_in: Address = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse()?; // WBNB
    ///     let token_out: Address = "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56".parse()?; // BUSD
    ///     let amount_in = U256::from(1000000000000000000u64); // 1 BNB
    ///     let slippage_percent = 1.0; // 1% slippage
    ///     
    ///     let result = service.auto_swap(token_in, token_out, amount_in, slippage_percent).await?;
    ///     println!("Auto swap result: {:?}", result);
    ///     Ok(())
    /// }
    /// ```
    pub async fn auto_swap(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        slippage_percent: f64,
    ) -> Result<crate::types::AutoSwapResult, EvmError> {
        // Get best price comparison
        let price_comparison = self.get_best_price(token_in, token_out, amount_in).await?;

        let price_comparison_clone = price_comparison.clone();

        let (selected_version, amount_out_min, tx_hash) = match price_comparison.best {
            crate::types::PriceSource::V2 => {
                let v2_info = price_comparison.v2.ok_or_else(|| {
                    EvmError::CalculationError("V2 price not available".to_string())
                })?;
                let amount_out_min =
                    self.calculate_amount_with_slippage(v2_info.amount_out, slippage_percent);
                let tx_hash = self
                    .swap_v2(token_in, token_out, amount_in, slippage_percent)
                    .await?;
                (crate::types::PoolVersion::V2, amount_out_min, tx_hash)
            }
            crate::types::PriceSource::V3 => {
                let v3_info = price_comparison.v3.ok_or_else(|| {
                    EvmError::CalculationError("V3 price not available".to_string())
                })?;
                let amount_out_min =
                    self.calculate_amount_with_slippage(v3_info.amount_out, slippage_percent);
                let fee = self.get_default_fee_tier(token_in, token_out);
                let tx_hash = self
                    .swap_v3(token_in, token_out, amount_in, slippage_percent, Some(fee))
                    .await?;
                (crate::types::PoolVersion::V3, amount_out_min, tx_hash)
            }
        };

        Ok(crate::types::AutoSwapResult {
            tx_hash,
            version: selected_version,
            expected_amount_out: amount_out_min,
            price_comparison: price_comparison_clone,
        })
    }

    /// Get best price comparison between V2 and V3
    pub async fn get_best_price(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<crate::types::PriceComparison, EvmError> {
        let v2_price = self.get_v2_price(token_in, token_out, amount_in).await;
        let v3_price = self.get_v3_price(token_in, token_out, amount_in).await;
        let best_price = match (&v2_price, &v3_price) {
            (Ok(v2), Ok(v3)) => {
                if v2.amount_out > v3.amount_out {
                    crate::types::PriceSource::V2
                } else {
                    crate::types::PriceSource::V3
                }
            }
            (Ok(_), Err(_)) => crate::types::PriceSource::V2,
            (Err(_), Ok(_)) => crate::types::PriceSource::V3,
            _ => return Err(EvmError::CalculationError("No price available".to_string())),
        };
        Ok(crate::types::PriceComparison {
            v2: v2_price.ok(),
            v3: v3_price.ok(),
            best: best_price,
        })
    }

    /// Swap exact tokens for tokens (V2)
    pub async fn swap_exact_tokens_for_tokens(
        &self,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        deadline: u64,
    ) -> Result<ethers::types::H256, EvmError> {
        if self.evm.client.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }
        let router_address =
            PancakeSwapConfig::v2_router_address(self.evm.client.evm_type.unwrap())?;
        let wallet_address = self.evm.client.wallet.as_ref().unwrap().address();
        let router = self.router.v2_router(router_address);
        let tx = router.swap_exact_tokens_for_tokens(
            amount_in,
            amount_out_min,
            path,
            wallet_address,
            deadline.into(),
        );
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to swap tokens: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// Get V2 price  
    async fn get_v2_price(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<PriceInfo, EvmError> {
        let amounts = self
            .get_amounts_out_v2(amount_in, vec![token_in, token_out])
            .await?;
        let amount_out = amounts
            .last()
            .ok_or_else(|| EvmError::CalculationError("Invalid path".to_string()))?;

        Ok(PriceInfo {
            token_in,
            token_out,
            amount_in,
            amount_out: *amount_out,
            price: amount_out.as_u128() as f64 / amount_in.as_u128() as f64,
            price_impact: 0.0,
            timestamp: crate::tool::time_utils::current_timestamp() as u64,
        })
    }

    /// Get V3 price  
    async fn get_v3_price(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<PriceInfo, EvmError> {
        let fee = self.get_default_fee_tier(token_in, token_out);
        let amount_out = self
            .simulate_v3_swap(token_in, token_out, fee, amount_in)
            .await?;

        Ok(PriceInfo {
            token_in,
            token_out,
            amount_in,
            amount_out,
            price: amount_out.as_u128() as f64 / amount_in.as_u128() as f64,
            price_impact: 0.0,
            timestamp: crate::tool::time_utils::current_timestamp() as u64,
        })
    }

    /// Simulate V3 swap to get expected output by querying the actual Quoter contract
    async fn simulate_v3_swap(
        &self,
        token_in: Address,
        token_out: Address,
        fee: u32,
        amount_in: U256,
    ) -> Result<U256, EvmError> {
        use ethers::prelude::*;
        // Get Quoter contract address
        let quoter_address = match self.evm.client.evm_type {
            Some(EvmType::BSC_MAINNET) => BSC_QUOTER
                .parse::<Address>()
                .map_err(|e| EvmError::ConfigError(format!("Invalid BSC quoter address: {}", e)))?,
            Some(EvmType::ETHEREUM_MAINNET) => ETHEREUM_QUOTER.parse::<Address>().map_err(|e| {
                EvmError::ConfigError(format!("Invalid Ethereum quoter address: {}", e))
            })?,
            Some(EvmType::BASE_MAINNET) => BASE_QUOTER.parse::<Address>().map_err(|e| {
                EvmError::ConfigError(format!("Invalid Ethereum quoter address: {}", e))
            })?,
            _ => {
                return Err(EvmError::ConfigError(
                    "Unsupported chain for V3 Quoter".to_string(),
                ));
            }
        };
        // Create Quoter contract instance
        let quoter = IQuoter::new(quoter_address, self.evm.client.provider.clone());
        let amount_out = quoter
            .quote_exact_input_single(token_in, token_out, fee.into(), amount_in, U256::zero())
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to quote V3 swap: {}", e)))?;
        Ok(amount_out)
    }

    /// Calculate amount with slippage
    fn calculate_amount_with_slippage(&self, amount: U256, slippage_percent: f64) -> U256 {
        let slippage_factor = (100.0 - slippage_percent) / 100.0;
        let amount_f64 = amount.as_u128() as f64 * slippage_factor;
        U256::from(amount_f64 as u128)
    }

    /// Get default fee tier based on token pair
    fn get_default_fee_tier(&self, token_a: Address, token_b: Address) -> u32 {
        // Simple logic: use lower fees for stablecoin pairs
        let stable_tokens = [
            PancakeSwapConfig::busd_address(self.evm.client.evm_type.unwrap()).unwrap_or_default(),
            PancakeSwapConfig::usdt_address(self.evm.client.evm_type.unwrap()).unwrap_or_default(),
        ];
        if stable_tokens.contains(&token_a) && stable_tokens.contains(&token_b) {
            100 // 0.01% for stable pairs
        } else {
            500 // 0.05% for other pairs
        }
    }
}

/// PancakeSwap configuration for different chains
pub struct PancakeSwapConfig;

impl PancakeSwapConfig {
    pub fn v2_router_address(chain: EvmType) -> Result<Address, EvmError> {
        match chain {
            EvmType::BSC_MAINNET => Ok(BSC_ROUTER_V2.parse().unwrap()),
            EvmType::ETHEREUM_MAINNET => Ok(ETHEREUM_ROUTER_V2.parse().unwrap()),
            EvmType::BASE_MAINNET => Ok(BSC_ROUTER_V2.parse().unwrap()),
            _ => Err(EvmError::ConfigError(
                "Unsupported chain for PancakeSwap V2".to_string(),
            )),
        }
    }

    pub fn v3_router_address(chain: EvmType) -> Result<Address, EvmError> {
        match chain {
            EvmType::BSC_MAINNET => Ok(BSC_ROUTER_V3.parse().unwrap()),
            EvmType::ETHEREUM_MAINNET => Ok(ETHEREUM_ROUTER_V3.parse().unwrap()),
            EvmType::BASE_MAINNET => Ok(BASE_ROUTER_V3.parse().unwrap()),
            _ => Err(EvmError::ConfigError(
                "Unsupported chain for PancakeSwap V3".to_string(),
            )),
        }
    }

    pub fn busd_address(chain: EvmType) -> Result<Address, EvmError> {
        match chain {
            EvmType::BSC_MAINNET => Ok("0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"
                .parse()
                .unwrap()),
            EvmType::ETHEREUM_MAINNET => Ok("0x4Fabb145d64652a948d72533023f6E7A623C7C53"
                .parse()
                .unwrap()),
            _ => Err(EvmError::ConfigError(
                "Unsupported chain for BUSD".to_string(),
            )),
        }
    }

    pub fn usdt_address(chain: EvmType) -> Result<Address, EvmError> {
        match chain {
            EvmType::BSC_MAINNET => Ok("0x55d398326f99059fF775485246999027B3197955"
                .parse()
                .unwrap()),
            EvmType::ETHEREUM_MAINNET => Ok("0xdAC17F958D2ee523a2206206994597C13D831ec7"
                .parse()
                .unwrap()),
            _ => Err(EvmError::ConfigError(
                "Unsupported chain for USDT".to_string(),
            )),
        }
    }
}
