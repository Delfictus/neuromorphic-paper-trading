//! WebSocket streaming demo
//! 
//! This example demonstrates how to use the WebSocket streaming interface
//! to subscribe to real-time market data from Binance.

use neuromorphic_paper_trader::exchanges::{
    BinanceWebSocketManager, StreamManager, StreamSubscription, Symbol, StreamType
};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🚀 Starting WebSocket streaming demo");

    // Create Binance WebSocket manager (using testnet)
    let mut manager = BinanceWebSocketManager::new(true);

    // Start the stream manager
    manager.start().await?;
    info!("✅ WebSocket manager started");

    // Get data receiver
    let mut receiver = manager.get_receiver()
        .ok_or("Failed to get data receiver")?;

    // Subscribe to BTC-USDT trade stream
    let btc_trade_subscription = StreamSubscription::trade(Symbol::new("BTCUSDT"));
    manager.subscribe(btc_trade_subscription).await?;
    info!("📈 Subscribed to BTC-USDT trades");

    // Subscribe to ETH-USDT quote stream
    let eth_quote_subscription = StreamSubscription::quote(Symbol::new("ETHUSDT"));
    manager.subscribe(eth_quote_subscription).await?;
    info!("💰 Subscribed to ETH-USDT quotes");

    // Subscribe to multiple symbols at once
    let symbols = vec![
        Symbol::new("ADAUSDT"),
        Symbol::new("SOLUSDT"),
    ];
    manager.subscribe_symbols(symbols, StreamType::Trade).await?;
    info!("🔄 Subscribed to multiple symbol trades");

    info!("📊 Listening for market data (press Ctrl+C to stop)...");

    // Listen for data with timeout
    let mut message_count = 0;
    let max_messages = 50; // Limit for demo purposes

    while message_count < max_messages {
        match timeout(Duration::from_secs(30), receiver.recv()).await {
            Ok(Ok(market_data)) => {
                message_count += 1;
                
                match market_data {
                    neuromorphic_paper_trader::exchanges::UniversalMarketData::Trade(trade) => {
                        info!(
                            "🔹 TRADE: {} {} {} @ {} (ID: {})",
                            trade.symbol,
                            match trade.side {
                                neuromorphic_paper_trader::exchanges::Side::Buy => "BUY",
                                neuromorphic_paper_trader::exchanges::Side::Sell => "SELL",
                            },
                            trade.quantity,
                            trade.price,
                            trade.trade_id
                        );
                    }
                    neuromorphic_paper_trader::exchanges::UniversalMarketData::Quote(quote) => {
                        info!(
                            "🔸 QUOTE: {} BID: {}@{} ASK: {}@{}",
                            quote.symbol,
                            quote.bid_price,
                            quote.bid_size,
                            quote.ask_price,
                            quote.ask_size
                        );
                    }
                    neuromorphic_paper_trader::exchanges::UniversalMarketData::OrderBook(book) => {
                        info!(
                            "📚 ORDERBOOK: {} {} bids, {} asks",
                            book.symbol,
                            book.bids.len(),
                            book.asks.len()
                        );
                    }
                }
            }
            Ok(Err(e)) => {
                error!("Error receiving data: {}", e);
                break;
            }
            Err(_) => {
                error!("Timeout waiting for data");
                break;
            }
        }
    }

    // Show stream metrics
    let metrics = manager.get_metrics().await;
    info!("📊 Stream Metrics:");
    info!("  Messages received: {}", metrics.messages_received);
    info!("  Messages parsed: {}", metrics.messages_parsed);
    info!("  Parse errors: {}", metrics.parse_errors);
    info!("  Connection errors: {}", metrics.connection_errors);
    info!("  Reconnections: {}", metrics.reconnection_count);
    
    if let Some(last_msg_time) = metrics.last_message_time {
        info!("  Last message: {:.2}s ago", last_msg_time.elapsed().as_secs_f64());
    }

    // Clean shutdown
    manager.stop().await?;
    info!("✅ WebSocket manager stopped");

    info!("🎯 Demo completed successfully!");
    Ok(())
}