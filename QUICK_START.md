# 🚀 Quick Start - Autonomous Neuromorphic Trading System

## One-Command Launch

```bash
./start-trading-system.sh
```

This single command will:
- Build the autonomous trading system
- Start all monitoring infrastructure (Grafana + metrics)
- Launch the AI trading engine
- Provide real-time system monitoring

## System Access

- **🤖 Trading System**: Automatically running and monitoring the entire stock market
- **📈 Grafana Dashboard**: [http://localhost:3000](http://localhost:3000) (admin/admin)
- **📊 Metrics API**: [http://localhost:3002](http://localhost:3002)
- **🔍 Trading Metrics**: [http://localhost:3001](http://localhost:3001)

## Trading Configuration

The autonomous system runs with these settings:
- 💰 **Starting Capital**: $100,000
- 🎯 **Min Confidence**: 72% (only trades on high-confidence opportunities)
- ⚡ **Risk Per Trade**: 1.5% (conservative risk management)
- 📊 **Max Daily Trades**: 25 (prevents over-trading)
- 📈 **Max Positions**: 12 (portfolio diversification)
- 🔥 **Portfolio Heat**: 12% (total exposure limit)

## Available Commands

```bash
# Start the complete system
./start-trading-system.sh

# Stop the system
./stop-trading-system.sh

# View trading logs
docker compose logs -f neuromorphic-trader

# View all logs
docker compose logs -f

# Restart just the trading engine
docker compose restart neuromorphic-trader
```

## System Components

1. **Neuromorphic Trader** (Port 3001) - The AI trading engine with 6 strategies:
   - Neuromorphic momentum detection
   - Volume spike analysis
   - Breakout pattern recognition
   - Gap and go strategies
   - Relative strength analysis
   - Volatility breakout detection

2. **Metrics Server** (Port 3002) - Real-time performance metrics
3. **Grafana Dashboard** (Port 3000) - Visual monitoring and analytics

## Market Coverage

The system continuously monitors:
- **NYSE** - All listed stocks
- **NASDAQ** - All listed stocks
- **Real-time data** from Yahoo Finance
- **24/7 monitoring** (market hours + pre/after market)

## AI Trading Intelligence

The system uses advanced AI strategies to:
- Detect neuromorphic momentum patterns
- Analyze volume spikes and breakouts
- Identify gap trading opportunities
- Manage risk automatically
- Execute trades only on high-confidence signals (>72%)

## Stop the System

```bash
./stop-trading-system.sh
```

## Troubleshooting

If you encounter issues:

1. **Check logs**: `docker compose logs -f neuromorphic-trader`
2. **Restart trading engine**: `docker compose restart neuromorphic-trader`
3. **Full restart**: `./stop-trading-system.sh && ./start-trading-system.sh`
4. **Clean restart**: `docker compose down -v && ./start-trading-system.sh`

## Manual Trading

To run the trading system manually without Docker:

```bash
cd neuromorphic-core
cargo run --example autonomous_trader
```