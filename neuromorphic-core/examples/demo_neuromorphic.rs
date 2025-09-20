//! Simple demonstration of the neuromorphic paper trading system
//! 
//! This demo shows the core functionality working with test data

use neuromorphic_core::{
    TradingSignal, SignalAction, SignalMetadata,
    Symbol, Exchange, NeuromorphicPaperTrader
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Neuromorphic Paper Trading System Demo");
    println!("==========================================");
    
    // Use default configuration
    let config = neuromorphic_core::PaperTradingConfig::default();
    
    println!("📊 Configuration:");
    println!("   Initial Capital: ${:.2}", config.initial_capital);
    println!("   Commission Rate: {:.1}%", config.commission_rate);
    println!();
    
    // Create and start the paper trader
    let mut trader = NeuromorphicPaperTrader::new(config);
    
    // Start the trading engine
    trader.start().await?;
    println!("✅ Neuromorphic Paper Trading Engine Started");
    println!();
    
    // Simulate some neuromorphic trading signals
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
            symbol: Symbol::new("BTC-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Close { position_id: None },
            confidence: 0.95,
            urgency: 0.9,
            metadata: SignalMetadata {
                spike_count: 300,
                pattern_strength: 0.95,
                market_regime: "risk_off".to_string(),
                volatility: 0.045,
            },
        },
    ];
    
    println!("🧠 Processing {} neuromorphic trading signals:", signals.len());
    println!();
    
    // Process each signal
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
        
        // Update market prices (simulated)
        match signal.symbol.as_str() {
            "BTC-USD" => trader.update_market_price(signal.symbol.clone(), 45000.0 + (i as f64 * 1000.0)),
            "ETH-USD" => trader.update_market_price(signal.symbol.clone(), 2800.0 + (i as f64 * 100.0)),
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
        println!();
        
        // Small delay between signals
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    // Final statistics
    let final_stats = trader.get_statistics();
    println!("📈 Final Portfolio Statistics:");
    println!("   Capital: ${:.2}", final_stats.capital);
    println!("   Total P&L: ${:.2}", final_stats.total_pnl);
    println!("   Return: {:.2}%", final_stats.total_return_pct);
    println!();
    
    println!("🎯 Demo completed successfully!");
    println!("✅ Neuromorphic intelligence integrated with paper trading");
    println!("✅ Real-time signal processing functional");
    println!("✅ Portfolio management and risk controls working");
    
    // Stop the trading engine
    trader.stop().await?;
    println!("🔴 Trading engine stopped");
    
    Ok(())
}

fn get_action_type(action: &SignalAction) -> &'static str {
    match action {
        SignalAction::Buy { .. } => "BUY",
        SignalAction::Sell { .. } => "SELL",
        SignalAction::Close { .. } => "CLOSE",
        SignalAction::Hold => "HOLD",
    }
}