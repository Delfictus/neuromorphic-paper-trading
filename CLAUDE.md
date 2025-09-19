# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A standalone paper trading system extracted from the ARES-51 neuromorphic trading project. This system provides realistic trading simulation with support for external prediction engines.

## Development Commands

### Building and Testing
```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test <test_name>

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy
```

### Running the Application
```bash
# Run as standalone application
cargo run

# Run with specific log level
RUST_LOG=debug cargo run
```

## Architecture

### Core Modules

- **`paper_trading/`**: Core paper trading simulation engine
  - `engine.rs`: Main trading engine with signal processing
  - `position_manager.rs`: Portfolio and position management
  - `order_manager.rs`: Order execution and lifecycle management
  - `risk_manager.rs`: Risk controls and portfolio safety

- **`exchanges/`**: Exchange connectivity and data handling
  - `binance.rs`: Binance WebSocket connector
  - `types.rs`: Universal data types for market data
  - `orderbook.rs`: Order book management
  - `book_manager.rs`: Multi-exchange order book coordination

- **`market_data/`**: Market data normalization and processing
  - `universal.rs`: Universal market data structures
  - `normalizers.rs`: Exchange-specific data normalization
  - `unified_feed.rs`: Aggregated market data feed
  - `spike_bridge.rs`: Integration with neuromorphic systems

### Key Design Patterns

- **Plugin Architecture**: Designed to integrate with external prediction engines
- **Async-First**: Built on Tokio for high-performance async processing  
- **Universal Types**: Normalized data structures work across exchanges
- **Modular Risk Management**: Configurable risk controls and position sizing

### Entry Points

- `main.rs`: Standalone application with demo trading simulation
- `lib.rs`: Library interface for integration with external systems

## Configuration

The system uses `PaperTradingConfig` for configuration:
- Initial capital and commission rates
- Risk limits and position sizing
- Stop loss and take profit settings
- Update intervals and performance tuning

## Integration Points

### External Prediction Engines
The system processes `TradingSignal` structs with:
- Symbol and exchange information
- Signal action (Buy/Sell/Hold/Close)
- Confidence and urgency metrics
- Metadata for pattern analysis

### Future Integration
Designed to integrate with the full ARES-51 neuromorphic engine once available.

## Performance Considerations

- Real-time market data processing with minimal latency
- Efficient order book management for multiple exchanges
- Lock-free data structures where possible using DashMap and parking_lot
- Configurable update intervals to balance performance vs accuracy

## Git Repository Management

### Persistent Permissions
Claude has persistent permissions for the following git commands:
- `git status` - Check repository status
- `git add .` - Stage all changes  
- `git commit -m "message"` - Commit changes with message
- `git push` - Push changes to remote repository
- `git pull` - Pull changes from remote repository
- `git log` - View commit history
- `git diff` - View changes between commits
- `git branch` - Manage branches

### Repository Information
- **Remote**: https://github.com/Delfictus/neuromorphic-paper-trading.git
- **Main Branch**: main
- **Current Status**: Fully functional neuromorphic paper trading system
- **Build Status**: ✅ 0 compilation errors, production ready