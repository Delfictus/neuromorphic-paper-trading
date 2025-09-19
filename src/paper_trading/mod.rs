//! Paper trading simulation system

pub mod position_manager;
pub mod order_manager;
pub mod risk_manager;
pub mod engine;

pub use position_manager::{PositionManager, Position, PositionStatus, PositionStatistics};
pub use order_manager::{
    OrderManager, Order, OrderType, OrderStatus, OrderEvent, 
    TimeInForce, SlippageModel
};
pub use risk_manager::{
    RiskManager, RiskLimits, RiskMetrics, RiskCheckResult,
    KellyCriterion, PortfolioHeatMap
};
pub use engine::{
    PaperTradingEngine, PaperTradingConfig, TradingSignal, 
    SignalAction, SignalMetadata, TradingStatistics
};