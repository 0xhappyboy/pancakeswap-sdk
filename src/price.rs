use crate::{EvmClient, EvmError};
use ethers::types::{Address, U256};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Represents historical price data for a token
#[derive(Debug, Clone)]
pub struct PriceHistory {
    pub timestamp: u64,
    pub price: f64,
    pub volume: f64,
}

/// Service for fetching and managing token prices
pub struct PriceService {
    client: Arc<EvmClient>,
    price_history: HashMap<Address, VecDeque<PriceHistory>>,
}

impl PriceService {
    pub fn new(client: Arc<EvmClient>) -> Self {
        Self {
            client,
            price_history: HashMap::new(),
        }
    }

    /// Get token price relative to another token
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use price::PriceService;
    /// async fn example(price_service: PriceService) -> Result<(), Box<dyn std::error::Error>> {
    /// let router = "0x10ED43C718714eb63d5aA57B78B54704E256024E".parse()?;
    /// let token_in = "0x...".parse()?;
    /// let token_out = "0x...".parse()?;
    /// let amount = U256::from(10_u64.pow(18));
    ///
    /// let price = price_service.get_price(router, token_in, token_out, amount).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_price(
        &self,
        router_address: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<U256, EvmError> {
        let router =
            crate::abi::IPancakeRouter02::new(router_address, self.client.provider.clone());
        let path = vec![token_in, token_out];
        let amounts = router
            .get_amounts_out(amount_in, path)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get price: {}", e)))?;
        if amounts.len() < 2 {
            return Err(EvmError::CalculationError(
                "Invalid amounts array".to_string(),
            ));
        }
        Ok(amounts[1])
    }

    /// Get prices for multiple tokens relative to a base token
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use price::PriceService;
    /// async fn example(price_service: PriceService) -> Result<(), Box<dyn std::error::Error>> {
    /// let router = "0x10ED43C718714eb63d5aA57B78B54704E256024E".parse()?;
    /// let base_token = "0x...".parse()?;
    /// let quote_tokens = vec!["0x...".parse()?, "0x...".parse()?];
    /// let amount = U256::from(10_u64.pow(18));
    ///
    /// let prices = price_service.get_prices(router, base_token, quote_tokens, amount).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_prices(
        &self,
        router_address: Address,
        base_token: Address,
        quote_tokens: Vec<Address>,
        amount_in: U256,
    ) -> Result<HashMap<Address, U256>, EvmError> {
        let mut prices = HashMap::new();
        for quote_token in quote_tokens {
            if base_token == quote_token {
                prices.insert(quote_token, amount_in);
                continue;
            }
            match self
                .get_price(router_address, base_token, quote_token, amount_in)
                .await
            {
                Ok(price) => {
                    prices.insert(quote_token, price);
                }
                Err(e) => {
                    eprintln!("Failed to get price for token {:?}: {}", quote_token, e);
                }
            }
        }
        Ok(prices)
    }

    /// Get token price relative to base token
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    /// use price::PriceService;
    /// async fn example(price_service: PriceService) -> Result<(), Box<dyn std::error::Error>> {
    /// let token = "0x...".parse()?;
    /// let base_token = "0x...".parse()?;
    ///
    /// let price = price_service.get_token_price(token, base_token).await?;
    /// println!("Price: {}", price);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_token_price(
        &self,
        token: Address,
        base_token: Address,
    ) -> Result<f64, EvmError> {
        if token == base_token {
            return Ok(1.0);
        }
        let router_address = self.get_default_router()?;
        let amount_in = U256::from(10_u64.pow(18)); // 1个代币
        match self
            .get_price(router_address, token, base_token, amount_in)
            .await
        {
            Ok(amount_out) => {
                let price = amount_out.as_u128() as f64 / 1e18;
                return Ok(price);
            }
            Err(_) => {}
        }
        let intermediate_tokens = self.get_common_intermediate_tokens();
        for intermediate in intermediate_tokens {
            if intermediate == token || intermediate == base_token {
                continue;
            }
            let path = vec![token, intermediate, base_token];
            let router =
                crate::abi::IPancakeRouter02::new(router_address, self.client.provider.clone());
            match router.get_amounts_out(amount_in, path).call().await {
                Ok(amounts) => {
                    if amounts.len() >= 3 {
                        let amount_out = amounts[2];
                        let price = amount_out.as_u128() as f64 / 1e18;
                        return Ok(price);
                    }
                }
                Err(_) => continue,
            }
        }
        Err(EvmError::CalculationError(format!(
            "Unable to get price for token {:?} relative to base token {:?}",
            token, base_token
        )))
    }

    fn get_default_router(&self) -> Result<Address, EvmError> {
        match self.client.chain {
            crate::EvmType::Bsc => {
                "0x10ED43C718714eb63d5aA57B78B54704E256024E" // PancakeSwap V2 Router
                    .parse()
                    .map_err(|_| EvmError::ConfigError("Invalid router address".to_string()))
            }
            crate::EvmType::Ethereum => {
                "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D" // Uniswap V2 Router
                    .parse()
                    .map_err(|_| EvmError::ConfigError("Invalid router address".to_string()))
            }
            _ => Err(EvmError::ConfigError("Unsupported chain".to_string())),
        }
    }

    fn get_common_intermediate_tokens(&self) -> Vec<Address> {
        match self.client.chain {
            crate::EvmType::Bsc => vec![
                // WBNB
                "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"
                    .parse()
                    .unwrap(),
                // BUSD
                "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"
                    .parse()
                    .unwrap(),
                // USDT
                "0x55d398326f99059fF775485246999027B3197955"
                    .parse()
                    .unwrap(),
            ],
            crate::EvmType::Ethereum => vec![
                // WETH
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(),
                // USDC
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .unwrap(),
                // USDT
                "0xdAC17F958D2ee523a2206206994597C13D831ec7"
                    .parse()
                    .unwrap(),
            ],
            _ => vec![],
        }
    }

    /// Get price via liquidity pair
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use price::PriceService;
    /// async fn example(price_service: PriceService) -> Result<(), Box<dyn std::error::Error>> {
    /// let pair = "0x...".parse()?;
    /// let token_in = "0x...".parse()?;
    /// let amount = U256::from(10_u64.pow(18));
    ///
    /// let price = price_service.get_price_via_pair(pair, token_in, amount).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn get_price_via_pair(
        &self,
        pair_address: Address,
        token_in: Address,
        amount_in: U256,
    ) -> Result<U256, EvmError> {
        let liquidity_service = crate::liquidity::LiquidityService::new(self.client.clone());
        let pool_info = liquidity_service.get_pool_info(pair_address).await?;
        if pool_info.reserve0.is_zero() || pool_info.reserve1.is_zero() {
            return Err(EvmError::CalculationError("Reserves are zero".to_string()));
        }
        let (reserve_in, reserve_out) = if token_in == pool_info.token0 {
            (pool_info.reserve0, pool_info.reserve1)
        } else if token_in == pool_info.token1 {
            (pool_info.reserve1, pool_info.reserve0)
        } else {
            return Err(EvmError::CalculationError("Token not in pair".to_string()));
        };
        let amount_in_with_fee = amount_in * U256::from(997);
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * U256::from(1000) + amount_in_with_fee;
        let amount_out = numerator / denominator;
        Ok(amount_out)
    }

    /// Calculate price impact for a trade
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use price::PriceService;
    /// async fn example(price_service: PriceService) -> Result<(), Box<dyn std::error::Error>> {
    /// let router = "0x...".parse()?;
    /// let token_in = "0x...".parse()?;
    /// let token_out = "0x...".parse()?;
    /// let amount = U256::from(10_u64.pow(18));
    ///
    /// let impact = price_service.get_price_impact(router, token_in, token_out, amount).await?;
    /// println!("Price impact: {}%", impact);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_price_impact(
        &self,
        router_address: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<f64, EvmError> {
        let price_service = PriceService::new(self.client.clone());
        let current_price = price_service
            .get_price(
                router_address,
                token_in,
                token_out,
                U256::from(10).pow(U256::from(18)),
            )
            .await?;
        let execution_price = price_service
            .get_price(router_address, token_in, token_out, amount_in)
            .await?;
        if current_price.is_zero() {
            return Err(EvmError::CalculationError(
                "Current price is zero".to_string(),
            ));
        }
        let price_impact = (current_price.as_u128() as f64 - execution_price.as_u128() as f64)
            / current_price.as_u128() as f64
            * 100.0;
        Ok(price_impact.abs())
    }

    /// Find optimal trading path
    ///
    /// # Example
    /// ```
    /// use ethers::types::{Address, U256};
    /// use price::PriceService;
    /// async fn example(price_service: PriceService) -> Result<(), Box<dyn std::error::Error>> {
    /// let router = "0x...".parse()?;
    /// let token_in = "0x...".parse()?;
    /// let token_out = "0x...".parse()?;
    /// let amount = U256::from(10_u64.pow(18));
    /// let intermediates = vec!["0x...".parse()?, "0x...".parse()?];
    ///
    /// let (path, amount) = price_service.find_optimal_path(
    ///     router, token_in, token_out, amount, intermediates
    /// ).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn find_optimal_path(
        &self,
        router_address: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        intermediate_tokens: Vec<Address>,
    ) -> Result<(Vec<Address>, U256), EvmError> {
        let mut best_amount = U256::zero();
        let mut best_path = vec![token_in, token_out];
        match self
            .get_price(router_address, token_in, token_out, amount_in)
            .await
        {
            Ok(amount) => {
                best_amount = amount;
            }
            Err(_) => {}
        }
        for intermediate in intermediate_tokens {
            if intermediate == token_in || intermediate == token_out {
                continue;
            }
            let path = vec![token_in, intermediate, token_out];
            let router =
                crate::abi::IPancakeRouter02::new(router_address, self.client.provider.clone());
            match router.get_amounts_out(amount_in, path.clone()).call().await {
                Ok(amounts) => {
                    if amounts.len() >= 3 {
                        let amount_out = amounts[2];
                        if amount_out > best_amount {
                            best_amount = amount_out;
                            best_path = path;
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        if best_amount.is_zero() {
            return Err(EvmError::CalculationError(
                "No valid path found".to_string(),
            ));
        }
        Ok((best_path, best_amount))
    }

    /// Record price history for analysis
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

    /// Calculate moving average for a token
    pub fn cal_moving_average(&self, token: Address, period: usize) -> Option<f64> {
        self.price_history.get(&token).and_then(|history| {
            if history.len() < period {
                return None;
            }
            let sum: f64 = history.iter().rev().take(period).map(|p| p.price).sum();
            Some(sum / period as f64)
        })
    }

    /// Calculate exponential moving average for a token
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

    /// Detect price anomalies using standard deviation
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

    /// Calculate Relative Strength Index (RSI)
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

    /// Calculate price volatility
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

    /// Calculate 24-hour price change
    pub async fn cal_price_change_24h(
        &self,
        pair_address: Address,
        base_token: Address,
    ) -> Result<f64, EvmError> {
        let liquidity_service = crate::liquidity::LiquidityService::new(self.client.clone());
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
}

/// Price data structure
#[derive(Debug, Clone)]
pub struct PriceData {
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub amount_out: U256,
    pub price: f64,
    pub timestamp: u64,
}

impl PriceData {
    pub fn new(
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        amount_out: U256,
        price: f64,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            token_in,
            token_out,
            amount_in,
            amount_out,
            price,
            timestamp,
        }
    }
}

/// Cache for price data with TTL
pub struct PriceCache {
    cache: HashMap<(Address, Address), (U256, u64)>,
    ttl: u64,
}

impl PriceCache {
    pub fn new(ttl: u64) -> Self {
        Self {
            cache: HashMap::new(),
            ttl,
        }
    }

    pub fn get(&self, token_in: Address, token_out: Address) -> Option<U256> {
        self.cache
            .get(&(token_in, token_out))
            .and_then(|(price, timestamp)| {
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if current_time - timestamp < self.ttl {
                    Some(*price)
                } else {
                    None
                }
            })
    }

    pub fn set(&mut self, token_in: Address, token_out: Address, price: U256) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.cache.insert((token_in, token_out), (price, timestamp));
    }

    pub fn clear_expired(&mut self) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.cache
            .retain(|_, (_, timestamp)| current_time - *timestamp < self.ttl);
    }
}
