//! Neuromorphic Paper Trading Library
//! 
//! A standalone paper trading system designed to work with neuromorphic
//! prediction engines. Can be used as a library or standalone application.

pub mod paper_trading;
pub mod exchanges;
pub mod metrics;
pub mod api;
pub mod market_scanner;

// Re-export main types for easy access
pub use paper_trading::{
    PaperTradingEngine, PaperTradingConfig, TradingSignal, SignalAction, 
    SignalMetadata, TradingStatistics, PositionManager, OrderManager, RiskManager
};
pub use exchanges::{Symbol, Exchange, Side, OrderType};
pub use metrics::MetricsCollector;
pub use api::MetricsApiServer;
pub use market_scanner::{
    MarketScannerService, MarketData, TradingOpportunity, ScannerConfig,
    StockScreener, StrategyEngine, MarketAnalytics
};

use anyhow::Result;
use std::sync::Arc;

/// Main interface for integrating with external prediction engines
pub struct NeuromorphicPaperTrader {
    engine: PaperTradingEngine,
    metrics_collector: Arc<MetricsCollector>,
}

/// Autonomous trading system that continuously monitors and trades the market
pub struct AutonomousTradingSystem {
    paper_trader: NeuromorphicPaperTrader,
    market_scanner: MarketScannerService,
    config: AutonomousConfig,
}

#[derive(Debug, Clone)]
pub struct AutonomousConfig {
    pub scanner_config: ScannerConfig,
    pub trading_config: PaperTradingConfig,
    pub max_positions: usize,
    pub max_daily_trades: usize,
    pub risk_per_trade: f64,
    pub enable_auto_trading: bool,
    pub min_opportunity_confidence: f64,
    pub portfolio_heat: f64,
}

impl NeuromorphicPaperTrader {
    /// Create a new paper trader with configuration
    pub fn new(config: PaperTradingConfig) -> Self {
        let metrics_collector = Arc::new(MetricsCollector::new());
        Self {
            engine: PaperTradingEngine::new(config),
            metrics_collector,
        }
    }

    /// Start the paper trading engine
    pub async fn start(&mut self) -> Result<()> {
        self.engine.start().await
    }

    /// Stop the paper trading engine
    pub async fn stop(&self) -> Result<()> {
        self.engine.stop().await
    }

    /// Process a trading signal from an external prediction engine
    pub async fn process_prediction_signal(&self, signal: TradingSignal) -> Result<()> {
        // Record signal metrics
        self.metrics_collector.record_signal(&signal);
        
        // Process the signal
        let result = self.engine.process_signal(signal).await;
        
        // Update portfolio metrics after processing
        let stats = self.engine.get_statistics();
        self.metrics_collector.update_portfolio_metrics(&stats);
        
        result
    }

    /// Update market price for a symbol
    pub fn update_market_price(&self, symbol: Symbol, price: f64) {
        self.engine.update_price(symbol.clone(), price);
        
        // Update market data metrics
        self.metrics_collector.update_market_data(symbol, price);
    }

    /// Get current trading statistics
    pub fn get_statistics(&self) -> TradingStatistics {
        self.engine.get_statistics()
    }

    /// Get access to position manager for detailed position info
    pub fn positions(&self) -> &std::sync::Arc<PositionManager> {
        self.engine.position_manager()
    }

    /// Get access to risk manager for risk metrics
    pub fn risk_manager(&self) -> &std::sync::Arc<RiskManager> {
        self.engine.risk_manager()
    }

    /// Get access to metrics collector for Grafana integration
    pub fn metrics_collector(&self) -> &Arc<MetricsCollector> {
        &self.metrics_collector
    }

    /// Start Grafana metrics API server
    pub async fn start_metrics_api(&self, port: u16) {
        let api_server = MetricsApiServer::new(self.metrics_collector.clone(), port);
        tokio::spawn(async move {
            api_server.start().await;
        });
    }
}

impl Default for AutonomousConfig {
    fn default() -> Self {
        Self {
            scanner_config: ScannerConfig::default(),
            trading_config: PaperTradingConfig::default(),
            max_positions: 10,
            max_daily_trades: 50,
            risk_per_trade: 0.02,
            enable_auto_trading: true,
            min_opportunity_confidence: 0.75,
            portfolio_heat: 0.1,
        }
    }
}

impl AutonomousTradingSystem {
    /// Create a new autonomous trading system
    pub fn new(config: AutonomousConfig) -> Self {
        let paper_trader = NeuromorphicPaperTrader::new(config.trading_config.clone());
        let market_scanner = MarketScannerService::new(config.scanner_config.clone());

        Self {
            paper_trader,
            market_scanner,
            config,
        }
    }

    /// Start the autonomous trading system
    pub async fn start(&mut self) -> Result<()> {
        println!("🤖 Starting Autonomous Neuromorphic Trading System");
        
        self.paper_trader.start().await?;
        self.paper_trader.start_metrics_api(3002).await;
        
        let (market_stream, opportunity_stream) = self.market_scanner.start().await?;
        
        self.start_trading_loop(market_stream, opportunity_stream).await?;
        
        Ok(())
    }

    /// Start the main trading loop
    async fn start_trading_loop(
        &self,
        mut market_stream: tokio::sync::broadcast::Receiver<MarketData>,
        mut opportunity_stream: tokio::sync::broadcast::Receiver<TradingOpportunity>,
    ) -> Result<()> {
        let mut daily_trades = 0;
        let mut last_reset = chrono::Utc::now().date_naive();
        
        println!("📊 Market scanner started - monitoring {} exchanges", 
                 self.config.scanner_config.included_exchanges.len());
        println!("🎯 Auto-trading: {} | Min confidence: {:.0}%", 
                 if self.config.enable_auto_trading { "ENABLED" } else { "DISABLED" },
                 self.config.min_opportunity_confidence * 100.0);

        loop {
            tokio::select! {
                Ok(market_data) = market_stream.recv() => {
                    self.paper_trader.update_market_price(
                        market_data.symbol.clone(), 
                        market_data.price
                    );
                }
                
                Ok(opportunity) = opportunity_stream.recv() => {
                    let today = chrono::Utc::now().date_naive();
                    if today != last_reset {
                        daily_trades = 0;
                        last_reset = today;
                        println!("📅 Daily trade counter reset");
                    }

                    if self.should_execute_trade(&opportunity, daily_trades).await {
                        match self.execute_opportunity(&opportunity).await {
                            Ok(_) => {
                                daily_trades += 1;
                                println!("✅ Executed trade #{}: {} {} @ ${:.2} (confidence: {:.1}%)",
                                        daily_trades,
                                        opportunity.strategy,
                                        opportunity.symbol.as_str(),
                                        opportunity.entry_price,
                                        opportunity.confidence * 100.0);
                            }
                            Err(e) => {
                                println!("❌ Failed to execute trade: {}", e);
                            }
                        }
                    } else {
                        println!("⏭️  Skipped opportunity: {} {} (confidence: {:.1}%, reason: filtering)",
                                opportunity.symbol.as_str(),
                                opportunity.strategy,
                                opportunity.confidence * 100.0);
                    }
                }
                
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                    self.print_status().await;
                }
            }
        }
    }

    /// Determine if we should execute a trading opportunity
    async fn should_execute_trade(&self, opportunity: &TradingOpportunity, daily_trades: usize) -> bool {
        if !self.config.enable_auto_trading {
            return false;
        }

        if opportunity.confidence < self.config.min_opportunity_confidence {
            return false;
        }

        if daily_trades >= self.config.max_daily_trades {
            return false;
        }

        let stats = self.paper_trader.get_statistics();
        let current_positions = stats.position_stats.open_positions;
        
        if current_positions >= self.config.max_positions as u64 {
            return false;
        }

        let portfolio_risk = (current_positions as f64 * self.config.risk_per_trade).min(self.config.portfolio_heat);
        if portfolio_risk >= self.config.portfolio_heat {
            return false;
        }

        true
    }

    /// Execute a trading opportunity
    async fn execute_opportunity(&self, opportunity: &TradingOpportunity) -> Result<()> {
        let signal_action = match opportunity.expected_move {
            x if x > 0.0 => SignalAction::Buy { size_hint: Some(opportunity.position_size) },
            x if x < 0.0 => SignalAction::Sell { size_hint: Some(opportunity.position_size) },
            _ => SignalAction::Hold,
        };

        let signal = TradingSignal {
            symbol: opportunity.symbol.clone(),
            exchange: Exchange::NYSE, // Default exchange
            action: signal_action,
            confidence: opportunity.confidence,
            urgency: 0.8,
            metadata: SignalMetadata {
                spike_count: 100,
                pattern_strength: opportunity.confidence,
                volatility: opportunity.risk_score,
                market_regime: "autonomous".to_string(),
            },
        };

        self.paper_trader.process_prediction_signal(signal).await
    }

    /// Print current system status
    async fn print_status(&self) {
        let stats = self.paper_trader.get_statistics();
        let market_metrics = self.market_scanner.get_market_metrics().await
            .unwrap_or_else(|_| market_scanner::MarketMetrics {
                total_symbols_tracked: 0,
                opportunities_detected: 0,
                market_volatility: 0.0,
                sector_performance: std::collections::HashMap::new(),
                trending_symbols: Vec::new(),
                market_regime: market_scanner::MarketRegime::Consolidation,
                overall_sentiment: 0.0,
            });

        println!("\n📈 AUTONOMOUS TRADING STATUS");
        println!("💰 Portfolio: ${:.2} | P&L: {:.2}% | Positions: {}",
                stats.capital, stats.total_return_pct, stats.position_stats.open_positions);
        println!("📊 Symbols tracked: {} | Opportunities: {} | Market volatility: {:.1}%",
                market_metrics.total_symbols_tracked,
                market_metrics.opportunities_detected,
                market_metrics.market_volatility * 100.0);
        println!("🎯 Win rate: {:.1}% | Sharpe: {:.2} | Max drawdown: {:.1}%",
                stats.position_stats.win_rate, stats.risk_metrics.sharpe_ratio, stats.risk_metrics.max_drawdown);
        println!("🔄 Market regime: {:?} | Sentiment: {:.2}\n",
                market_metrics.market_regime, market_metrics.overall_sentiment);
    }

    /// Get top opportunities currently available
    pub async fn get_top_opportunities(&self, limit: usize) -> Result<Vec<TradingOpportunity>> {
        self.market_scanner.get_top_opportunities(limit).await
    }

    /// Get current market metrics
    pub async fn get_market_metrics(&self) -> Result<market_scanner::MarketMetrics> {
        self.market_scanner.get_market_metrics().await
    }

    /// Stop the autonomous trading system
    pub async fn stop(&self) -> Result<()> {
        println!("🛑 Stopping Autonomous Trading System");
        self.paper_trader.stop().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_paper_trader_creation() {
        let config = PaperTradingConfig::default();
        let trader = NeuromorphicPaperTrader::new(config);
        
        // Should be able to get initial statistics
        let stats = trader.get_statistics();
        assert_eq!(stats.capital, 100000.0); // Default initial capital
    }

    #[tokio::test]
    async fn test_paper_trader_lifecycle() {
        let config = PaperTradingConfig::default();
        let mut trader = NeuromorphicPaperTrader::new(config);
        
        // Should start and stop without error
        assert!(trader.start().await.is_ok());
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(trader.stop().await.is_ok());
    }
}