#!/usr/bin/env python3
"""
Prometheus-compatible metrics server for neuromorphic trading
This creates metrics in Prometheus format that Grafana can scrape directly
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import time
import random

class PrometheusMetricsHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/metrics':
            self.send_response(200)
            self.send_header('Content-type', 'text/plain; version=0.0.4')
            self.end_headers()
            
            # Generate Prometheus format metrics
            metrics = self.generate_prometheus_metrics()
            self.wfile.write(metrics.encode())
        else:
            self.send_response(404)
            self.end_headers()
    
    def generate_prometheus_metrics(self):
        # Current timestamp
        timestamp = int(time.time() * 1000)
        
        # Simulate some variance in metrics
        variance = random.uniform(-0.1, 0.1)
        
        metrics = f"""# HELP neuromorphic_portfolio_capital_total Total portfolio capital in USD
# TYPE neuromorphic_portfolio_capital_total gauge
neuromorphic_portfolio_capital_total {102500.0 + variance * 1000}

# HELP neuromorphic_portfolio_pnl_total Total profit and loss in USD
# TYPE neuromorphic_portfolio_pnl_total gauge
neuromorphic_portfolio_pnl_total {2500.0 + variance * 500}

# HELP neuromorphic_portfolio_return_pct Portfolio return percentage
# TYPE neuromorphic_portfolio_return_pct gauge
neuromorphic_portfolio_return_pct {2.5 + variance * 0.5}

# HELP neuromorphic_portfolio_win_rate Win rate as percentage
# TYPE neuromorphic_portfolio_win_rate gauge
neuromorphic_portfolio_win_rate {60.0 + variance * 5}

# HELP neuromorphic_portfolio_positions_total Number of total positions
# TYPE neuromorphic_portfolio_positions_total gauge
neuromorphic_portfolio_positions_total 5

# HELP neuromorphic_portfolio_positions_active Number of active positions
# TYPE neuromorphic_portfolio_positions_active gauge
neuromorphic_portfolio_positions_active 3

# HELP neuromorphic_portfolio_sharpe_ratio Sharpe ratio of the portfolio
# TYPE neuromorphic_portfolio_sharpe_ratio gauge
neuromorphic_portfolio_sharpe_ratio {1.2 + variance * 0.2}

# HELP neuromorphic_signals_processed_total Total number of signals processed
# TYPE neuromorphic_signals_processed_total counter
neuromorphic_signals_processed_total {127 + int(time.time()) % 10}

# HELP neuromorphic_signals_confidence_avg Average confidence of signals (0-100)
# TYPE neuromorphic_signals_confidence_avg gauge
neuromorphic_signals_confidence_avg {72.0 + variance * 5}

# HELP neuromorphic_signals_urgency_avg Average urgency of signals (0-100)
# TYPE neuromorphic_signals_urgency_avg gauge
neuromorphic_signals_urgency_avg {58.0 + variance * 8}

# HELP neuromorphic_signals_pattern_strength_avg Average pattern strength (0-100)
# TYPE neuromorphic_signals_pattern_strength_avg gauge
neuromorphic_signals_pattern_strength_avg {78.0 + variance * 6}

# HELP neuromorphic_signals_spike_count_avg Average spike count in signals
# TYPE neuromorphic_signals_spike_count_avg gauge
neuromorphic_signals_spike_count_avg {145.0 + variance * 20}

# HELP neuromorphic_signals_volatility_avg Average volatility percentage
# TYPE neuromorphic_signals_volatility_avg gauge
neuromorphic_signals_volatility_avg {3.2 + variance * 0.5}

# HELP neuromorphic_signals_per_minute Rate of signal processing per minute
# TYPE neuromorphic_signals_per_minute gauge
neuromorphic_signals_per_minute {2.1 + variance * 0.3}

# HELP neuromorphic_signal_distribution Signal distribution by type
# TYPE neuromorphic_signal_distribution gauge
neuromorphic_signal_distribution{{type="buy"}} 45
neuromorphic_signal_distribution{{type="sell"}} 32
neuromorphic_signal_distribution{{type="hold"}} 35
neuromorphic_signal_distribution{{type="close"}} 15

# HELP neuromorphic_market_regime Market regime detection
# TYPE neuromorphic_market_regime gauge
neuromorphic_market_regime{{regime="strong_uptrend"}} 25
neuromorphic_market_regime{{regime="mild_uptrend"}} 18
neuromorphic_market_regime{{regime="consolidation"}} 40
neuromorphic_market_regime{{regime="weak_downtrend"}} 12
neuromorphic_market_regime{{regime="risk_off"}} 8
"""
        return metrics

if __name__ == '__main__':
    server = HTTPServer(('localhost', 9090), PrometheusMetricsHandler)
    print("🚀 Prometheus Metrics Server starting on http://localhost:9090")
    print("📊 Metrics endpoint: http://localhost:9090/metrics")
    print("📈 Ready for Grafana Prometheus data source!")
    print("\nTo use in Grafana:")
    print("1. Add Prometheus data source")
    print("2. URL: http://localhost:9090")
    print("3. Scrape interval: 15s")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n🔴 Server stopped")
        server.server_close()