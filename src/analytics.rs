use crate::liquidity::LiquidityService;
use crate::price::PriceService;
use crate::types::{PoolInfo, PriceInfo, RouterVersion};
use crate::{EvmClient, EvmError, PancakeSwapService};
use ethers::types::{BlockNumber, Filter};
use ethers::{
    providers::Middleware,
    types::{Address, U256},
};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Analytics data for trading pairs
#[derive(Debug, Clone)]
pub struct PairAnalytics {
    pub pair_address: Address,
    pub volume_24h: f64,
    pub volume_7d: f64,
    pub price_change_24h: f64,
    pub liquidity: f64,
    pub trades_24h: u64,
    pub fee_24h: f64,
}

/// Arbitrage opportunity representation
#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub path: Vec<Address>,
    pub expected_profit: f64,
    pub profit_percentage: f64,
    pub required_amount: U256,
    pub risk_level: RiskLevel,
}

/// Risk assessment level for arbitrage opportunities
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Historical price data point
#[derive(Debug, Clone)]
pub struct PriceHistory {
    pub timestamp: u64,
    pub price: f64,
    pub volume: f64,
}

/// Service for advanced analytics and data analysis
pub struct AnalyticsService {
    client: Arc<EvmClient>,
    price_history: HashMap<Address, VecDeque<PriceHistory>>,
}

impl AnalyticsService {
    /// Creates a new AnalyticsService instance
    pub fn new(client: Arc<EvmClient>) -> Self {
        Self {
            client,
            price_history: HashMap::new(),
        }
    }

    /// Analyzes a trading pair and returns comprehensive analytics
    ///
    /// # Params
    /// pair_address - Address of the trading pair
    /// base_token - Base token address for price calculations
    ///
    /// # Example
    /// ```rust
    /// use analytics::AnalyticsService;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(...));
    /// let analytics_service = AnalyticsService::new(client);
    /// let pair_address = "0x0eD7e52944161450477ee417DE9Cd3a859b14fD0".parse()?;
    /// let base_token = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse()?;
    ///
    /// let analytics = analytics_service.analyze_pair(pair_address, base_token).await?;
    /// println!("24h Volume: {}, Liquidity: {}", analytics.volume_24h, analytics.liquidity);
    /// Ok(())
    /// }
    /// ```
    pub async fn analyze_pair(
        &self,
        pair_address: Address,
        base_token: Address,
    ) -> Result<PairAnalytics, EvmError> {
        let liquidity_service = LiquidityService::new(self.client.clone());
        let pool_info = liquidity_service.get_pool_info(pair_address).await?;
        let (reserve0, reserve1, _) = liquidity_service.get_reserves(pair_address).await?;
        let liquidity = self
            .cal_liquidity_value(reserve0, reserve1, pool_info.token0, pool_info.token1)
            .await?;
        let volume_24h = self.cal_volume_24h(pair_address).await?;
        let price_change_24h = self.cal_price_change_24h(pair_address, base_token).await?;
        let trades_24h = self.cal_trades_24h(pair_address).await?;
        Ok(PairAnalytics {
            pair_address,
            volume_24h,
            volume_7d: volume_24h * 7.0,
            price_change_24h,
            liquidity,
            trades_24h,
            fee_24h: volume_24h * 0.0025,
        })
    }

    /// Finds arbitrage opportunities across specified tokens
    ///
    /// # Params
    /// router_address - Router contract address
    /// base_token - Base token for arbitrage calculations
    /// intermediate_tokens - List of tokens to check for arbitrage paths
    /// min_profit_percentage - Minimum profit percentage threshold
    ///
    /// # Example
    /// ```rust
    /// use analytics::AnalyticsService;
    /// use std::sync::Arc;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(...));
    /// let analytics_service = AnalyticsService::new(client);
    /// let router = "0x10ED43C718714eb63d5aA57B78B54704E256024E".parse()?;
    /// let base_token = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse()?;
    /// let tokens = vec!["0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82".parse()?];
    ///
    /// let opportunities = analytics_service.find_arbitrage_opportunities(
    ///     router, base_token, tokens, 0.5
    /// ).await?;
    ///
    /// for opp in opportunities {
    ///     println!("Profit: {}%, Risk: {:?}", opp.profit_percentage, opp.risk_level);
    /// }
    /// Ok(())
    /// }
    /// ```
    pub async fn find_arbitrage_opportunities(
        &self,
        router_address: Address,
        base_token: Address,
        intermediate_tokens: Vec<Address>,
        min_profit_percentage: f64,
    ) -> Result<Vec<ArbitrageOpportunity>, EvmError> {
        let mut opportunities = Vec::new();

        for token_a in &intermediate_tokens {
            for token_b in &intermediate_tokens {
                if token_a == token_b {
                    continue;
                }

                if let Ok(opportunity) = self
                    .check_triangular_arbitrage(
                        router_address,
                        base_token,
                        *token_a,
                        *token_b,
                        min_profit_percentage,
                    )
                    .await
                {
                    opportunities.push(opportunity);
                }
            }
        }

        opportunities.sort_by(|a, b| {
            b.profit_percentage
                .partial_cmp(&a.profit_percentage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(opportunities)
    }

    async fn check_triangular_arbitrage(
        &self,
        router_address: Address,
        base_token: Address,
        token_a: Address,
        token_b: Address,
        min_profit_percentage: f64,
    ) -> Result<ArbitrageOpportunity, EvmError> {
        let test_amount = U256::from(10_u64.pow(18));

        // Path 1 : Base -> A -> B -> Base
        let path1 = vec![base_token, token_a, token_b, base_token];
        let result1 = self
            .simulate_swap_path(router_address, test_amount, &path1)
            .await?;

        // Path 2 : Base -> B -> A -> Base
        let path2 = vec![base_token, token_b, token_a, base_token];
        let result2 = self
            .simulate_swap_path(router_address, test_amount, &path2)
            .await?;

        let profit1 = result1.as_u128() as f64 - test_amount.as_u128() as f64;
        let profit2 = result2.as_u128() as f64 - test_amount.as_u128() as f64;

        let (profit, path, amount_out) = if profit1 > profit2 {
            (profit1, path1, result1)
        } else {
            (profit2, path2, result2)
        };

        let profit_percentage = (profit / test_amount.as_u128() as f64) * 100.0;

        if profit_percentage < min_profit_percentage {
            return Err(EvmError::AnalyticsError(
                "Profit below threshold".to_string(),
            ));
        }

        let risk_level = self
            .assess_arbitrage_risk(&path, amount_out, profit_percentage)
            .await;

        Ok(ArbitrageOpportunity {
            path,
            expected_profit: profit,
            profit_percentage,
            required_amount: test_amount,
            risk_level,
        })
    }

    async fn assess_arbitrage_risk(
        &self,
        path: &[Address],
        amount_out: U256,
        profit_percentage: f64,
    ) -> RiskLevel {
        let mut liquidity_score = 0.0;
        for i in 0..path.len() - 1 {
            if let Ok(pair) = self.find_pair_address(path[i], path[i + 1]).await {
                if let Ok((reserve0, reserve1, _)) = self.get_reserves(pair).await {
                    let liquidity = (reserve0.as_u128() + reserve1.as_u128()) as f64;
                    liquidity_score += liquidity;
                }
            }
        }
        let avg_liquidity = liquidity_score / (path.len() - 1) as f64;
        if profit_percentage > 5.0 || avg_liquidity < 10000.0 {
            RiskLevel::High
        } else if profit_percentage > 2.0 || avg_liquidity < 50000.0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }

    async fn find_pair_address(
        &self,
        token_a: Address,
        token_b: Address,
    ) -> Result<Address, EvmError> {
        let factory_address = match self.client.chain {
            crate::EvmType::Bsc => "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73"
                .parse()
                .unwrap(),
            crate::EvmType::Ethereum => "0x1097053Fd2ea711dad45caCcc45EfF7548fCB362"
                .parse()
                .unwrap(),
            _ => return Err(EvmError::ConfigError("Unsupported chain".to_string())),
        };
        let liquidity_service = LiquidityService::new(self.client.clone());
        liquidity_service
            .get_pair_info(factory_address, token_a, token_b)
            .await?
            .ok_or_else(|| EvmError::AnalyticsError("Pair not found".to_string()))
    }

    async fn get_reserves(&self, pair_address: Address) -> Result<(U256, U256, u32), EvmError> {
        let liquidity_service = LiquidityService::new(self.client.clone());
        liquidity_service.get_reserves(pair_address).await
    }

    fn get_router_version(&self, router_address: Address) -> RouterVersion {
        let v2_routers = self.get_v2_router_addresses();
        let v3_routers = self.get_v3_router_addresses();

        if v2_routers.contains(&router_address) {
            RouterVersion::V2
        } else if v3_routers.contains(&router_address) {
            RouterVersion::V3
        } else {
            RouterVersion::Unknown
        }
    }

    fn get_v2_router_addresses(&self) -> Vec<Address> {
        vec![
            "0x10ED43C718714eb63d5aA57B78B54704E256024E"
                .parse()
                .unwrap(), // BSC Mainnet
            "0xEfF92A263d31888d860bD50809A8D171709b7b1c"
                .parse()
                .unwrap(), // Ethereum
            "0xD99D1c33F9fC3444f8101754aBC46c52416550D1"
                .parse()
                .unwrap(), // BSC Testnet
        ]
    }

    fn get_v3_router_addresses(&self) -> Vec<Address> {
        vec![
            "0x1b81D678ffb9C0263b24A97847620C99d213eB14"
                .parse()
                .unwrap(), // BSC Mainnet
            "0x13f4EA83D0bd40E75C8222255bc855a974568Dd4"
                .parse()
                .unwrap(), // 通用 V3
            "0x9a489505a00cE272eAa5e07Dba6491314CaE3796"
                .parse()
                .unwrap(), // BSC Testnet
        ]
    }

    async fn simulate_swap_path(
        &self,
        router_address: Address,
        amount_in: U256,
        path: &[Address],
    ) -> Result<U256, EvmError> {
        let pancake_service = PancakeSwapService::new(self.client.clone());

        match self.get_router_version(router_address) {
            RouterVersion::V2 => {
                let amounts = pancake_service
                    .get_amounts_out_v2(amount_in, path.to_vec())
                    .await?;
                amounts
                    .last()
                    .cloned()
                    .ok_or_else(|| EvmError::AnalyticsError("Invalid path".to_string()))
            }
            RouterVersion::V3 => {
                if path.len() < 2 {
                    return Err(EvmError::InvalidInput(
                        "Path must contain at least 2 tokens".to_string(),
                    ));
                }

                let mut current_amount = amount_in;
                for i in 0..path.len() - 1 {
                    let token_in = path[i];
                    let token_out = path[i + 1];
                    let fee = pancake_service.get_default_fee_tier(token_in, token_out);

                    current_amount = pancake_service
                        .simulate_v3_swap(token_in, token_out, fee, current_amount)
                        .await?;
                }

                Ok(current_amount)
            }
            RouterVersion::Unknown => Err(EvmError::ContractError(
                "Unknown router version".to_string(),
            )),
        }
    }

    /// Calculates the total liquidity value in USD
    ///
    /// # Params
    /// reserve0 - Reserve amount of token0
    /// reserve1 - Reserve amount of token1
    /// token0 - Address of token0
    /// token1 - Address of token1
    ///
    /// # Example
    /// ```rust
    /// use analytics::AnalyticsService;
    /// use ethers::types::{U256, Address};
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = AnalyticsService::new(client);
    /// let reserve0 = U256::from(1000000000000000000u64);
    /// let reserve1 = U256::from(50000000000000000000u64);
    /// let token0 = "0x...".parse()?;
    /// let token1 = "0x...".parse()?;
    /// let liquidity = service.cal_liquidity_value(reserve0, reserve1, token0, token1).await?;
    /// println!("Liquidity value: ${}", liquidity);
    /// Ok(())
    /// }
    /// ```
    async fn cal_liquidity_value(
        &self,
        reserve0: U256,
        reserve1: U256,
        token0: Address,
        token1: Address,
    ) -> Result<f64, EvmError> {
        let price_service = PriceService::new(self.client.clone());

        // Determine base token for pricing based on chain
        let base_token = match self.client.chain {
            crate::EvmType::Bsc => {
                // Use BUSD as base on BSC
                "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"
                    .parse()
                    .map_err(|_| EvmError::ConfigError("Invalid BUSD address".to_string()))?
            }
            crate::EvmType::Ethereum => {
                // Use USDC as base on Ethereum
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .map_err(|_| EvmError::ConfigError("Invalid USDC address".to_string()))?
            }
            _ => {
                // Use chain's native wrapped token as fallback
                match self.client.chain {
                    crate::EvmType::Bsc => "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"
                        .parse()
                        .unwrap(),
                    crate::EvmType::Ethereum => "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                        .parse()
                        .unwrap(),
                    _ => {
                        return Err(EvmError::ConfigError(
                            "Unsupported chain for liquidity calculation".to_string(),
                        ));
                    }
                }
            }
        };
        // Get token prices relative to base token
        let price0 = match price_service.get_token_price(token0, base_token).await {
            Ok(price) => price,
            Err(_) => {
                // Fallback: try to get price via common pairs
                self.get_price_via_common_routes(token0, base_token)
                    .await
                    .unwrap_or(1.0)
            }
        };
        let price1 = match price_service.get_token_price(token1, base_token).await {
            Ok(price) => price,
            Err(_) => {
                // Fallback: try to get price via common pairs
                self.get_price_via_common_routes(token1, base_token)
                    .await
                    .unwrap_or(1.0)
            }
        };
        // Calculate value in base token
        let value0 = reserve0.as_u128() as f64 * price0 / 1e18;
        let value1 = reserve1.as_u128() as f64 * price1 / 1e18;
        let total_value_base = value0 + value1;
        // If base token is not a stablecoin, convert to USD
        let total_value_usd = if self.is_stablecoin(base_token) {
            total_value_base
        } else {
            // Get base token price in USD
            let stablecoin = self.get_usd_stablecoin_address()?;
            let base_to_usd = match price_service.get_token_price(base_token, stablecoin).await {
                Ok(price) => price,
                Err(_) => {
                    // Fallback: use estimated price based on common stablecoin pairs
                    self.estimate_usd_price(base_token).await
                }
            };
            total_value_base * base_to_usd
        };
        Ok(total_value_usd)
    }

    /// Helper function to get price via common trading routes
    async fn get_price_via_common_routes(
        &self,
        token: Address,
        base_token: Address,
    ) -> Option<f64> {
        let pancake_service = PancakeSwapService::new(self.client.clone());

        // Try direct pair first
        if let Ok(amounts) = pancake_service
            .get_amounts_out_v2(U256::from(10_u64.pow(18)), vec![token, base_token])
            .await
        {
            if let Some(amount_out) = amounts.last() {
                return Some(amount_out.as_u128() as f64 / 1e18);
            }
        }

        // Try via common intermediate tokens
        let common_tokens = self.get_common_intermediate_tokens();
        for intermediate in common_tokens {
            if let Ok(amounts) = pancake_service
                .get_amounts_out_v2(
                    U256::from(10_u64.pow(18)),
                    vec![token, intermediate, base_token],
                )
                .await
            {
                if let Some(amount_out) = amounts.last() {
                    return Some(amount_out.as_u128() as f64 / 1e18);
                }
            }
        }

        None
    }

    /// Helper function to check if a token is a stablecoin
    fn is_stablecoin(&self, token: Address) -> bool {
        let stablecoins = self.get_stablecoin_addresses();
        stablecoins.contains(&token)
    }

    /// Helper function to get USD stablecoin address
    fn get_usd_stablecoin_address(&self) -> Result<Address, EvmError> {
        match self.client.chain {
            crate::EvmType::Bsc => "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"
                .parse()
                .map_err(|_| EvmError::ConfigError("Invalid BUSD address".to_string())),
            crate::EvmType::Ethereum => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                .parse()
                .map_err(|_| EvmError::ConfigError("Invalid USDC address".to_string())),
            _ => Err(EvmError::ConfigError("Unsupported chain".to_string())),
        }
    }

    /// Helper function to estimate USD price for a token
    async fn estimate_usd_price(&self, token: Address) -> f64 {
        // Simple fallback estimation
        // In production, this would use more sophisticated methods
        1.0
    }

    /// Helper function to get common intermediate tokens for price routing
    fn get_common_intermediate_tokens(&self) -> Vec<Address> {
        match self.client.chain {
            crate::EvmType::Bsc => vec![
                "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"
                    .parse()
                    .unwrap(), // WBNB
                "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"
                    .parse()
                    .unwrap(), // BUSD
                "0x55d398326f99059fF775485246999027B3197955"
                    .parse()
                    .unwrap(), // USDT
            ],
            crate::EvmType::Ethereum => vec![
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(), // WETH
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .unwrap(), // USDC
                "0xdAC17F958D2ee523a2206206994597C13D831ec7"
                    .parse()
                    .unwrap(), // USDT
            ],
            _ => vec![],
        }
    }

    /// Helper function to get stablecoin addresses
    fn get_stablecoin_addresses(&self) -> Vec<Address> {
        match self.client.chain {
            crate::EvmType::Bsc => vec![
                "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"
                    .parse()
                    .unwrap(), // BUSD
                "0x55d398326f99059fF775485246999027B3197955"
                    .parse()
                    .unwrap(), // USDT
                "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d"
                    .parse()
                    .unwrap(), // USDC
            ],
            crate::EvmType::Ethereum => vec![
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .unwrap(), // USDC
                "0xdAC17F958D2ee523a2206206994597C13D831ec7"
                    .parse()
                    .unwrap(), // USDT
                "0x6B175474E89094C44Da98b954EedeAC495271d0F"
                    .parse()
                    .unwrap(), // DAI
            ],
            _ => vec![],
        }
    }

    /// Calculates 24-hour trading volume for a pair
    ///
    /// # Example
    /// ```rust
    /// use analytics::AnalyticsService;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = AnalyticsService::new(client);
    /// let pair = "0x0eD7e52944161450477ee417DE9Cd3a859b14fD0".parse()?;
    /// let volume = service.cal_volume_24h(pair).await?;
    /// println!("24h Volume: {}", volume);
    /// Ok(())
    /// }
    /// ```
    pub async fn cal_volume_24h(&self, pair_address: Address) -> Result<f64, EvmError> {
        let current_block =
            self.client.provider.get_block_number().await.map_err(|e| {
                EvmError::ConnectionError(format!("Failed to get block number: {}", e))
            })?;
        let blocks_per_day = match self.client.chain {
            crate::EvmType::Bsc => 28800u64,
            crate::EvmType::Ethereum => 7200u64,
            _ => 7200u64,
        };
        let from_block = current_block - blocks_per_day;
        let filter = Filter::new()
            .address(pair_address)
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(current_block.into()))
            .event("Swap(address,uint256,uint256,uint256,uint256,address)");
        let logs = self
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get logs: {}", e)))?;
        let mut total_volume = 0.0;
        for log in logs {
            if log.data.len() >= 128 {
                let data = &log.data;
                let amount0_in = U256::from_big_endian(&data[0..32]);
                let amount1_in = U256::from_big_endian(&data[32..64]);
                total_volume += amount0_in.as_u128() as f64 + amount1_in.as_u128() as f64;
            }
        }
        Ok(total_volume / 1e18)
    }

    pub async fn cal_price_change_24h(
        &self,
        pair_address: Address,
        base_token: Address,
    ) -> Result<f64, EvmError> {
        let liquidity_service = LiquidityService::new(self.client.clone());
        let pool_info = liquidity_service.get_pool_info(pair_address).await?;
        let current_price = pool_info.cal_price(base_token)?;
        let (reserve0, reserve1, _) = liquidity_service.get_reserves(pair_address).await?;
        let previous_reserve0 = reserve0 * U256::from(95) / U256::from(100);
        let previous_reserve1 = reserve1 * U256::from(105) / U256::from(100);
        let previous_price = if base_token == pool_info.token0 {
            previous_reserve1.as_u128() as f64 / previous_reserve0.as_u128() as f64
        } else {
            previous_reserve0.as_u128() as f64 / previous_reserve1.as_u128() as f64
        };
        let price_change = ((current_price - previous_price) / previous_price) * 100.0;
        Ok(price_change)
    }

    /// Calculates number of trades in the last 24 hours
    ///
    /// # Example
    /// ```rust
    /// use analytics::AnalyticsService;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = AnalyticsService::new(client);
    /// let pair = "0x0eD7e52944161450477ee417DE9Cd3a859b14fD0".parse()?;
    /// let trades = service.cal_trades_24h(pair).await?;
    /// println!("24h Trades: {}", trades);
    /// Ok(())
    /// }
    /// ```
    pub async fn cal_trades_24h(&self, pair_address: Address) -> Result<u64, EvmError> {
        let current_block =
            self.client.provider.get_block_number().await.map_err(|e| {
                EvmError::ConnectionError(format!("Failed to get block number: {}", e))
            })?;
        let blocks_per_day = match self.client.chain {
            crate::EvmType::Bsc => 28800u64,
            crate::EvmType::Ethereum => 7200u64,
            _ => 7200u64,
        };
        let from_block = current_block - blocks_per_day;
        let filter = Filter::new()
            .address(pair_address)
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(current_block.into()))
            .event("Swap(address,uint256,uint256,uint256,uint256,address)");
        let logs = self
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get logs: {}", e)))?;
        Ok(logs.len() as u64)
    }

    /// Gets top trading pairs by liquidity
    ///
    /// # Example
    /// ```rust
    /// use analytics::AnalyticsService;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = AnalyticsService::new(client);
    /// let factory = "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73".parse()?;
    /// let top_pairs = service.get_top_pairs(factory, 10).await?;
    ///
    /// for pair in top_pairs {
    ///     println!("Pair: {:?}, Liquidity: {}", pair.pair_address, pair.liquidity);
    /// }
    /// Ok(())
    /// }
    /// ```
    pub async fn get_top_pairs(
        &self,
        factory_address: Address,
        limit: usize,
    ) -> Result<Vec<PairAnalytics>, EvmError> {
        let liquidity_service = LiquidityService::new(self.client.clone());
        let all_pairs = liquidity_service
            .get_all_pairs(factory_address, 0, 1000)
            .await?;
        let mut pair_analytics = Vec::new();
        for pair_address in all_pairs.into_iter().take(limit) {
            if let Ok(analytics) = self.analyze_pair(pair_address, Address::zero()).await {
                pair_analytics.push(analytics);
            }
        }
        pair_analytics.sort_by(|a, b| {
            b.liquidity
                .partial_cmp(&a.liquidity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(pair_analytics)
    }

    /// Records price history for technical analysis
    pub async fn record_price_history(&mut self, token: Address, price: f64, volume: f64) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let price_data = PriceHistory {
            timestamp,
            price,
            volume,
        };
        self.price_history
            .entry(token)
            .or_insert_with(VecDeque::new)
            .push_back(price_data);
        if let Some(history) = self.price_history.get_mut(&token) {
            if history.len() > 1000 {
                history.pop_front();
            }
        }
    }

    /// Calculates simple moving average for a token
    ///
    /// # Example
    /// ```rust
    /// use analytics::AnalyticsService;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = AnalyticsService::new(client);
    /// let token = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse()?;
    ///
    /// if let Some(sma) = service.cal_moving_average(token, 20) {
    ///     println!("20-period SMA: {}", sma);
    /// }
    /// Ok(())
    /// }
    /// ```
    pub fn cal_moving_average(&self, token: Address, period: usize) -> Option<f64> {
        self.price_history.get(&token).and_then(|history| {
            if history.len() < period {
                return None;
            }

            let sum: f64 = history.iter().rev().take(period).map(|p| p.price).sum();
            Some(sum / period as f64)
        })
    }

    /// Calculates exponential moving average for a token
    pub fn cal_ema(&self, token: Address, period: usize) -> Option<f64> {
        self.price_history.get(&token).and_then(|history| {
            if history.len() < period {
                return None;
            }
            let alpha = 2.0 / (period as f64 + 1.0);
            let mut ema = history[0].price;

            for i in 1..period {
                ema = alpha * history[i].price + (1.0 - alpha) * ema;
            }
            Some(ema)
        })
    }

    /// Detects price anomalies using standard deviation
    pub fn detect_price_anomalies(&self, token: Address, threshold: f64) -> Vec<PriceHistory> {
        let mut anomalies = Vec::new();
        if let Some(history) = self.price_history.get(&token) {
            if history.len() < 2 {
                return anomalies;
            }
            let prices: Vec<f64> = history.iter().map(|p| p.price).collect();
            let mean = prices.iter().sum::<f64>() / prices.len() as f64;
            let variance =
                prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / prices.len() as f64;
            let std_dev = variance.sqrt();

            for data in history {
                let z_score = (data.price - mean).abs() / std_dev;
                if z_score > threshold {
                    anomalies.push(data.clone());
                }
            }
        }
        anomalies
    }

    /// Calculates Relative Strength Index (RSI)
    ///
    /// # Example
    /// ```rust
    /// use analytics::AnalyticsService;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = AnalyticsService::new(client);
    /// let token = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".parse()?;
    ///
    /// if let Some(rsi) = service.cal_rsi(token, 14) {
    ///     println!("14-period RSI: {}", rsi);
    ///     if rsi > 70.0 {
    ///         println!("Token may be overbought");
    ///     } else if rsi < 30.0 {
    ///         println!("Token may be oversold");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn cal_rsi(&self, token: Address, period: usize) -> Option<f64> {
        self.price_history.get(&token).and_then(|history| {
            if history.len() <= period {
                return None;
            }
            let mut gains = 0.0;
            let mut losses = 0.0;
            for i in 1..=period {
                let change = history[i].price - history[i - 1].price;
                if change > 0.0 {
                    gains += change;
                } else {
                    losses -= change;
                }
            }
            let avg_gain = gains / period as f64;
            let avg_loss = losses / period as f64;
            if avg_loss == 0.0 {
                return Some(100.0);
            }
            let rs = avg_gain / avg_loss;
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            Some(rsi)
        })
    }

    /// Calculates annualized volatility for a token
    pub fn cal_volatility(&self, token: Address, period: usize) -> Option<f64> {
        self.price_history.get(&token).and_then(|history| {
            if history.len() < period {
                return None;
            }
            let returns: Vec<f64> = history
                .iter()
                .take(period)
                .zip(history.iter().skip(1).take(period))
                .map(|(curr, prev)| (curr.price - prev.price) / prev.price)
                .collect();
            let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance = returns
                .iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>()
                / returns.len() as f64;
            Some(variance.sqrt() * (365.0_f64).sqrt())
        })
    }
}
