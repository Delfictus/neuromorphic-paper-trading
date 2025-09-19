//! Neuromorphic Paper Trading Application
//! 
//! Hybrid application that combines neuromorphic trading signals with
//! the Barter-rs trading framework for production-grade execution.

use anyhow::Result;
use std::time::Duration;
use tokio::signal;
use tracing::{info, warn, error};

use neuromorphic_core::exchanges::{Symbol, Exchange, BinanceWebSocketManager, StreamManager, StreamSubscription};
use neuromorphic_core::paper_trading::{TradingSignal, SignalAction, SignalMetadata};
use neuromorphic_barter_bridge::NeuromorphicBarterBridge;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🚀 Starting Neuromorphic Paper Trading System (Hybrid with Barter-rs)");

    // Create the neuromorphic-barter bridge
    let mut bridge = NeuromorphicBarterBridge::new().await?;
    bridge.start().await?;
    info!("✅ Neuromorphic-Barter bridge started");

    // Create WebSocket manager for real-time data
    let mut ws_manager = BinanceWebSocketManager::new(true); // Using testnet
    ws_manager.start().await?;
    info!("✅ Binance WebSocket manager started");

    // Subscribe to market data
    let symbols = vec![
        Symbol::new("BTCUSDT"),
        Symbol::new("ETHUSDT"),
        Symbol::new("ADAUSDT"),
    ];

    for symbol in &symbols {
        let trade_subscription = StreamSubscription::trade(symbol.clone());
        ws_manager.subscribe(trade_subscription).await?;
        info!("📈 Subscribed to {} trades", symbol);
    }

    // Get market data receiver
    let mut market_data_receiver = ws_manager.get_receiver()
        .ok_or_else(|| anyhow::anyhow!("Failed to get market data receiver"))?;

    info!("🧠 Starting neuromorphic signal generation and market data processing...");

    // Spawn market data processing task
    let bridge_handle = bridge;
    let market_data_task = tokio::spawn(async move {
        let mut message_count = 0;
        
        while let Ok(market_data) = market_data_receiver.recv().await {
            message_count += 1;
            
            // Process market data through the bridge
            if let Err(e) = bridge_handle.process_market_data(market_data.clone()).await {
                error!("Failed to process market data: {}", e);
            }
            
            // Generate neuromorphic signals based on market data
            if message_count % 10 == 0 { // Generate signal every 10 market events
                let signal = generate_demo_signal(&symbols).await;
                
                if let Err(e) = bridge_handle.send_signal(signal).await {
                    error!("Failed to send neuromorphic signal: {}", e);
                }
            }
            
            // Limit processing for demo
            if message_count >= 100 {
                info!("📊 Processed {} market events, stopping demo", message_count);
                break;
            }
        }
    });

    // Spawn portfolio monitoring task
    let portfolio_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            // Note: This would need a reference to the bridge to get actual stats
            // For now, just log that we're monitoring
            info!("💰 Portfolio monitoring - checking stats...");
        }
    });

    info!("📊 System is running. Press Ctrl+C to stop.");

    // Run until interrupted
    let shutdown = signal::ctrl_c();
    tokio::select! {
        _ = shutdown => {
            info!("🛑 Shutdown signal received...");
        }
        _ = market_data_task => {
            info!("📈 Market data processing completed");
        }
        _ = portfolio_task => {
            info!("💰 Portfolio monitoring completed");
        }
        _ = tokio::time::sleep(Duration::from_secs(300)) => {
            info!("⏰ Demo timeout reached...");
        }
    }

    // Clean shutdown
    ws_manager.stop().await?;
    info!("✅ Neuromorphic paper trading system shutdown complete");

    Ok(())
}

/// Generate a demo neuromorphic trading signal
async fn generate_demo_signal(symbols: &[Symbol]) -> TradingSignal {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Simple demo signal generation (in production this would use ARES)
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    let symbol_idx = (timestamp % symbols.len() as u128) as usize;
    let symbol = symbols[symbol_idx].clone();
    
    // Generate pseudo-random signal parameters
    let action_type = timestamp % 4;
    let confidence = 0.6 + ((timestamp % 40) as f64 / 100.0);
    let urgency = (timestamp % 100) as f64 / 100.0;
    
    let action = match action_type {
        0 => SignalAction::Buy { size_hint: Some(1000.0) },
        1 => SignalAction::Sell { size_hint: Some(500.0) },
        2 => SignalAction::Close { position_id: None },
        _ => SignalAction::Hold,
    };
    
    TradingSignal {
        symbol,
        exchange: Exchange::Binance,
        action,
        confidence,
        urgency,
        metadata: SignalMetadata {
            spike_count: (timestamp % 1000) as u64,
            pattern_strength: confidence,
            market_regime: "demo_trending".to_string(),
            volatility: 0.02,
        },
    }
}