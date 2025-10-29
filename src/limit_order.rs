use crate::{PancakeSwapConfig, PancakeSwapService, price::PriceService};
use ethers::types::{Address, U256};
use evm_sdk::Evm;
use evm_sdk::types::EvmError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, interval};

/// Represents the status of a limit order
#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Pending,
    Filled,
    Cancelled,
    Expired,
}

/// Contains all information about a limit order
#[derive(Debug, Clone)]
pub struct LimitOrder {
    pub order_id: U256,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub amount_out_min: U256,
    pub limit_price: f64,
    pub actual_price: Option<f64>,
    pub status: OrderStatus,
    pub created_at: u64,
    pub expiry: u64,
    pub path: Vec<Address>,
    pub tx_hash: Option<ethers::types::H256>,
}

/// Service for managing and executing limit orders
pub struct LimitOrderService {
    evm: Arc<Evm>,
    pending_orders: HashMap<U256, LimitOrder>,
}

impl LimitOrderService {
    /// Creates a new LimitOrderService instance
    pub fn new(evm: Arc<Evm>) -> Self {
        Self {
            evm,
            pending_orders: HashMap::new(),
        }
    }

    /// Creates a new limit order
    ///
    /// # Params
    /// router_address - Address of the DEX router
    /// token_in - Input token address
    /// token_out - Output token address
    /// amount_in - Amount of input token
    /// limit_price - Target price for execution
    /// expiry_minutes - Order validity period in minutes
    /// path - Optional custom swap path
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::{Address, U256};
    /// use std::str::FromStr;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = EvmClient::new(EvmType::Bsc).await?;
    /// let mut service = LimitOrderService::new(client);
    ///
    /// let router = Address::from_str("0x10ED43C718714eb63d5aA57B78B54704E256024E")?;
    /// let wbnb = Address::from_str("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c")?;
    /// let busd = Address::from_str("0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56")?;
    ///
    /// let order_id = service.create_limit_order(
    ///     router,
    ///     wbnb,
    ///     busd,
    ///     U256::from(1_000_000_000_000_000_000u64), // 1 BNB
    ///     300.0, // Limit price: 1 BNB = 300 BUSD
    ///     60, // Expires in 60 minutes
    ///     None, // Use default path
    /// ).await?;
    /// Ok(())
    /// }
    /// ```
    pub async fn create_limit_order(
        &mut self,
        router_address: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        limit_price: f64,
        expiry_minutes: u64,
        path: Option<Vec<Address>>,
    ) -> Result<U256, EvmError> {
        let current_price = self
            .get_current_price(router_address, token_in, token_out, amount_in)
            .await?;
        if current_price >= limit_price {
            return Err(EvmError::Error(
                "Current price is already better than limit price".to_string(),
            ));
        }
        let order_id = U256::from(ethers::utils::keccak256(
            format!("{}{}{}{}", token_in, token_out, amount_in, limit_price).as_bytes(),
        ));
        let path = path.unwrap_or_else(|| vec![token_in, token_out]);
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiry = created_at + expiry_minutes * 60;
        let amount_out_min = self
            .calculate_amount_out_min(amount_in, limit_price, current_price)
            .await?;
        let order = LimitOrder {
            order_id,
            token_in,
            token_out,
            amount_in,
            amount_out_min,
            limit_price,
            actual_price: None,
            status: OrderStatus::Pending,
            created_at,
            expiry,
            path,
            tx_hash: None,
        };
        self.pending_orders.insert(order_id, order.clone());
        self.start_order_monitoring(order_id, router_address)
            .await?;
        Ok(order_id)
    }

    /// Gets the current price for a token pair
    async fn get_current_price(
        &self,
        router_address: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<f64, EvmError> {
        let price_service = PriceService::new(self.evm.clone());
        let amount_out = price_service
            .get_price(router_address, token_in, token_out, amount_in)
            .await?;
        let price = amount_out.as_u128() as f64 / amount_in.as_u128() as f64;
        Ok(price)
    }

    /// Calculates the minimum output amount with slippage protection
    async fn calculate_amount_out_min(
        &self,
        amount_in: U256,
        limit_price: f64,
        current_price: f64,
    ) -> Result<U256, EvmError> {
        let expected_amount_out = (amount_in.as_u128() as f64 * limit_price) as u128;
        let amount_out_min = (expected_amount_out as f64 * 0.995) as u128; // 0.5% 滑点保护
        Ok(U256::from(amount_out_min))
    }

    /// Starts monitoring an order for execution conditions
    async fn start_order_monitoring(
        &mut self,
        order_id: U256,
        router_address: Address,
    ) -> Result<(), EvmError> {
        let client = self.evm.client.clone();
        let mut interval = interval(Duration::from_secs(10)); // 每10秒检查一次
        tokio::spawn(async move { todo!() });
        Ok(())
    }

    /// Executes a limit order when conditions are met
    ///
    /// # Params
    /// order_id - ID of the order to execute
    ///
    /// # Example
    /// ```rust
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let mut service = LimitOrderService::new(client);
    /// let order_id = U256::from(12345u64);
    /// let tx_hash = service.execute_limit_order(order_id).await?;
    /// println!("Order executed with tx: {:?}", tx_hash);
    /// Ok(())
    /// }
    /// ```
    pub async fn execute_limit_order(
        &mut self,
        order_id: U256,
    ) -> Result<ethers::types::H256, EvmError> {
        let order = self
            .pending_orders
            .get(&order_id)
            .ok_or_else(|| EvmError::Error("Order not found".to_string()))?;
        if order.status != OrderStatus::Pending {
            return Err(EvmError::Error("Order is not pending".to_string()));
        }
        if std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            > order.expiry
        {
            return Err(EvmError::Error("Order has expired".to_string()));
        }
        let pancake_service = PancakeSwapService::new(self.evm.clone());
        let tx_hash = pancake_service
            .swap_exact_tokens_for_tokens(
                order.amount_in,
                order.amount_out_min,
                order.path.clone(),
                order.expiry as u64,
            )
            .await?;
        if let Some(order) = self.pending_orders.get_mut(&order_id) {
            order.status = OrderStatus::Filled;
            order.tx_hash = Some(tx_hash);
        }
        Ok(tx_hash)
    }

    /// Cancels a pending limit order
    ///
    /// # Params
    /// order_id` - ID of the order to cancel
    ///
    /// # Example
    /// ```rust
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let mut service = LimitOrderService::new(client);
    /// let order_id = U256::from(12345u64);
    /// service.cancel_limit_order(order_id)?;
    /// println!("Order cancelled successfully");
    /// Ok(())
    /// }
    /// ```
    pub fn cancel_limit_order(&mut self, order_id: U256) -> Result<(), EvmError> {
        if let Some(order) = self.pending_orders.get_mut(&order_id) {
            if order.status == OrderStatus::Pending {
                order.status = OrderStatus::Cancelled;
                Ok(())
            } else {
                Err(EvmError::Error(
                    "Cannot cancel non-pending order".to_string(),
                ))
            }
        } else {
            Err(EvmError::Error("Order not found".to_string()))
        }
    }

    /// Retrieves order information by ID
    pub fn get_order(&self, order_id: U256) -> Option<&LimitOrder> {
        self.pending_orders.get(&order_id)
    }

    /// Returns all orders regardless of status
    pub fn get_all_orders(&self) -> Vec<&LimitOrder> {
        self.pending_orders.values().collect()
    }

    /// Returns only pending orders
    pub fn get_pending_orders(&self) -> Vec<&LimitOrder> {
        self.pending_orders
            .values()
            .filter(|order| order.status == OrderStatus::Pending)
            .collect()
    }

    /// Checks and executes all orders that meet their execution conditions
    ///
    /// # Returns
    /// * `Ok(Vec<H256>)` - Vector of transaction hashes for executed orders
    /// * `Err(EvmError)` - Error if execution fails
    ///
    /// # Example
    /// ```rust
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(EvmClient::new(EvmType::Bsc).await?);
    /// let mut service = LimitOrderService::new(client);
    /// let executed_orders = service.check_and_execute_orders().await?;
    /// println!("Executed {} orders", executed_orders.len());
    /// Ok(())
    /// }
    /// ```
    pub async fn check_and_execute_orders(&mut self) -> Result<Vec<ethers::types::H256>, EvmError> {
        let mut executed_orders = Vec::new();
        let pending_orders: Vec<U256> = self
            .get_pending_orders()
            .iter()
            .map(|order| order.order_id)
            .collect();
        for order_id in pending_orders {
            let should_execute = self.should_execute_order(order_id).await?;
            if should_execute {
                match self.execute_limit_order(order_id).await {
                    Ok(tx_hash) => executed_orders.push(tx_hash),
                    Err(e) => eprintln!("Failed to execute order {}: {}", order_id, e),
                }
            }
        }
        Ok(executed_orders)
    }

    /// Determines if an order should be executed based on current market conditions
    async fn should_execute_order(&self, order_id: U256) -> Result<bool, EvmError> {
        let order = self
            .pending_orders
            .get(&order_id)
            .ok_or_else(|| EvmError::Error("Order not found".to_string()))?;
        let current_price = self
            .get_current_price(
                PancakeSwapConfig::v2_router_address(self.evm.client.evm_type.unwrap())?,
                order.token_in,
                order.token_out,
                order.amount_in,
            )
            .await?;
        Ok(current_price >= order.limit_price)
    }
}
