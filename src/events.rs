use crate::tool::event_parsers::{
    parse_burn_log, parse_mint_log, parse_pair_created_log, parse_swap_log, parse_v3_burn_log,
    parse_v3_mint_log, parse_v3_swap_log,
};
use crate::types::{
    BurnEvent, MintEvent, PairCreatedEvent, SwapEvent, V3BurnEvent, V3MintEvent, V3SwapEvent,
};
use crate::{EvmClient, EvmError};
use ethers::providers::Middleware;
use ethers::types::Address;
use ethers::types::{Filter, ValueOrArray};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use tokio::time::{Duration, MissedTickBehavior, interval};

/// Configuration for event listener behavior
#[derive(Debug, Clone)]
pub struct EventListenerConfig {
    pub poll_interval_secs: u64,
    pub max_blocks_per_poll: u64,
    pub confirmation_blocks: u64,
}

impl Default for EventListenerConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 3,
            max_blocks_per_poll: 2000,
            confirmation_blocks: 1,
        }
    }
}

struct EventListenerState {
    last_block_number: AtomicU64,
    is_running: AtomicBool,
}

/// Event listener for PancakeSwap V2 and V3 events
pub struct PancakeSwapEventListener {
    client: Arc<EvmClient>,
    config: EventListenerConfig,
    state: Arc<EventListenerState>,
}

impl PancakeSwapEventListener {
    /// Creates a new event listener with default configuration
    pub fn new(client: Arc<EvmClient>) -> Self {
        Self {
            client,
            config: EventListenerConfig::default(),
            state: Arc::new(EventListenerState {
                last_block_number: AtomicU64::new(0),
                is_running: AtomicBool::new(false),
            }),
        }
    }

    /// Creates a new event listener with custom configuration
    pub fn with_config(client: Arc<EvmClient>, config: EventListenerConfig) -> Self {
        Self {
            client,
            config,
            state: Arc::new(EventListenerState {
                last_block_number: AtomicU64::new(0),
                is_running: AtomicBool::new(false),
            }),
        }
    }

    /// Starts listening for Swap events from V2 pairs
    ///
    /// # Example
    /// ```no_run
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    ///
    /// let listener = PancakeSwapEventListener::new(client);
    /// let pair_address = Address::from_str("0x...").unwrap();
    ///
    /// listener.start_swap_listener(
    ///     vec![pair_address],
    ///     |swap_event| {
    ///         println!("Swap detected: {:?}", swap_event);
    ///     }
    /// ).await.unwrap();
    /// ```
    pub async fn start_swap_listener(
        &self,
        pair_addresses: Vec<Address>,
        on_swap: impl Fn(SwapEvent) + Send + Sync + 'static,
    ) -> Result<(), EvmError> {
        self.start_listener(pair_addresses, "Swap".to_string(), move |log| {
            if let Ok(swap_event) = parse_swap_log(&log) {
                on_swap(swap_event);
            }
        })
        .await
    }

    // Starts listening for PairCreated events from factory contracts
    ///
    /// # Example
    /// ```no_run
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    ///
    /// let listener = PancakeSwapEventListener::new(client);
    /// let factory_address = Address::from_str("0x...").unwrap();
    ///
    /// listener.start_pair_created_listener(
    ///     vec![factory_address],
    ///     |pair_event| {
    ///         println!("New pair created: {:?}", pair_event);
    ///     }
    /// ).await.unwrap();
    /// ```
    pub async fn start_pair_created_listener(
        &self,
        factory_addresses: Vec<Address>,
        on_pair_created: impl Fn(PairCreatedEvent) + Send + Sync + 'static,
    ) -> Result<(), EvmError> {
        self.start_listener(factory_addresses, "PairCreated".to_string(), move |log| {
            if let Ok(pair_event) = parse_pair_created_log(&log) {
                on_pair_created(pair_event);
            }
        })
        .await
    }

    /// Starts listening for Mint events from V2 pairs
    pub async fn start_mint_listener(
        &self,
        pair_addresses: Vec<Address>,
        on_mint: impl Fn(MintEvent) + Send + Sync + 'static,
    ) -> Result<(), EvmError> {
        self.start_listener(pair_addresses, "Mint".to_string(), move |log| {
            if let Ok(mint_event) = parse_mint_log(&log) {
                on_mint(mint_event);
            }
        })
        .await
    }

    /// Starts listening for Burn events from V2 pairs
    pub async fn start_burn_listener(
        &self,
        pair_addresses: Vec<Address>,
        on_burn: impl Fn(BurnEvent) + Send + Sync + 'static,
    ) -> Result<(), EvmError> {
        self.start_listener(pair_addresses, "Burn".to_string(), move |log| {
            if let Ok(burn_event) = parse_burn_log(&log) {
                on_burn(burn_event);
            }
        })
        .await
    }

    /// Starts listening for Swap events from V3 pools
    ///
    /// # Example
    /// ```no_run
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    ///
    /// let listener = PancakeSwapEventListener::new(client);
    /// let pool_address = Address::from_str("0x...").unwrap();
    ///
    /// listener.start_v3_swap_listener(
    ///     vec![pool_address],
    ///     |swap_event| {
    ///         println!("V3 Swap detected: {:?}", swap_event);
    ///     }
    /// ).await.unwrap();
    /// ```
    pub async fn start_v3_swap_listener(
        &self,
        pool_addresses: Vec<Address>,
        on_swap: impl Fn(V3SwapEvent) + Send + Sync + 'static,
    ) -> Result<(), EvmError> {
        self.start_listener(pool_addresses, "Swap".to_string(), move |log| {
            if let Ok(swap_event) = parse_v3_swap_log(&log) {
                on_swap(swap_event);
            }
        })
        .await
    }

    /// Starts listening for Mint events from V3 pools
    pub async fn start_v3_mint_listener(
        &self,
        pool_addresses: Vec<Address>,
        on_mint: impl Fn(V3MintEvent) + Send + Sync + 'static,
    ) -> Result<(), EvmError> {
        self.start_listener(pool_addresses, "Mint".to_string(), move |log| {
            if let Ok(mint_event) = parse_v3_mint_log(&log) {
                on_mint(mint_event);
            }
        })
        .await
    }

    /// Starts listening for Burn events from V3 pools
    pub async fn start_v3_burn_listener(
        &self,
        pool_addresses: Vec<Address>,
        on_burn: impl Fn(V3BurnEvent) + Send + Sync + 'static,
    ) -> Result<(), EvmError> {
        self.start_listener(pool_addresses, "Burn".to_string(), move |log| {
            if let Ok(burn_event) = parse_v3_burn_log(&log) {
                on_burn(burn_event);
            }
        })
        .await
    }

    /// Internal method to start a generic event listener
    async fn start_listener(
        &self,
        addresses: Vec<Address>,
        event_name: String,
        on_event: impl Fn(ethers::types::Log) + Send + Sync + 'static,
    ) -> Result<(), EvmError> {
        if self.state.is_running.load(Ordering::SeqCst) {
            return Err(EvmError::ListenerError(
                "Listener already running".to_string(),
            ));
        }
        self.state.is_running.store(true, Ordering::SeqCst);
        let client = self.client.clone();
        let config = self.config.clone();
        let state = self.state.clone();
        let current_block =
            client.provider.get_block_number().await.map_err(|e| {
                EvmError::ProviderError(format!("Failed to get block number: {}", e))
            })?;

        state.last_block_number.store(
            current_block.as_u64() - config.confirmation_blocks,
            Ordering::SeqCst,
        );

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.poll_interval_secs));
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

            while state.is_running.load(Ordering::SeqCst) {
                if let Err(e) =
                    Self::poll_events(&client, &state, &config, &addresses, &event_name, &on_event)
                        .await
                {
                    eprintln!("Error polling events: {}", e);
                }

                interval.tick().await;
            }
        });

        Ok(())
    }

    /// Stops the event listener
    pub fn stop_listener(&self) {
        self.state.is_running.store(false, Ordering::SeqCst);
    }

    /// Polls for new events in a range of blocks
    async fn poll_events(
        client: &EvmClient,
        state: &EventListenerState,
        config: &EventListenerConfig,
        addresses: &[Address],
        event_name: &str,
        on_event: &impl Fn(ethers::types::Log),
    ) -> Result<(), EvmError> {
        let from_block = state.last_block_number.load(Ordering::SeqCst) + 1;
        let current_block =
            client.provider.get_block_number().await.map_err(|e| {
                EvmError::ProviderError(format!("Failed to get block number: {}", e))
            })?;

        let to_block = std::cmp::min(
            current_block.as_u64() - config.confirmation_blocks,
            from_block + config.max_blocks_per_poll - 1,
        );

        if from_block > to_block {
            return Ok(());
        }

        let filter = Filter::new()
            .from_block(from_block)
            .to_block(to_block)
            .address(ValueOrArray::Array(addresses.to_vec()))
            .event(event_name);

        let logs = client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::ProviderError(format!("Failed to get logs: {}", e)))?;

        for log in logs {
            on_event(log);
        }

        state.last_block_number.store(to_block, Ordering::SeqCst);

        Ok(())
    }
}
