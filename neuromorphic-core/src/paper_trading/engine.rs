//! Paper trading engine

use super::{
    position_manager::{PositionManager, Position, PositionStatistics},
    order_manager::{OrderManager, Order, OrderEvent, OrderType, SlippageModel},
    risk_manager::{RiskManager, RiskLimits, RiskCheckResult, RiskMetrics},
};
use crate::exchanges::{Symbol, Exchange, Side};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

/// Trading signal from neuromorphic system
#[derive(Clone, Debug)]
pub struct TradingSignal {
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub action: SignalAction,
    pub confidence: f64,
    pub urgency: f64,
    pub metadata: SignalMetadata,
}

/// Signal action
#[derive(Clone, Debug)]
pub enum SignalAction {
    Buy { size_hint: Option<f64> },
    Sell { size_hint: Option<f64> },
    Close { position_id: Option<String> },
    Hold,
}

/// Signal metadata
#[derive(Clone, Debug, Default)]
pub struct SignalMetadata {
    pub spike_count: u64,
    pub pattern_strength: f64,
    pub market_regime: String,
    pub volatility: f64,
}

/// Paper trading configuration
pub struct PaperTradingConfig {
    pub initial_capital: f64,
    pub commission_rate: f64,
    pub slippage_model: SlippageModel,
    pub risk_limits: RiskLimits,
    pub enable_stop_loss: bool,
    pub enable_take_profit: bool,
    pub update_interval: Duration,
}

impl Default for PaperTradingConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100000.0,
            commission_rate: 0.1, // 0.1%
            slippage_model: SlippageModel::Percentage(0.01), // 0.01%
            risk_limits: RiskLimits::default(),
            enable_stop_loss: true,
            enable_take_profit: true,
            update_interval: Duration::from_millis(100),
        }
    }
}

/// Paper trading statistics
#[derive(Default, Clone, Debug)]
pub struct TradingStatistics {
    pub capital: f64,
    pub total_pnl: f64,
    pub total_return_pct: f64,
    pub position_stats: PositionStatistics,
    pub risk_metrics: RiskMetrics,
    pub signals_processed: u64,
    pub signals_executed: u64,
}

/// Paper trading engine
pub struct PaperTradingEngine {
    position_manager: Arc<PositionManager>,
    order_manager: Arc<OrderManager>,
    risk_manager: Arc<RiskManager>,
    config: PaperTradingConfig,
    current_capital: Arc<parking_lot::RwLock<f64>>,
    current_prices: Arc<DashMap<Symbol, f64>>,
    signal_sender: mpsc::UnboundedSender<TradingSignal>,
    signal_receiver: Option<mpsc::UnboundedReceiver<TradingSignal>>,
    statistics: Arc<parking_lot::RwLock<TradingStatistics>>,
    running: Arc<tokio::sync::RwLock<bool>>,
    returns_history: Arc<parking_lot::RwLock<Vec<f64>>>,
}

impl PaperTradingEngine {
    pub fn new(config: PaperTradingConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let initial_capital = config.initial_capital;
        let commission_rate = config.commission_rate;
        let slippage_model = config.slippage_model.clone();
        let risk_limits = config.risk_limits.clone();
        
        let mut stats = TradingStatistics::default();
        stats.capital = initial_capital;
        
        Self {
            position_manager: Arc::new(PositionManager::new()),
            order_manager: Arc::new(OrderManager::new(commission_rate, slippage_model)),
            risk_manager: Arc::new(RiskManager::new(risk_limits, initial_capital)),
            config,
            current_capital: Arc::new(parking_lot::RwLock::new(initial_capital)),
            current_prices: Arc::new(DashMap::new()),
            signal_sender: tx,
            signal_receiver: Some(rx),
            statistics: Arc::new(parking_lot::RwLock::new(stats)),
            running: Arc::new(tokio::sync::RwLock::new(false)),
            returns_history: Arc::new(parking_lot::RwLock::new(Vec::new())),
        }
    }
    
    /// Start the trading engine
    pub async fn start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        drop(running);
        
        // Start signal processing
        self.spawn_signal_processor().await?;
        
        // Start order processing
        self.spawn_order_processor().await?;
        
        // Start statistics updater
        self.spawn_statistics_updater().await?;
        
        Ok(())
    }
    
    /// Stop the trading engine
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }
    
    /// Process trading signal
    pub async fn process_signal(&self, signal: TradingSignal) -> Result<()> {
        self.signal_sender.send(signal)?;
        Ok(())
    }
    
    /// Update market price
    pub fn update_price(&self, symbol: Symbol, price: f64) {
        self.current_prices.insert(symbol, price);
        self.position_manager.update_prices(&self.current_prices);
    }
    
    /// Spawn signal processor task
    async fn spawn_signal_processor(&mut self) -> Result<()> {
        let mut receiver = self.signal_receiver
            .take()
            .ok_or_else(|| anyhow::anyhow!("Signal receiver already taken"))?;
        
        let position_manager = self.position_manager.clone();
        let order_manager = self.order_manager.clone();
        let risk_manager = self.risk_manager.clone();
        let current_capital = self.current_capital.clone();
        let current_prices = self.current_prices.clone();
        let statistics = self.statistics.clone();
        let running = self.running.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            while *running.read().await {
                tokio::select! {
                    Some(signal) = receiver.recv() => {
                        // Update statistics
                        statistics.write().signals_processed += 1;
                        
                        // Process signal based on action
                        match signal.action {
                            SignalAction::Buy { size_hint } => {
                                if let Err(e) = Self::handle_buy_signal(
                                    &signal,
                                    size_hint,
                                    &position_manager,
                                    &order_manager,
                                    &risk_manager,
                                    &current_capital,
                                    &current_prices,
                                    &statistics,
                                    &config,
                                ).await {
                                    eprintln!("Error handling buy signal: {}", e);
                                }
                            }
                            SignalAction::Sell { size_hint } => {
                                if let Err(e) = Self::handle_sell_signal(
                                    &signal,
                                    size_hint,
                                    &position_manager,
                                    &order_manager,
                                    &risk_manager,
                                    &current_capital,
                                    &current_prices,
                                    &statistics,
                                    &config,
                                ).await {
                                    eprintln!("Error handling sell signal: {}", e);
                                }
                            }
                            SignalAction::Close { position_id } => {
                                if let Err(e) = Self::handle_close_signal(
                                    &signal,
                                    position_id,
                                    &position_manager,
                                    &order_manager,
                                    &current_prices,
                                    &statistics,
                                ).await {
                                    eprintln!("Error handling close signal: {}", e);
                                }
                            }
                            SignalAction::Hold => {
                                // No action needed
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {
                        // Continue loop
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Handle buy signal
    async fn handle_buy_signal(
        signal: &TradingSignal,
        size_hint: Option<f64>,
        position_manager: &Arc<PositionManager>,
        order_manager: &Arc<OrderManager>,
        risk_manager: &Arc<RiskManager>,
        current_capital: &Arc<parking_lot::RwLock<f64>>,
        current_prices: &Arc<DashMap<Symbol, f64>>,
        statistics: &Arc<parking_lot::RwLock<TradingStatistics>>,
        config: &PaperTradingConfig,
    ) -> Result<()> {
        let capital = *current_capital.read();
        let price = current_prices
            .get(&signal.symbol)
            .map(|p| *p)
            .ok_or_else(|| anyhow::anyhow!("No price for {}", signal.symbol))?;
        
        // Calculate position size
        let position_size = if let Some(hint) = size_hint {
            hint
        } else {
            risk_manager.calculate_position_size(&signal.symbol, capital, signal.confidence)
        };
        
        let quantity = position_size / price;
        
        // Risk check
        match risk_manager.check_order(&signal.symbol, Side::Buy, quantity, price, capital) {
            RiskCheckResult::Approved => {},
            RiskCheckResult::Rejected { reason } => {
                println!("Order rejected: {}", reason);
                return Ok(());
            }
            RiskCheckResult::Warning { message } => {
                println!("Risk warning: {}", message);
            }
        }
        
        // Create order
        let order = if signal.urgency > 0.8 {
            Order::market(signal.symbol.clone(), signal.exchange, Side::Buy, quantity)
        } else {
            Order::limit(signal.symbol.clone(), signal.exchange, Side::Buy, quantity, price * 0.999)
        };
        
        // Submit order
        let order_id = order_manager.submit_order(order)?;
        risk_manager.record_order();
        
        // Create stop loss and take profit if enabled
        if config.enable_stop_loss || config.enable_take_profit {
            let stop_price = price * (1.0 - config.risk_limits.stop_loss_pct / 100.0);
            let tp_price = price * (1.0 + config.risk_limits.take_profit_pct / 100.0);
            
            if config.enable_stop_loss && config.enable_take_profit {
                order_manager.create_bracket_order(
                    signal.symbol.clone(),
                    signal.exchange,
                    Side::Buy,
                    quantity,
                    None,
                    stop_price,
                    tp_price,
                )?;
            }
        }
        
        statistics.write().signals_executed += 1;
        
        Ok(())
    }
    
    /// Handle sell signal
    async fn handle_sell_signal(
        signal: &TradingSignal,
        size_hint: Option<f64>,
        position_manager: &Arc<PositionManager>,
        order_manager: &Arc<OrderManager>,
        risk_manager: &Arc<RiskManager>,
        current_capital: &Arc<parking_lot::RwLock<f64>>,
        current_prices: &Arc<DashMap<Symbol, f64>>,
        statistics: &Arc<parking_lot::RwLock<TradingStatistics>>,
        config: &PaperTradingConfig,
    ) -> Result<()> {
        let capital = *current_capital.read();
        let price = current_prices
            .get(&signal.symbol)
            .map(|p| *p)
            .ok_or_else(|| anyhow::anyhow!("No price for {}", signal.symbol))?;
        
        // Check if we have a position to sell
        let net_position = position_manager.get_net_position(&signal.symbol);
        
        let quantity = if net_position > 0.0 {
            // Close long position
            net_position.min(size_hint.unwrap_or(net_position))
        } else {
            // Open short position
            let position_size = if let Some(hint) = size_hint {
                hint
            } else {
                risk_manager.calculate_position_size(&signal.symbol, capital, signal.confidence)
            };
            position_size / price
        };
        
        // Risk check
        match risk_manager.check_order(&signal.symbol, Side::Sell, quantity, price, capital) {
            RiskCheckResult::Approved => {},
            RiskCheckResult::Rejected { reason } => {
                println!("Order rejected: {}", reason);
                return Ok(());
            }
            RiskCheckResult::Warning { message } => {
                println!("Risk warning: {}", message);
            }
        }
        
        // Create order
        let order = if signal.urgency > 0.8 {
            Order::market(signal.symbol.clone(), signal.exchange, Side::Sell, quantity)
        } else {
            Order::limit(signal.symbol.clone(), signal.exchange, Side::Sell, quantity, price * 1.001)
        };
        
        // Submit order
        order_manager.submit_order(order)?;
        risk_manager.record_order();
        
        statistics.write().signals_executed += 1;
        
        Ok(())
    }
    
    /// Handle close signal
    async fn handle_close_signal(
        signal: &TradingSignal,
        position_id: Option<String>,
        position_manager: &Arc<PositionManager>,
        order_manager: &Arc<OrderManager>,
        current_prices: &Arc<DashMap<Symbol, f64>>,
        statistics: &Arc<parking_lot::RwLock<TradingStatistics>>,
    ) -> Result<()> {
        let price = current_prices
            .get(&signal.symbol)
            .map(|p| *p)
            .ok_or_else(|| anyhow::anyhow!("No price for {}", signal.symbol))?;
        
        if let Some(id) = position_id {
            // Close specific position
            if let Some(position) = position_manager.get_position(&id) {
                let side = match position.side {
                    Side::Buy => Side::Sell,
                    Side::Sell => Side::Buy,
                };
                
                let order = Order::market(
                    position.symbol,
                    position.exchange,
                    side,
                    position.quantity
                );
                
                order_manager.submit_order(order)?;
            }
        } else {
            // Close all positions for symbol
            let positions = position_manager.get_open_positions_by_symbol(&signal.symbol);
            
            for position in positions {
                let side = match position.side {
                    Side::Buy => Side::Sell,
                    Side::Sell => Side::Buy,
                };
                
                let order = Order::market(
                    position.symbol,
                    position.exchange,
                    side,
                    position.quantity
                );
                
                order_manager.submit_order(order)?;
            }
        }
        
        statistics.write().signals_executed += 1;
        
        Ok(())
    }
    
    /// Spawn order processor task
    async fn spawn_order_processor(&self) -> Result<()> {
        let order_manager = self.order_manager.clone();
        let position_manager = self.position_manager.clone();
        let current_prices = self.current_prices.clone();
        let current_capital = self.current_capital.clone();
        let running = self.running.clone();
        let update_interval = self.config.update_interval;
        
        tokio::spawn(async move {
            while *running.read().await {
                // Process pending orders
                if let Ok(filled_orders) = order_manager.process_orders(&current_prices) {
                    for order_id in filled_orders {
                        if let Some(order) = order_manager.get_order(&order_id) {
                            // Update positions
                            match order.side {
                                Side::Buy => {
                                    position_manager.open_position(
                                        order.symbol,
                                        order.exchange,
                                        order.side,
                                        order.filled_quantity,
                                        order.avg_fill_price,
                                        order.commission,
                                        order.slippage,
                                    ).ok();
                                }
                                Side::Sell => {
                                    // Check if closing existing position
                                    let positions = position_manager.get_open_positions_by_symbol(&order.symbol);
                                    if !positions.is_empty() {
                                        // Close position
                                        for pos in positions {
                                            if pos.side == Side::Buy {
                                                position_manager.close_position(
                                                    &pos.id,
                                                    order.avg_fill_price,
                                                    order.commission,
                                                    order.slippage,
                                                ).ok();
                                                break;
                                            }
                                        }
                                    } else {
                                        // Open short position
                                        position_manager.open_position(
                                            order.symbol,
                                            order.exchange,
                                            order.side,
                                            order.filled_quantity,
                                            order.avg_fill_price,
                                            order.commission,
                                            order.slippage,
                                        ).ok();
                                    }
                                }
                            }
                            
                            // Update capital
                            let mut capital = current_capital.write();
                            *capital -= order.commission + order.slippage;
                        }
                    }
                }
                
                tokio::time::sleep(update_interval).await;
            }
        });
        
        Ok(())
    }
    
    /// Spawn statistics updater task
    async fn spawn_statistics_updater(&self) -> Result<()> {
        let position_manager = self.position_manager.clone();
        let risk_manager = self.risk_manager.clone();
        let current_capital = self.current_capital.clone();
        let current_prices = self.current_prices.clone();
        let statistics = self.statistics.clone();
        let returns_history = self.returns_history.clone();
        let running = self.running.clone();
        let initial_capital = self.config.initial_capital;
        
        tokio::spawn(async move {
            let mut last_capital = initial_capital;
            
            while *running.read().await {
                // Update position prices
                position_manager.update_prices(&current_prices);
                
                // Get position statistics
                let pos_stats = position_manager.get_statistics();
                
                // Calculate current capital
                let realized_pnl = pos_stats.total_realized_pnl;
                let unrealized_pnl = pos_stats.total_unrealized_pnl;
                let total_pnl = realized_pnl + unrealized_pnl;
                let current_cap = initial_capital + total_pnl;
                
                // Calculate return
                let return_pct = if last_capital > 0.0 {
                    (current_cap - last_capital) / last_capital
                } else {
                    0.0
                };
                
                // Update returns history
                {
                    let mut returns = returns_history.write();
                    returns.push(return_pct);
                    if returns.len() > 1000 {
                        returns.remove(0);
                    }
                }
                
                // Calculate total exposure
                let positions = position_manager.get_open_positions();
                let total_exposure: f64 = positions.iter()
                    .map(|p| p.quantity * current_prices.get(&p.symbol).map(|pr| *pr).unwrap_or(0.0))
                    .sum();
                
                // Update risk metrics
                let returns_copy = returns_history.read().clone();
                risk_manager.update_metrics(
                    current_cap,
                    total_exposure,
                    realized_pnl,
                    &returns_copy
                );
                
                // Update Kelly parameters if we have enough data
                if pos_stats.winning_positions + pos_stats.losing_positions > 20 {
                    risk_manager.update_kelly_parameters(
                        pos_stats.win_rate / 100.0,
                        pos_stats.avg_win,
                        pos_stats.avg_loss
                    );
                }
                
                // Update statistics
                let mut stats = statistics.write();
                stats.capital = current_cap;
                stats.total_pnl = total_pnl;
                stats.total_return_pct = ((current_cap - initial_capital) / initial_capital) * 100.0;
                stats.position_stats = pos_stats;
                stats.risk_metrics = risk_manager.get_metrics();
                
                // Update current capital
                *current_capital.write() = current_cap;
                last_capital = current_cap;
                
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
        
        Ok(())
    }
    
    /// Get current statistics
    pub fn get_statistics(&self) -> TradingStatistics {
        self.statistics.read().clone()
    }
    
    /// Get position manager
    pub fn position_manager(&self) -> &Arc<PositionManager> {
        &self.position_manager
    }
    
    /// Get order manager
    pub fn order_manager(&self) -> &Arc<OrderManager> {
        &self.order_manager
    }
    
    /// Get risk manager
    pub fn risk_manager(&self) -> &Arc<RiskManager> {
        &self.risk_manager
    }
}

/// Clone implementation for config
impl Clone for PaperTradingConfig {
    fn clone(&self) -> Self {
        Self {
            initial_capital: self.initial_capital,
            commission_rate: self.commission_rate,
            slippage_model: self.slippage_model.clone(),
            risk_limits: self.risk_limits.clone(),
            enable_stop_loss: self.enable_stop_loss,
            enable_take_profit: self.enable_take_profit,
            update_interval: self.update_interval,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_paper_trading_engine() {
        let config = PaperTradingConfig::default();
        let mut engine = PaperTradingEngine::new(config);
        
        engine.start().await.unwrap();
        
        // Update price
        engine.update_price(Symbol::new("BTC-USD"), 50000.0);
        
        // Send buy signal
        let signal = TradingSignal {
            symbol: Symbol::new("BTC-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Buy { size_hint: Some(5000.0) },
            confidence: 0.8,
            urgency: 0.9,
            metadata: SignalMetadata::default(),
        };
        
        engine.process_signal(signal).await.unwrap();
        
        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check statistics
        let stats = engine.get_statistics();
        assert_eq!(stats.signals_processed, 1);
        
        engine.stop().await.unwrap();
    }
}