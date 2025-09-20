#!/usr/bin/env python3
"""
Simple HTTP server that provides neuromorphic trading metrics for Grafana
This serves as a bridge until we get the full Rust API working
"""

import json
import datetime
import random
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

class MetricsHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed_path = urlparse(self.path)
        
        # Enable CORS for Grafana
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        
        if parsed_path.path == '/health':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {
                "status": "ok",
                "service": "neuromorphic-trading-metrics",
                "timestamp": datetime.datetime.now().isoformat()
            }
            self.wfile.write(json.dumps(response).encode())
            
        elif parsed_path.path == '/api/v1/metrics/portfolio':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            
            # Simulate realistic portfolio metrics
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
                "win_rate": 0.6,
                "avg_win": 400.0,
                "avg_loss": -200.0,
                "max_drawdown": 0.05,
                "sharpe_ratio": 1.2
            }
            self.wfile.write(json.dumps(response).encode())
            
        elif parsed_path.path == '/api/v1/metrics/signals':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            
            # Simulate neuromorphic signal metrics
            response = {
                "timestamp": datetime.datetime.now().isoformat(),
                "signals_processed": 127 + random.randint(0, 10),
                "signals_per_minute": 2.1 + random.uniform(-0.5, 0.5),
                "avg_confidence": 0.72 + random.uniform(-0.1, 0.1),
                "avg_urgency": 0.58 + random.uniform(-0.1, 0.1),
                "signal_distribution": {
                    "Buy": 45,
                    "Sell": 32,
                    "Hold": 35,
                    "Close": 15
                },
                "pattern_strength_avg": 0.78 + random.uniform(-0.05, 0.05),
                "spike_count_avg": 145.0 + random.uniform(-20, 20),
                "volatility_avg": 0.032 + random.uniform(-0.005, 0.005),
                "market_regimes": {
                    "strong_uptrend": 25,
                    "mild_uptrend": 18,
                    "consolidation": 40,
                    "weak_downtrend": 12,
                    "risk_off": 8
                }
            }
            self.wfile.write(json.dumps(response).encode())
            
        elif parsed_path.path == '/api/v1/metrics/all':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            
            # Combined metrics response
            response = {
                "portfolio": {
                    "timestamp": datetime.datetime.now().isoformat(),
                    "total_capital": 102500.0,
                    "total_pnl": 2500.0,
                    "total_return_pct": 2.5,
                    "win_rate": 0.6,
                    "positions_count": 5,
                    "active_positions_count": 3
                },
                "signals": {
                    "timestamp": datetime.datetime.now().isoformat(),
                    "signals_processed": 127,
                    "avg_confidence": 0.72,
                    "avg_urgency": 0.58,
                    "pattern_strength_avg": 0.78
                },
                "positions": [],
                "market_data": [],
                "risk": {
                    "timestamp": datetime.datetime.now().isoformat(),
                    "sharpe_ratio": 1.2,
                    "max_drawdown": 0.05
                }
            }
            self.wfile.write(json.dumps(response).encode())
            
        else:
            self.send_response(404)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {"error": "Endpoint not found"}
            self.wfile.write(json.dumps(response).encode())
    
    def do_OPTIONS(self):
        # Handle CORS preflight requests
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()

if __name__ == '__main__':
    server = HTTPServer(('localhost', 3001), MetricsHandler)
    print("🚀 Neuromorphic Trading Metrics Server starting on http://localhost:3001")
    print("📊 Available endpoints:")
    print("   - http://localhost:3001/health")
    print("   - http://localhost:3001/api/v1/metrics/portfolio")
    print("   - http://localhost:3001/api/v1/metrics/signals")
    print("   - http://localhost:3001/api/v1/metrics/all")
    print("📈 Ready for Grafana integration!")
    server.serve_forever()