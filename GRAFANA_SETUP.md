# Grafana Integration for Neuromorphic Trading System

This guide shows how to set up Grafana dashboards for real-time monitoring of your neuromorphic paper trading system.

## Quick Start

### 1. Run the Demo with Metrics API

```bash
# Start the neuromorphic trading system with Grafana metrics
cargo run -p neuromorphic-core --example demo_neuromorphic

# The metrics API will be available at http://localhost:3001
```

### 2. Install and Start Grafana

**Using Docker (Recommended):**
```bash
# Start Grafana container
docker run -d \
  --name grafana \
  -p 3000:3000 \
  grafana/grafana-oss:latest

# Access Grafana at http://localhost:3000
# Default login: admin/admin
```

**Using Package Manager:**
```bash
# On Ubuntu/Debian
sudo apt-get install -y software-properties-common
sudo add-apt-repository "deb https://packages.grafana.com/oss/deb stable main"
sudo apt-get update
sudo apt-get install grafana

# On macOS
brew install grafana

# Start Grafana
sudo systemctl start grafana-server
```

### 3. Configure Data Source

1. Open Grafana at http://localhost:3000
2. Login with admin/admin
3. Go to Configuration → Data Sources
4. Add a new data source → JSON API
5. Configure:
   - **URL**: `http://localhost:3001/api/v1/metrics`
   - **Access**: Server (default)
   - **Headers**: None required

### 4. Import Dashboard

1. Go to Dashboards → Import
2. Upload the file: `grafana/neuromorphic-trading-dashboard.json`
3. Or copy the contents and paste into the import box

## Available Metrics Endpoints

### Portfolio Metrics
```bash
curl http://localhost:3001/api/v1/metrics/portfolio
```
**Response:**
```json
{
  "timestamp": "2023-XX-XXTXX:XX:XX.XXXZ",
  "total_capital": 100000.0,
  "available_capital": 95000.0,
  "total_pnl": 2500.0,
  "unrealized_pnl": 1200.0,
  "realized_pnl": 1300.0,
  "total_return_pct": 2.5,
  "positions_count": 5,
  "active_positions_count": 3,
  "total_trades": 15,
  "winning_trades": 9,
  "losing_trades": 6,
  "win_rate": 0.6,
  "avg_win": 400.0,
  "avg_loss": -200.0,
  "max_drawdown": 0.05,
  "sharpe_ratio": 1.2
}
```

### Signal Metrics
```bash
curl http://localhost:3001/api/v1/metrics/signals
```
**Response:**
```json
{
  "timestamp": "2023-XX-XXTXX:XX:XX.XXXZ",
  "signals_processed": 127,
  "signals_per_minute": 2.1,
  "avg_confidence": 0.72,
  "avg_urgency": 0.58,
  "signal_distribution": {
    "Buy": 45,
    "Sell": 32,
    "Hold": 35,
    "Close": 15
  },
  "pattern_strength_avg": 0.78,
  "spike_count_avg": 145.0,
  "volatility_avg": 0.032,
  "market_regimes": {
    "strong_uptrend": 25,
    "mild_uptrend": 18,
    "consolidation": 40,
    "weak_downtrend": 12,
    "risk_off": 8
  }
}
```

### All Metrics
```bash
curl http://localhost:3001/api/v1/metrics/all
```

### Health Check
```bash
curl http://localhost:3001/health
```

## Dashboard Panels

### 1. Portfolio Overview
- **Total Capital**: Current portfolio value
- **Total P&L**: Profit and loss with color coding
- **Win Rate**: Gauge showing success percentage
- **Signals Processed**: Count of neuromorphic signals

### 2. Time Series Charts
- **Portfolio P&L Over Time**: Real-time P&L tracking
- **Signal Confidence Over Time**: Neuromorphic confidence levels

### 3. Distribution Charts
- **Signal Distribution**: Pie chart of Buy/Sell/Hold/Close signals
- **Market Regimes**: Distribution of detected market conditions

### 4. Neuromorphic Metrics Table
- Average confidence levels
- Pattern strength analysis
- Spike count statistics
- Volatility measurements

## Advanced Configuration

### Custom Alerts

Set up alerts based on trading performance:

```yaml
# Grafana Alert Rules
- alert: Low Win Rate
  expr: win_rate < 0.4
  for: 5m
  annotations:
    summary: "Trading win rate below 40%"

- alert: High Drawdown
  expr: max_drawdown > 0.1
  for: 2m
  annotations:
    summary: "Portfolio drawdown exceeding 10%"

- alert: Signal Processing Stopped
  expr: increase(signals_processed[5m]) == 0
  for: 3m
  annotations:
    summary: "No new signals processed in 5 minutes"
```

### Custom Data Source Configuration

For production deployments, you may want to configure authentication:

```json
{
  "url": "http://your-server:3001/api/v1/metrics",
  "httpMethod": "GET",
  "httpHeaderName1": "Authorization",
  "httpHeaderValue1": "Bearer YOUR_API_TOKEN"
}
```

### Time Series Data

The system provides time series endpoints for historical data:

```bash
# Portfolio P&L time series
curl "http://localhost:3001/api/v1/timeseries/portfolio_pnl?from=1640995200&to=1641081600"

# Signal confidence time series  
curl "http://localhost:3001/api/v1/timeseries/signal_confidence?from=1640995200&to=1641081600"
```

## Troubleshooting

### Common Issues

1. **Connection Failed**
   - Ensure the neuromorphic trading demo is running
   - Check that port 3001 is accessible
   - Verify no firewall blocking the connection

2. **No Data Showing**
   - Confirm the trading system is processing signals
   - Check the time range in Grafana matches data availability
   - Verify the data source URL is correct

3. **Performance Issues**
   - Reduce refresh rate from 5s to 10s or 30s
   - Limit time range for historical queries
   - Consider using caching for high-frequency updates

### Debug Commands

```bash
# Test API endpoints
curl -v http://localhost:3001/health
curl -v http://localhost:3001/api/v1/metrics/portfolio

# Check logs
tail -f your-trading-system.log

# Monitor network traffic
netstat -tuln | grep 3001
```

## Production Deployment

### Security Considerations

1. **API Authentication**: Add API key validation
2. **HTTPS**: Use TLS encryption for production
3. **Rate Limiting**: Implement request throttling
4. **CORS**: Configure appropriate CORS policies

### Scalability

1. **Database Storage**: Store metrics in InfluxDB or Prometheus
2. **Load Balancing**: Use multiple API server instances
3. **Caching**: Implement Redis for frequently accessed data
4. **Monitoring**: Add health checks and performance metrics

### Example Production Configuration

```rust
// Production metrics API with authentication
use warp::header;

let auth_header = header::exact("authorization", "Bearer YOUR_SECRET_TOKEN");

let authenticated_routes = portfolio_metrics
    .or(signal_metrics)
    .and(auth_header);

warp::serve(authenticated_routes)
    .tls()
    .cert_path("cert.pem")
    .key_path("key.pem")
    .run(([0, 0, 0, 0], 3001))
    .await;
```

## Next Steps

1. **Custom Panels**: Create application-specific visualizations
2. **Integration**: Connect with other monitoring tools
3. **Automation**: Set up automated reporting
4. **Machine Learning**: Use Grafana ML for anomaly detection

For advanced customization, refer to the [Grafana Documentation](https://grafana.com/docs/) and explore the neuromorphic-specific metrics provided by the trading system.