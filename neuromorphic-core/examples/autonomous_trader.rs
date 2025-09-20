use neuromorphic_core::{
    AutonomousTradingSystem, AutonomousConfig, ScannerConfig, PaperTradingConfig,
    Exchange
};
use anyhow::Result;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("🚀 Launching Autonomous Neuromorphic Trading System");
    println!("📊 This system will continuously monitor the entire stock market");
    println!("🎯 and execute trades on high-confidence opportunities");
    println!();

    let autonomous_config = AutonomousConfig {
        scanner_config: ScannerConfig {
            max_symbols: 1000,
            scan_interval_ms: 5000,
            min_volume_threshold: 500000.0,
            min_price_threshold: 10.0,
            max_price_threshold: 1000.0,
            excluded_sectors: vec!["Penny Stocks".to_string(), "OTC".to_string()],
            included_exchanges: vec![Exchange::NYSE, Exchange::NASDAQ],
            enable_premarket: true,
            enable_afterhours: true,
            momentum_lookback_periods: vec![5, 15, 30, 60],
            volatility_threshold: 2.0,
            volume_spike_threshold: 3.0,
        },
        trading_config: PaperTradingConfig {
            initial_capital: 100000.0,
            commission_rate: 0.001,
            max_position_size: 0.05,
            stop_loss_pct: 0.03,
            take_profit_pct: 0.08,
            risk_free_rate: 0.02,
            max_positions: 8,
            position_sizing_method: "fixed_fractional".to_string(),
            enable_stop_loss: true,
            enable_take_profit: true,
            enable_trailing_stop: true,
            update_interval_ms: 1000,
        },
        max_positions: 12,
        max_daily_trades: 25,
        risk_per_trade: 0.015,
        enable_auto_trading: true,
        min_opportunity_confidence: 0.72,
        portfolio_heat: 0.12,
    };

    let mut trading_system = AutonomousTradingSystem::new(autonomous_config);

    println!("⚙️  Configuration:");
    println!("   💰 Starting capital: $100,000");
    println!("   📈 Max positions: 12");
    println!("   🎯 Min confidence: 72%");
    println!("   ⚡ Risk per trade: 1.5%");
    println!("   📊 Max daily trades: 25");
    println!("   🔥 Portfolio heat limit: 12%");
    println!();

    let system_handle = tokio::spawn(async move {
        if let Err(e) = trading_system.start().await {
            eprintln!("❌ Trading system error: {}", e);
        }
    });

    let ctrl_c = signal::ctrl_c();
    println!("🎮 System running! Press Ctrl+C to stop gracefully...");
    println!("📈 Grafana dashboard available at: http://localhost:3000");
    println!("🔗 Metrics API available at: http://localhost:3002");
    println!();

    tokio::select! {
        _ = ctrl_c => {
            println!("\n🛑 Shutdown signal received");
        }
        _ = system_handle => {
            println!("💀 Trading system terminated");
        }
    }

    println!("✅ Autonomous trading system stopped");
    Ok(())
}