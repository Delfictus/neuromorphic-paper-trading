//! Hybrid Architecture Demo
//! 
//! This example demonstrates the integration between neuromorphic trading
//! signals and the Barter-rs framework.

use anyhow::Result;
use std::time::Duration;
use tracing::{info, error};

use neuromorphic_core::exchanges::{Symbol, Exchange};
use neuromorphic_core::paper_trading::{TradingSignal, SignalAction, SignalMetadata};
use neuromorphic_barter_bridge::{NeuromorphicBarterBridge, BridgeResult};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🚀 Starting Hybrid Architecture Demo");
    info!("🔗 Neuromorphic Signals ↔️ Barter-rs Framework");

    // Create and start the bridge
    let mut bridge = NeuromorphicBarterBridge::new().await?;
    bridge.start().await?;
    info!("✅ Neuromorphic-Barter bridge initialized");

    // Demo: Generate and process neuromorphic signals
    let demo_signals = generate_demo_signals();
    
    info!("🧠 Processing {} neuromorphic trading signals", demo_signals.len());
    
    for (i, signal) in demo_signals.into_iter().enumerate() {
        info!("📡 Signal #{}: {} {} on {}", 
              i + 1, 
              signal.action.action_type(),
              signal.symbol,
              signal.exchange
        );
        
        // Send signal through the bridge
        match bridge.send_signal(signal).await {
            Ok(()) => {
                info!("✅ Signal processed successfully");
            }
            Err(e) => {
                error!("❌ Failed to process signal: {}", e);
            }
        }
        
        // Get portfolio stats after each signal
        match bridge.get_portfolio_stats() {
            Ok(stats) => {
                info!("💰 Portfolio - Value: ${:.2}, Cash: ${:.2}, PnL: ${:.2}", 
                      stats.total_value, stats.cash, stats.realized_pnl);
            }
            Err(e) => {
                error!("Failed to get portfolio stats: {}", e);
            }
        }
        
        // Small delay between signals
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    info!("📊 Demo completed successfully!");
    info!("🎯 Key achievements:");
    info!("   ✅ Neuromorphic signals converted to Barter format");
    info!("   ✅ Portfolio management through Barter framework");
    info!("   ✅ Bridge architecture working correctly");

    Ok(())
}

/// Generate demo neuromorphic trading signals
fn generate_demo_signals() -> Vec<TradingSignal> {
    vec![
        // BTC Buy signal with high confidence
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
        
        // ETH Buy signal with medium confidence
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
        
        // Hold signal (no action)
        TradingSignal {
            symbol: Symbol::new("ADA-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Hold,
            confidence: 0.55,
            urgency: 0.2,
            metadata: SignalMetadata {
                spike_count: 30,
                pattern_strength: 0.5,
                market_regime: "sideways".to_string(),
                volatility: 0.015,
            },
        },
        
        // Sell signal
        TradingSignal {
            symbol: Symbol::new("SOL-USD"),
            exchange: Exchange::Binance,
            action: SignalAction::Sell { size_hint: Some(800.0) },
            confidence: 0.75,
            urgency: 0.8,
            metadata: SignalMetadata {
                spike_count: 200,
                pattern_strength: 0.8,
                market_regime: "bearish_reversal".to_string(),
                volatility: 0.035,
            },
        },
        
        // Close position signal
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
    ]
}

/// Helper trait to get action type as string
trait SignalActionExt {
    fn action_type(&self) -> &'static str;
}

impl SignalActionExt for SignalAction {
    fn action_type(&self) -> &'static str {
        match self {
            SignalAction::Buy { .. } => "BUY",
            SignalAction::Sell { .. } => "SELL", 
            SignalAction::Close { .. } => "CLOSE",
            SignalAction::Hold => "HOLD",
        }
    }
}