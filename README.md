# Neuromorphic Paper Trader

A standalone paper trading system extracted from the ARES-51 neuromorphic trading project. This system provides realistic trading simulation with support for external prediction engines.

## Features

- 📈 **Complete Paper Trading Simulation** - Virtual portfolio with realistic commission and slippage
- 🎯 **Signal Processing** - Process trading signals from any prediction engine
- 📊 **Advanced Analytics** - Comprehensive performance metrics and statistics
- 🛡️ **Risk Management** - Position sizing, stop losses, and risk controls
- ⚡ **High Performance** - Async processing with real-time updates
- 🔌 **Plugin Architecture** - Easy integration with external prediction systems

## Quick Start

### As a Standalone Application

```bash
# Clone and run
git clone <repository>
cd neuromorphic-paper-trader
cargo run
```

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
neuromorphic-paper-trader = { path = "../neuromorphic-paper-trader" }
```

Basic usage:

```rust
use neuromorphic_paper_trader::{
    NeuromorphicPaperTrader, PaperTradingConfig, TradingSignal, 
    SignalAction, Symbol, Exchange
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure paper trader
    let config = PaperTradingConfig {
        initial_capital: 100_000.0,
        commission_rate: 0.1, // 0.1%
        ..Default::default()
    };
    
    // Create and start trader
    let mut trader = NeuromorphicPaperTrader::new(config);
    trader.start().await?;
    
    // Update market prices
    trader.update_market_price(Symbol::new("BTC-USD"), 50000.0);
    
    // Send trading signal
    let signal = TradingSignal {
        symbol: Symbol::new("BTC-USD"),
        exchange: Exchange::Binance,
        action: SignalAction::Buy { size_hint: Some(1000.0) },
        confidence: 0.8,
        urgency: 0.9,
        metadata: Default::default(),
    };
    
    trader.process_prediction_signal(signal).await?;
    
    // Get statistics
    let stats = trader.get_statistics();
    println!("P&L: ${:.2}", stats.total_pnl);
    
    trader.stop().await?;
    Ok(())
}
```

## Integration with Prediction Engines

The system is designed to work with any prediction engine that can generate trading signals:

```rust
// Your prediction engine
struct MyPredictionEngine;

impl MyPredictionEngine {
    async fn generate_signal(&self, market_data: MarketData) -> Option<TradingSignal> {
        // Your prediction logic here
        Some(TradingSignal {
            symbol: Symbol::new("BTC-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Buy { size_hint: Some(1000.0) },
            confidence: 0.75,
            urgency: 0.5,
            metadata: SignalMetadata {
                pattern_strength: 0.8,
                volatility: 0.02,
                // ... other metadata
                ..Default::default()
            },
        })
    }
}

// Connect to paper trader
let engine = MyPredictionEngine;
let mut trader = NeuromorphicPaperTrader::new(config);

// Process predictions
if let Some(signal) = engine.generate_signal(market_data).await {
    trader.process_prediction_signal(signal).await?;
}
```

## Configuration

```rust
let config = PaperTradingConfig {
    initial_capital: 100_000.0,     // Starting capital
    commission_rate: 0.1,           // 0.1% commission
    slippage_model: SlippageModel::Percentage(0.01), // 0.01% slippage
    enable_stop_loss: true,         // Auto stop losses
    enable_take_profit: true,       // Auto take profits
    update_interval: Duration::from_millis(100), // Update frequency
    risk_limits: RiskLimits {
        max_position_size_pct: 10.0, // Max 10% per position
        max_daily_loss_pct: 5.0,     // Max 5% daily loss
        stop_loss_pct: 2.0,          // 2% stop loss
        take_profit_pct: 6.0,        // 6% take profit
    },
};
```

## Performance Metrics

The system tracks comprehensive trading metrics:

- **Portfolio Metrics**: Capital, P&L, returns, drawdown
- **Position Metrics**: Win rate, average win/loss, profit factor
- **Risk Metrics**: Sharpe ratio, maximum drawdown, VaR
- **Execution Metrics**: Signals processed, execution rate, latency

```rust
let stats = trader.get_statistics();
println!("Win Rate: {:.1}%", stats.position_stats.win_rate);
println!("Sharpe Ratio: {:.3}", stats.risk_metrics.sharpe_ratio);
println!("Max Drawdown: {:.2}%", stats.risk_metrics.max_drawdown * 100.0);
```

## Architecture

```
┌─────────────────────┐
│ Prediction Engine   │ ──► Trading Signals
└─────────────────────┘
            │
            ▼
┌─────────────────────┐
│ Paper Trading Engine│
├─────────────────────┤
│ • Signal Processing │
│ • Risk Management   │
│ • Order Simulation  │
│ • P&L Calculation   │
└─────────────────────┘
            │
            ▼
┌─────────────────────┐
│ Portfolio & Stats   │
└─────────────────────┘
```

## Future Integration

This project is designed to eventually integrate with the full ARES-51 neuromorphic engine:

```toml
# Future dependency
[dependencies]
neuromorphic-engine = { path = "../ARES-51/neuromorphic-engine" }
```

## License

MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## Related Projects

- [ARES-51](../ARES-51) - Full neuromorphic trading system
- Original paper trading implementation extracted from ARES-51 project