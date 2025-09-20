//! Neuromorphic Paper Trading Library
//! 
//! A standalone paper trading system designed to work with neuromorphic
//! prediction engines. Can be used as a library or standalone application.

pub mod paper_trading;
pub mod exchanges;
pub mod metrics;
pub mod api;

// Re-export main types for easy access
pub use paper_trading::{
    PaperTradingEngine, PaperTradingConfig, TradingSignal, SignalAction, 
    SignalMetadata, TradingStatistics, PositionManager, OrderManager, RiskManager
};
pub use exchanges::{Symbol, Exchange, Side, OrderType};
pub use metrics::MetricsCollector;
pub use api::MetricsApiServer;

use anyhow::Result;
use std::sync::Arc;

/// Main interface for integrating with external prediction engines
pub struct NeuromorphicPaperTrader {
    engine: PaperTradingEngine,
    metrics_collector: Arc<MetricsCollector>,
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