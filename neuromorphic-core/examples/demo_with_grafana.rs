//! Neuromorphic trading demo with Grafana metrics integration
//! 
//! This demo shows the neuromorphic paper trading system with real-time
//! metrics collection and Grafana API endpoints.

use neuromorphic_core::{
    TradingSignal, SignalAction, SignalMetadata,
    Symbol, Exchange, NeuromorphicPaperTrader, PaperTradingConfig
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("🚀 Neuromorphic Paper Trading System Demo with Grafana Integration");
    println!("================================================================");
    
    // Use default configuration
    let config = PaperTradingConfig::default();
    
    println!("📊 Configuration:");
    println!("   Initial Capital: ${:.2}", config.initial_capital);
    println!("   Commission Rate: {:.1}%", config.commission_rate);
    println!();
    
    // Create and start the paper trader
    let mut trader = NeuromorphicPaperTrader::new(config);
    
    // Start the trading engine
    trader.start().await?;
    println!("✅ Neuromorphic Paper Trading Engine Started");
    
    // Start Grafana metrics API server
    trader.start_metrics_api(3001).await;
    println!("📈 Grafana Metrics API started on http://localhost:3001");
    println!("   Available endpoints:");
    println!("   - http://localhost:3001/health");
    println!("   - http://localhost:3001/api/v1/metrics/portfolio");
    println!("   - http://localhost:3001/api/v1/metrics/signals");
    println!("   - http://localhost:3001/api/v1/metrics/all");
    println!("   - http://localhost:3001/api/v1/timeseries/portfolio_pnl");
    println!();
    
    // Give the API server time to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Simulate extended neuromorphic trading signals for better metrics
    let signals = vec![
        TradingSignal {
            symbol: Symbol::new("BTC-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Buy { size_hint: Some(1000.0) },
            confidence: 0.85,
            urgency: 0.7,
            metadata: SignalMetadata {
                spike_count: 150,
                pattern_strength: 0.9,
                market_regime: "strong_uptrend".to_string(),
                volatility: 0.025,
            },
        },
        TradingSignal {
            symbol: Symbol::new("ETH-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Buy { size_hint: Some(2000.0) },
            confidence: 0.65,
            urgency: 0.4,
            metadata: SignalMetadata {
                spike_count: 80,
                pattern_strength: 0.7,
                market_regime: "consolidation".to_string(),
                volatility: 0.018,
            },
        },
        TradingSignal {
            symbol: Symbol::new("ADA-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Buy { size_hint: Some(5000.0) },
            confidence: 0.75,
            urgency: 0.6,
            metadata: SignalMetadata {
                spike_count: 120,
                pattern_strength: 0.8,
                market_regime: "mild_uptrend".to_string(),
                volatility: 0.035,
            },
        },
        TradingSignal {
            symbol: Symbol::new("SOL-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Sell { size_hint: Some(1500.0) },
            confidence: 0.55,
            urgency: 0.3,
            metadata: SignalMetadata {
                spike_count: 60,
                pattern_strength: 0.6,
                market_regime: "weak_downtrend".to_string(),
                volatility: 0.045,
            },
        },
        TradingSignal {
            symbol: Symbol::new("BTC-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Close { position_id: None },
            confidence: 0.95,
            urgency: 0.9,
            metadata: SignalMetadata {
                spike_count: 300,
                pattern_strength: 0.95,
                market_regime: "risk_off".to_string(),
                volatility: 0.055,
            },
        },
        // Additional signals for richer metrics
        TradingSignal {
            symbol: Symbol::new("LINK-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Buy { size_hint: Some(3000.0) },
            confidence: 0.70,
            urgency: 0.5,
            metadata: SignalMetadata {
                spike_count: 100,
                pattern_strength: 0.75,
                market_regime: "recovery".to_string(),
                volatility: 0.030,
            },
        },
        TradingSignal {
            symbol: Symbol::new("DOT-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Hold,
            confidence: 0.45,
            urgency: 0.2,
            metadata: SignalMetadata {
                spike_count: 40,
                pattern_strength: 0.5,
                market_regime: "sideways".to_string(),
                volatility: 0.022,
            },
        },
    ];
    
    println!("🧠 Processing {} neuromorphic trading signals:", signals.len());
    println!();
    
    // Process each signal with realistic price updates
    for (i, signal) in signals.iter().enumerate() {
        println!("📡 Signal #{}: {} {} on {}", 
                 i + 1, 
                 get_action_type(&signal.action),
                 signal.symbol,
                 signal.exchange
        );
        println!("   Confidence: {:.1}%, Urgency: {:.1}%",
                 signal.confidence * 100.0,
                 signal.urgency * 100.0
        );
        println!("   Pattern: {} (strength: {:.1}%)",
                 signal.metadata.market_regime,
                 signal.metadata.pattern_strength * 100.0
        );
        println!("   Spikes: {}, Volatility: {:.1}%",
                 signal.metadata.spike_count,
                 signal.metadata.volatility * 100.0
        );
        
        // Update market prices with some variation
        let price_factor = 1.0 + (i as f64 * 0.02) - 0.05; // ±5% variation
        match signal.symbol.as_str() {
            "BTC-USD" => trader.update_market_price(signal.symbol.clone(), 45000.0 * price_factor),
            "ETH-USD" => trader.update_market_price(signal.symbol.clone(), 2800.0 * price_factor),
            "ADA-USD" => trader.update_market_price(signal.symbol.clone(), 0.45 * price_factor),
            "SOL-USD" => trader.update_market_price(signal.symbol.clone(), 95.0 * price_factor),
            "LINK-USD" => trader.update_market_price(signal.symbol.clone(), 14.50 * price_factor),
            "DOT-USD" => trader.update_market_price(signal.symbol.clone(), 6.80 * price_factor),
            _ => {}
        }
        
        // Process the signal
        match trader.process_prediction_signal(signal.clone()).await {
            Ok(()) => println!("   ✅ Signal processed successfully"),
            Err(e) => println!("   ❌ Signal processing failed: {}", e),
        }
        
        // Get current statistics
        let stats = trader.get_statistics();
        println!("   💰 Portfolio - Capital: ${:.2}, P&L: ${:.2}",
                 stats.capital, stats.total_pnl);
        
        // Show sample metrics
        let metrics = trader.metrics_collector().get_signal_metrics();
        println!("   📊 Signals processed: {}, Avg confidence: {:.1}%",
                 metrics.signals_processed, metrics.avg_confidence * 100.0);
        println!();
        
        // Delay between signals for realistic timing
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    
    // Final statistics
    let final_stats = trader.get_statistics();
    println!("📈 Final Portfolio Statistics:");
    println!("   Capital: ${:.2}", final_stats.capital);
    println!("   Total P&L: ${:.2}", final_stats.total_pnl);
    println!("   Return: {:.2}%", final_stats.total_return_pct);
    println!();
    
    // Show comprehensive metrics
    let all_metrics = trader.metrics_collector().get_all_metrics();
    println!("📊 Comprehensive Metrics Summary:");
    println!("   Portfolio Metrics:");
    println!("     - Total Positions: {}", all_metrics.portfolio.positions_count);
    println!("     - Win Rate: {:.1}%", all_metrics.portfolio.win_rate * 100.0);
    println!("     - Active Positions: {}", all_metrics.portfolio.active_positions_count);
    println!();
    println!("   Signal Metrics:");
    println!("     - Total Signals: {}", all_metrics.signals.signals_processed);
    println!("     - Avg Pattern Strength: {:.1}%", all_metrics.signals.pattern_strength_avg * 100.0);
    println!("     - Avg Spike Count: {:.0}", all_metrics.signals.spike_count_avg);
    println!("     - Signal Distribution:");
    for (action, count) in &all_metrics.signals.signal_distribution {
        println!("       - {}: {}", action, count);
    }
    println!("     - Market Regimes:");
    for (regime, count) in &all_metrics.signals.market_regimes {
        println!("       - {}: {}", regime, count);
    }
    println!();
    
    println!("🎯 Demo running with live metrics!");
    println!("📈 Visit Grafana and add data source: http://localhost:3001");
    println!("🔗 Test endpoints:");
    println!("   curl http://localhost:3001/health");
    println!("   curl http://localhost:3001/api/v1/metrics/portfolio");
    println!("   curl http://localhost:3001/api/v1/metrics/signals");
    println!();
    println!("💡 Press Ctrl+C to stop the demo");
    
    // Keep the demo running to allow Grafana testing
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        
        // Optionally process more signals periodically
        let random_signal = TradingSignal {
            symbol: Symbol::new("BTC-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Hold,
            confidence: 0.5 + (rand::random::<f64>() * 0.3),
            urgency: 0.3 + (rand::random::<f64>() * 0.4),
            metadata: SignalMetadata {
                spike_count: 50 + (rand::random::<u32>() % 100),
                pattern_strength: 0.5 + (rand::random::<f64>() * 0.3),
                market_regime: "live_monitoring".to_string(),
                volatility: 0.02 + (rand::random::<f64>() * 0.03),
            },
        };
        
        // Update price with small random movement
        let btc_price = 45000.0 + (rand::random::<f64>() - 0.5) * 2000.0;
        trader.update_market_price(Symbol::new("BTC-USD"), btc_price);
        
        if let Err(e) = trader.process_prediction_signal(random_signal).await {
            println!("Error processing live signal: {}", e);
        }
        
        let current_stats = trader.get_statistics();
        println!("📊 Live Update - P&L: ${:.2}, Signals: {}", 
                 current_stats.total_pnl,
                 trader.metrics_collector().get_signal_metrics().signals_processed);
    }
}

fn get_action_type(action: &SignalAction) -> &'static str {
    match action {
        SignalAction::Buy { .. } => "BUY",
        SignalAction::Sell { .. } => "SELL",
        SignalAction::Close { .. } => "CLOSE",
        SignalAction::Hold => "HOLD",
    }
}

// Simple random number generation (replace with proper crate if needed)
mod rand {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static SEED: AtomicU64 = AtomicU64::new(12345);
    
    pub fn random<T>() -> T 
    where 
        T: From<u64>
    {
        let current = SEED.load(Ordering::Relaxed);
        let next = current.wrapping_mul(1103515245).wrapping_add(12345);
        SEED.store(next, Ordering::Relaxed);
        T::from(next)
    }
}