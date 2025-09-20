//! Neuromorphic Paper Trading Application
//! 
//! Standalone paper trading system that can integrate with neuromorphic
//! prediction engines for signal generation.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{info, warn};

mod paper_trading;
mod exchanges;

use paper_trading::{PaperTradingEngine, PaperTradingConfig, TradingSignal, SignalAction, SignalMetadata};
use exchanges::{Symbol, Exchange, Side};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🚀 Starting Neuromorphic Paper Trading System");

    // Configure paper trading
    let config = PaperTradingConfig {
        initial_capital: 100_000.0,
        commission_rate: 0.1, // 0.1%
        enable_stop_loss: true,
        enable_take_profit: true,
        update_interval: Duration::from_millis(100),
        ..Default::default()
    };

    // Create paper trading engine
    let mut engine = PaperTradingEngine::new(config);
    
    // Start the engine
    engine.start().await?;
    info!("✅ Paper trading engine started");

    // Wrap engine in Arc to share between tasks
    let engine = Arc::new(engine);

    // Simulate some market data and trading signals
    let engine_clone = engine.clone();
    tokio::spawn(async move {
        simulate_trading_session(&*engine_clone).await;
    });

    // Run until interrupted
    info!("📈 Paper trading system is running. Press Ctrl+C to stop.");
    
    let shutdown = signal::ctrl_c();
    tokio::select! {
        _ = shutdown => {
            info!("🛑 Shutdown signal received...");
        }
        _ = tokio::time::sleep(Duration::from_secs(3600)) => {
            info!("⏰ Max runtime reached...");
        }
    }

    // Stop the engine
    engine.stop().await?;

    // Print final statistics
    let stats = engine.get_statistics();
    info!("📊 Final Statistics:");
    info!("  Final Capital: ${:.2}", stats.capital);
    info!("  Total P&L: ${:.2} ({:.2}%)", stats.total_pnl, stats.total_return_pct);
    info!("  Signals Processed: {}", stats.signals_processed);
    info!("  Signals Executed: {}", stats.signals_executed);
    info!("  Win Rate: {:.1}%", stats.position_stats.win_rate);

    info!("✅ Paper trading system shutdown complete");
    Ok(())
}

/// Simulate a trading session with mock signals
async fn simulate_trading_session(engine: &PaperTradingEngine) {
    let symbols = vec![
        Symbol::new("BTC-USD"),
        Symbol::new("ETH-USD"),
        Symbol::new("SOL-USD"),
    ];

    let mut price_btc = 50000.0;
    let mut price_eth = 3000.0;
    let mut price_sol = 100.0;

    for i in 0..1000 {
        // Simulate price movements
        price_btc += (rand::random_f64() - 0.5) * 100.0;
        price_eth += (rand::random_f64() - 0.5) * 10.0;
        price_sol += (rand::random_f64() - 0.5) * 2.0;

        // Update prices
        engine.update_price(symbols[0].clone(), price_btc);
        engine.update_price(symbols[1].clone(), price_eth);
        engine.update_price(symbols[2].clone(), price_sol);

        // Generate trading signals every 10 iterations
        if i % 10 == 0 {
            let symbol = &symbols[i % symbols.len()];
            let confidence = 0.6 + (rand::random_f64() * 0.3);
            let urgency = rand::random_f64();

            let action = match rand::random_u32() % 4 {
                0 => SignalAction::Buy { size_hint: Some(1000.0) },
                1 => SignalAction::Sell { size_hint: Some(500.0) },
                2 => SignalAction::Close { position_id: None },
                _ => SignalAction::Hold,
            };

            let signal = TradingSignal {
                symbol: symbol.clone(),
                exchange: Exchange::Binance,
                action,
                confidence,
                urgency,
                metadata: SignalMetadata {
                    spike_count: (rand::random_u32() as u64) % 1000,
                    pattern_strength: confidence,
                    market_regime: "trending".to_string(),
                    volatility: 0.02,
                },
            };

            if let Err(e) = engine.process_signal(signal).await {
                warn!("Error processing signal: {}", e);
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// Mock random function for demo purposes
mod rand {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn random_f64() -> f64 {
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        (hasher.finish() as f64) / (u64::MAX as f64)
    }

    pub fn random_u32() -> u32 {
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        hasher.finish() as u32
    }
}