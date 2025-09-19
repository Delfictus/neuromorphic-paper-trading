# Neuromorphic Paper Trader (Hybrid Architecture)

A high-performance paper trading system that combines neuromorphic trading signals with the proven Barter-rs framework. This hybrid approach leverages the best of both worlds: cutting-edge neuromorphic intelligence and production-ready trading infrastructure.

## 🏗️ **Architecture Overview**

This project uses a **Cargo workspace** with three main components:

```
neuromorphic-paper-trader/
├── neuromorphic-core/              # Core neuromorphic components
│   ├── exchanges/                  # Exchange connectivity & WebSocket
│   ├── paper_trading/              # Paper trading engine
│   └── market_data/                # ARES spike encoding bridge
├── neuromorphic-barter-bridge/     # Integration layer
│   └── bridge.rs                   # Converts signals to Barter format
├── paper-trader-app/               # Main application
│   ├── main.rs                     # Hybrid trading application
│   └── examples/                   # Demo applications
└── Cargo.toml                      # Workspace configuration
```

## 🎯 **Key Features**

### **🧠 Neuromorphic Intelligence**
- ✅ **ARES Integration** - Spike encoding and pattern recognition
- ✅ **Real-time Processing** - Nanosecond precision signal generation
- ✅ **Adaptive Learning** - Dynamic sensitivity adjustment

### **⚡ Production-Grade Trading (via Barter-rs)**
- ✅ **Multi-Exchange Support** - Unified API across exchanges
- ✅ **Live & Paper Trading** - Identical code for both modes
- ✅ **Advanced Analytics** - Comprehensive performance metrics
- ✅ **Risk Management** - Position sizing and portfolio controls

### **🔌 Real-Time Data Streaming**
- ✅ **WebSocket Feeds** - Binance, Coinbase, Kraken support
- ✅ **Connection Management** - Auto-reconnection and health monitoring
- ✅ **Data Normalization** - Universal format across exchanges

## 🚀 **Quick Start**

### **Run the Hybrid Application**

```bash
# Clone the repository
git clone <repository>
cd neuromorphic-paper-trader

# Run the main hybrid application
cargo run --bin neuromorphic-trader

# Or run the demo
cargo run --example hybrid_demo
```

### **Run WebSocket Demo**

```bash
# Test real-time market data streaming
cargo run --example websocket_demo -p neuromorphic-core
```

## 🧪 **Development Workflow**

### **Working with the Workspace**

```bash
# Build all crates
cargo build

# Test all crates
cargo test

# Run specific crate
cargo run -p paper-trader-app

# Check specific crate
cargo check -p neuromorphic-core
```

### **Adding Dependencies**

Add to workspace `Cargo.toml`:

```toml
[workspace.dependencies]
new-dependency = "1.0"
```

Then use in individual crates:

```toml
[dependencies]
new-dependency = { workspace = true }
```

## 🔧 **Architecture Details**

### **1. Neuromorphic Core (`neuromorphic-core`)**

Contains our original neuromorphic trading components:

```rust
use neuromorphic_core::{
    exchanges::{BinanceWebSocketManager, StreamManager},
    paper_trading::{TradingSignal, SignalAction},
    market_data::{MarketDataSpikeBridge}
};
```

Key modules:
- `exchanges/` - Exchange APIs and WebSocket streaming
- `paper_trading/` - Core trading engine and risk management  
- `market_data/` - ARES spike encoding integration

### **2. Barter Bridge (`neuromorphic-barter-bridge`)**

Converts between our neuromorphic signals and Barter-rs format:

```rust
use neuromorphic_barter_bridge::NeuromorphicBarterBridge;

// Create bridge
let mut bridge = NeuromorphicBarterBridge::new().await?;

// Send neuromorphic signal
let signal = TradingSignal { /* ... */ };
bridge.send_signal(signal).await?;

// Get portfolio stats from Barter
let stats = bridge.get_portfolio_stats()?;
```

### **3. Main Application (`paper-trader-app`)**

Orchestrates the complete system:

```rust
// Real-time market data
let mut ws_manager = BinanceWebSocketManager::new(true);
let mut receiver = ws_manager.get_receiver().unwrap();

// Neuromorphic-Barter bridge
let mut bridge = NeuromorphicBarterBridge::new().await?;

// Process market data and generate signals
while let Ok(market_data) = receiver.recv().await {
    // Send to Barter for processing
    bridge.process_market_data(market_data).await?;
    
    // Generate neuromorphic signal
    let signal = generate_neuromorphic_signal(&market_data).await;
    bridge.send_signal(signal).await?;
}
```

## 📊 **Signal Flow**

```
Market Data → WebSocket → Neuromorphic Core → ARES Processing
     ↓                                             ↓
Portfolio Stats ← Barter Engine ← Bridge ← Trading Signals
```

1. **Real-time market data** streams via WebSocket
2. **ARES spike encoding** processes market patterns
3. **Neuromorphic signals** generated from spike patterns
4. **Bridge converts** signals to Barter format
5. **Barter engine** executes trades and manages portfolio
6. **Portfolio statistics** tracked and reported

## 🎛️ **Configuration**

### **Neuromorphic Configuration**

```rust
use neuromorphic_core::market_data::SpikeBridgeConfig;

let spike_config = SpikeBridgeConfig {
    neuron_count: 10000,
    spike_buffer_size: 100000,
    batch_size: 100,
    encoding_timeout: Duration::from_millis(10),
    enable_adaptive_encoding: true,
};
```

### **WebSocket Configuration**

```rust
use neuromorphic_core::exchanges::WebSocketConfig;

let ws_config = WebSocketConfig {
    base_url: "wss://stream.binance.com:9443/ws".to_string(),
    ping_interval: Duration::from_secs(30),
    reconnect_interval: Duration::from_secs(5),
    max_reconnect_attempts: 10,
    message_timeout: Duration::from_secs(30),
    buffer_size: 1000,
};
```

### **Trading Configuration**

```rust
// Portfolio setup via Barter
let portfolio = PortfolioBuilder::new()
    .initial_cash(100_000.0)
    .build()?;
```

## 📈 **Performance Metrics**

The hybrid system provides comprehensive analytics:

### **Neuromorphic Metrics**
- Spike generation rate
- Pattern recognition accuracy
- Signal latency and throughput
- Adaptive encoding performance

### **Trading Metrics (via Barter)**
- Portfolio value and P&L
- Win rate and profit factor
- Sharpe ratio and drawdown
- Execution statistics

```rust
// Get combined metrics
let bridge_stats = bridge.get_portfolio_stats()?;
let spike_stats = spike_bridge.get_statistics()?;
let ws_metrics = ws_manager.get_metrics().await;

println!("Portfolio Value: ${:.2}", bridge_stats.total_value);
println!("Spikes Generated: {}", spike_stats.spikes_generated);
println!("WebSocket Latency: {:.2}ms", ws_metrics.average_latency_ms);
```

## 🔬 **Testing**

```bash
# Run all tests
cargo test

# Test specific functionality
cargo test -p neuromorphic-core exchange
cargo test -p neuromorphic-barter-bridge bridge
cargo test -p paper-trader-app integration

# Run with logging
RUST_LOG=debug cargo test -- --nocapture
```

## 🎯 **Examples**

### **Basic Signal Processing**

```rust
use neuromorphic_core::paper_trading::{TradingSignal, SignalAction};
use neuromorphic_barter_bridge::NeuromorphicBarterBridge;

let signal = TradingSignal {
    symbol: Symbol::new("BTC-USD"),
    exchange: Exchange::Binance,
    action: SignalAction::Buy { size_hint: Some(1000.0) },
    confidence: 0.85,
    urgency: 0.7,
    metadata: SignalMetadata { /* ... */ },
};

let mut bridge = NeuromorphicBarterBridge::new().await?;
bridge.send_signal(signal).await?;
```

### **Real-Time Market Data**

```rust
use neuromorphic_core::exchanges::{BinanceWebSocketManager, StreamSubscription};

let mut manager = BinanceWebSocketManager::new(false);
manager.start().await?;

// Subscribe to BTC trades
manager.subscribe(StreamSubscription::trade(Symbol::new("BTCUSDT"))).await?;

// Process incoming data
let mut receiver = manager.get_receiver().unwrap();
while let Ok(market_data) = receiver.recv().await {
    println!("Received: {:?}", market_data);
}
```

## 🤝 **Contributing**

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/amazing-feature`
3. **Make** your changes in the appropriate workspace crate
4. **Add tests** for new functionality
5. **Ensure** all tests pass: `cargo test`
6. **Submit** a pull request

## 📚 **Documentation**

- [Barter-rs Documentation](https://docs.rs/barter)
- [ARES Neuromorphic Engine](../ARES-51)
- [API Documentation](docs/api.md)

## ⚡ **Performance Benefits**

### **Hybrid Approach Advantages**

| Component | Traditional | Our Hybrid | Benefit |
|-----------|------------|------------|---------|
| **Trading Engine** | Custom Implementation | Barter-rs Framework | ✅ Production-ready, Battle-tested |
| **Signal Generation** | Rule-based | Neuromorphic | ✅ Adaptive, Pattern Recognition |
| **Exchange Connectivity** | Basic REST | WebSocket + REST | ✅ Real-time + Reliable |
| **Risk Management** | Basic | Barter Portfolio System | ✅ Advanced Position Management |
| **Backtesting** | None | Barter Backtesting | ✅ Historical Strategy Validation |

## 🔮 **Future Roadmap**

- [ ] **Additional Exchanges** - Coinbase Pro, Kraken, OKX
- [ ] **Live Trading Mode** - Production deployment
- [ ] **Advanced Strategies** - Multi-timeframe analysis
- [ ] **ML Integration** - TensorFlow/PyTorch models
- [ ] **Web Dashboard** - Real-time monitoring UI
- [ ] **Cloud Deployment** - Kubernetes orchestration

## 📄 **License**

MIT License - see [LICENSE](LICENSE) file for details.

## 🔗 **Related Projects**

- [Barter-rs](https://github.com/barter-rs/barter-rs) - Rust trading framework
- [ARES-51](../ARES-51) - Neuromorphic trading system
- [NautilusTrader](https://github.com/nautechsystems/nautilus_trader) - Alternative trading platform