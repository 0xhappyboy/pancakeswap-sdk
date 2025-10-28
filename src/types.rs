use ethers::types::{Address, U256};
use std::fmt;

#[derive(Debug)]
pub enum EvmError {
    ConfigError(String),
    ConnectionError(String),
    RpcError(String),
    WalletError(String),
    TransactionError(String),
    ContractError(String),
    InvalidInput(String),
    IOError(String),
    AaveError(String),
    ListenerError(String),
    ProviderError(String),
    CalculationError(String),
    MempoolError(String),
    LimitOrderError(String),
    AnalyticsError(String),
    ArbitrageError(String),
    PriceError(String),
    VersionError(String),
    Error(String),
}

impl fmt::Display for EvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvmError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            EvmError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            EvmError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            EvmError::WalletError(msg) => write!(f, "Wallet error: {}", msg),
            EvmError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            EvmError::ContractError(msg) => write!(f, "Contract error: {}", msg),
            EvmError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            EvmError::IOError(msg) => write!(f, "IO Error: {}", msg),
            EvmError::AaveError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::ListenerError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::ProviderError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::CalculationError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::MempoolError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::LimitOrderError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::AnalyticsError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::Error(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::ArbitrageError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::PriceError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::VersionError(msg) => write!(f, "Aave Error: {}", msg),
        }
    }
}

impl std::error::Error for EvmError {}

#[derive(Debug, Clone, PartialEq)]
pub enum RouterVersion {
    V2,
    V3,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvmType {
    Ethereum,
    Arb,
    Bsc,
    Base,
    HyperEVM,
    Plasma,
}

impl EvmType {
    pub fn name(&self) -> &'static str {
        match self {
            EvmType::Ethereum => "Ethereum",
            EvmType::Arb => "Arbitrum",
            EvmType::Bsc => "Binance Smart Chain",
            EvmType::Base => "Base",
            EvmType::HyperEVM => "HyperEVM",
            EvmType::Plasma => "Plasma",
        }
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            EvmType::Ethereum => 1,
            EvmType::Arb => 42161,
            EvmType::Bsc => 56,
            EvmType::Base => 8453,
            EvmType::HyperEVM => 777,
            EvmType::Plasma => 94,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwapEvent {
    pub sender: Address,
    pub to: Address,
    pub amount0_in: U256,
    pub amount1_in: U256,
    pub amount0_out: U256,
    pub amount1_out: U256,
}

#[derive(Debug, Clone)]
pub struct MintEvent {
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
}

#[derive(Debug, Clone)]
pub struct BurnEvent {
    pub sender: Address,
    pub to: Address,
    pub amount0: U256,
    pub amount1: U256,
}

#[derive(Debug, Clone)]
pub struct PairCreatedEvent {
    pub token0: Address,
    pub token1: Address,
    pub pair: Address,
}

#[derive(Debug, Clone)]
pub struct V3SwapEvent {
    pub sender: Address,
    pub recipient: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub sqrt_price_x96: U256,
    pub liquidity: U256,
    pub tick: i32,
}

#[derive(Debug, Clone)]
pub struct V3MintEvent {
    pub sender: Address,
    pub owner: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: U256,
    pub amount0: U256,
    pub amount1: U256,
}

#[derive(Debug, Clone)]
pub struct V3BurnEvent {
    pub owner: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: U256,
    pub amount0: U256,
    pub amount1: U256,
}

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub input_token: Address,
    pub output_token: Address,
    pub input_amount: U256,
    pub output_amount: U256,
    pub path: Vec<Address>,
    pub tx_hash: ethers::types::H256,
    pub gas_used: U256,
    pub gas_price: U256,
}

#[derive(Debug, Clone)]
pub struct AddLiquidityResult {
    pub token_a: Address,
    pub token_b: Address,
    pub amount_a: U256,
    pub amount_b: U256,
    pub liquidity: U256,
    pub tx_hash: ethers::types::H256,
}

#[derive(Debug, Clone)]
pub struct RemoveLiquidityResult {
    pub token_a: Address,
    pub token_b: Address,
    pub amount_a: U256,
    pub amount_b: U256,
    pub liquidity: U256,
    pub tx_hash: ethers::types::H256,
}

#[derive(Debug, Clone)]
pub struct PriceInfo {
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub amount_out: U256,
    pub price: f64,
    pub price_impact: f64,
    pub timestamp: u64,
}

impl From<PriceInfo> for SwapQuote {
    fn from(info: PriceInfo) -> Self {
        SwapQuote {
            amount_out: info.amount_out,
            path: vec![info.token_in, info.token_out],
            gas_estimate: U256::zero(),
            price_impact: info.price_impact,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub path: Vec<Address>,
    pub amounts: Vec<U256>,
    pub gas_estimate: U256,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub address: Address,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub total_supply: U256,
}

#[derive(Debug, Clone)]
pub struct PoolState {
    pub reserve0: U256,
    pub reserve1: U256,
    pub block_timestamp_last: u32,
    pub price0_cumulative_last: U256,
    pub price1_cumulative_last: U256,
    pub k_last: U256,
}

#[derive(Debug, Clone)]
pub struct V3PoolState {
    pub sqrt_price_x96: U256,
    pub tick: i32,
    pub observation_index: u16,
    pub observation_cardinality: u16,
    pub observation_cardinality_next: u16,
    pub fee_protocol: u8,
    pub unlocked: bool,
    pub liquidity: U256,
    pub fee_growth_global0_x128: U256,
    pub fee_growth_global1_x128: U256,
}

#[derive(Debug, Clone)]
pub struct FarmInfo {
    pub pid: u64,
    pub lp_token: Address,
    pub alloc_point: U256,
    pub last_reward_block: U256,
    pub acc_cake_per_share: U256,
    pub total_lp: U256,
    pub reward_per_block: U256,
}

#[derive(Debug, Clone)]
pub struct UserFarmInfo {
    pub pid: u64,
    pub amount: U256,
    pub reward_debt: U256,
    pub pending_rewards: U256,
}

#[derive(Debug)]
pub enum PancakeSwapError {
    ContractError(String),
    TransactionError(String),
    CalculationError(String),
    InvalidInput(String),
    InsufficientLiquidity(String),
    SlippageExceeded(String),
    EventParsingError(String),
}

#[derive(Debug, Clone)]
pub struct SwapPath {
    pub path: Vec<Address>,
    pub version: PoolVersion,
    pub expected_amount: U256,
}

#[derive(Debug, Clone)]
pub enum PoolVersion {
    V2,
    V3,
    Auto,
}

#[derive(Debug, Clone)]
pub struct PriceComparison {
    pub v2: Option<PriceInfo>,
    pub v3: Option<PriceInfo>,
    pub best: PriceSource,
}

#[derive(Debug, Clone)]
pub enum PriceSource {
    V2,
    V3,
}

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub address: Address,
    pub token0: Address,
    pub token1: Address,
    pub version: PoolVersion,
    pub liquidity: f64,
    pub volume_24h: f64,
    pub fee_tier: f64,
}

#[derive(Debug, Clone)]
pub struct LargeSwapEvent {
    pub swap_event: crate::types::SwapEvent,
    pub estimated_value_usd: f64,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct NewPairEvent {
    pub pair_event: crate::types::PairCreatedEvent,
    pub created_at: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct CrossVersionArbitrage {
    pub token_in: Address,
    pub token_out: Address,
    pub buy_version: PoolVersion,
    pub sell_version: PoolVersion,
    pub price_difference: f64,
    pub expected_profit_percentage: f64,
    pub test_amount: U256,
    pub v2_price: U256,
    pub v3_price: U256,
}

#[derive(Debug, Clone)]
pub struct SandwichOpportunity {
    pub target_tx_hash: ethers::types::H256,
    pub token: Address,
    pub expected_profit_eth: f64,
    pub risk_level: RiskLevel,
    pub required_gas: U256,
}

#[derive(Debug, Clone)]
pub struct TokenPrice {
    pub price: f64,
    pub source: PriceSource,
    pub timestamp: std::time::SystemTime,
    pub liquidity: f64,
}

#[derive(Debug, Clone)]
pub struct PriceCandle {
    pub timestamp: std::time::SystemTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub struct SwapQuote {
    pub amount_out: U256,
    pub path: Vec<Address>,
    pub gas_estimate: U256,
    pub price_impact: f64,
}

#[derive(Debug, Clone)]
pub struct PriceAlert {
    pub token_in: Address,
    pub token_out: Address,
    pub old_price: f64,
    pub new_price: f64,
    pub change_percentage: f64,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct AutoSwapResult {
    pub tx_hash: ethers::types::H256,
    pub version: PoolVersion,
    pub expected_amount_out: U256,
    pub price_comparison: PriceComparison,
}

#[derive(Debug, Clone)]
pub struct PendingSwap {
    pub hash: ethers::types::H256,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub min_amount_out: U256,
}

#[derive(Debug, Clone)]
pub enum Timeframe {
    Minute1,
    Minute5,
    Minute15,
    Hour1,
    Hour4,
    Day1,
}

impl Timeframe {
    pub fn seconds(&self) -> u64 {
        match self {
            Timeframe::Minute1 => 60,
            Timeframe::Minute5 => 300,
            Timeframe::Minute15 => 900,
            Timeframe::Hour1 => 3600,
            Timeframe::Hour4 => 14400,
            Timeframe::Day1 => 86400,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}
