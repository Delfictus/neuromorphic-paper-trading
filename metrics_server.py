#!/usr/bin/env python3
"""
Simple HTTP server for neuromorphic trading metrics - Grafana integration
"""

import json
import datetime
import random
from http.server import HTTPServer, BaseHTTPRequestHandler

class MetricsHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()
        
        if self.path == '/health':
            response = {
                "status": "ok",
                "service": "neuromorphic-trading-metrics",
                "timestamp": datetime.datetime.now().isoformat()
            }
        elif self.path == '/api/v1/metrics/portfolio':
            response = {
                "timestamp": datetime.datetime.now().isoformat(),
                "total_capital": 102500.0,
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
                "win_rate": 60.0,
                "avg_win": 400.0,
                "avg_loss": 200.0,
                "max_drawdown": 5.0,
                "sharpe_ratio": 1.2
            }
        elif self.path == '/api/v1/metrics/signals':
            response = {
                "timestamp": datetime.datetime.now().isoformat(),
                "signals_processed": 127,
                "signals_per_minute": 2.1,
                "avg_confidence": 72.0,
                "avg_urgency": 58.0,
                "signal_distribution": {
                    "Buy": 45,
                    "Sell": 32,
                    "Hold": 35,
                    "Close": 15
                },
                "pattern_strength_avg": 78.0,
                "spike_count_avg": 145.0,
                "volatility_avg": 3.2,
                "market_regimes": {
                    "strong_uptrend": 25,
                    "mild_uptrend": 18,
                    "consolidation": 40,
                    "weak_downtrend": 12,
                    "risk_off": 8
                }
            }
        else:
            response = {"error": "Endpoint not found"}
            
        self.wfile.write(json.dumps(response, indent=2).encode())
    
    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()

if __name__ == '__main__':
    server = HTTPServer(('0.0.0.0', 3002), MetricsHandler)
    print("🚀 Neuromorphic Trading Metrics Server starting on http://localhost:3002")
    print("📊 Available endpoints:")
    print("   - http://localhost:3002/health")
    print("   - http://localhost:3002/api/v1/metrics/portfolio")
    print("   - http://localhost:3002/api/v1/metrics/signals")
    print("📈 Ready for Grafana integration!")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n🔴 Server stopped")
        server.server_close()