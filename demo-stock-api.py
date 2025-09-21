#!/usr/bin/env python3
"""
Demo Stock API Server
Provides mock stock data for Grafana dashboard demonstration
"""

import json
import time
import random
from datetime import datetime, timedelta
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

class StockAPIHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed_path = urlparse(self.path)
        path = parsed_path.path
        query = parse_qs(parsed_path.query)
        
        # Enable CORS
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
        self.end_headers()
        
        try:
            if path == '/stocks':
                response = self.get_stocks()
            elif path.startswith('/') and path.endswith('/history'):
                symbol = path.split('/')[1]
                hours = int(query.get('hours', [24])[0])
                response = self.get_stock_history(symbol, hours)
            elif path == '/health':
                response = {"status": "ok", "service": "demo-stock-api"}
            else:
                response = {"error": "Endpoint not found"}
                
            self.wfile.write(json.dumps(response).encode())
        except Exception as e:
            error_response = {"error": str(e)}
            self.wfile.write(json.dumps(error_response).encode())
    
    def get_stocks(self):
        """Generate demo stock data"""
        stocks = [
            {"symbol": "AAPL", "base_price": 175.50, "volatility": 0.02},
            {"symbol": "MSFT", "base_price": 342.80, "volatility": 0.015},
            {"symbol": "GOOGL", "base_price": 138.45, "volatility": 0.025},
            {"symbol": "TSLA", "base_price": 242.80, "volatility": 0.04},
            {"symbol": "NVDA", "base_price": 465.20, "volatility": 0.03},
            {"symbol": "META", "base_price": 298.75, "volatility": 0.025},
            {"symbol": "AMZN", "base_price": 127.35, "volatility": 0.02},
            {"symbol": "NFLX", "base_price": 445.60, "volatility": 0.035},
        ]
        
        result = []
        for stock in stocks:
            # Add some random price movement
            price_change = random.uniform(-0.05, 0.05)
            current_price = stock["base_price"] * (1 + price_change)
            change_24h = price_change * 100
            
            result.append({
                "symbol": stock["symbol"],
                "price": round(current_price, 2),
                "change_24h": round(change_24h, 2),
                "volume": random.randint(5000000, 25000000),
                "volatility": round(stock["volatility"] * 100, 2),
                "last_updated": datetime.now().isoformat(),
                "trend": "up" if change_24h > 0 else "down" if change_24h < 0 else "flat"
            })
        
        return {
            "stocks": result,
            "total_monitored": len(result),
            "last_updated": datetime.now().isoformat()
        }
    
    def get_stock_history(self, symbol, hours=24):
        """Generate demo price history for a stock"""
        base_prices = {
            "AAPL": 175.50, "MSFT": 342.80, "GOOGL": 138.45, "TSLA": 242.80,
            "NVDA": 465.20, "META": 298.75, "AMZN": 127.35, "NFLX": 445.60
        }
        
        base_price = base_prices.get(symbol, 100.0)
        history = []
        
        for i in range(hours):
            timestamp = datetime.now() - timedelta(hours=hours-i)
            # Generate realistic price movement
            price_variation = random.uniform(-0.02, 0.02) + (i % 7 - 3) * 0.005
            price = base_price * (1 + price_variation)
            
            history.append({
                "timestamp": int(timestamp.timestamp() * 1000),
                "price": round(price, 2),
                "volume": random.randint(1000000, 5000000)
            })
        
        return {
            "symbol": symbol,
            "timeframe_hours": hours,
            "data_points": len(history),
            "price_history": history
        }

def run_server(port=3003):
    """Run the demo stock API server"""
    server_address = ('', port)
    httpd = HTTPServer(server_address, StockAPIHandler)
    print(f"🚀 Demo Stock API server running on port {port}")
    print(f"📊 Available endpoints:")
    print(f"   GET /stocks - Current stock data")
    print(f"   GET /{'{symbol}'}/history?hours=24 - Stock price history")
    print(f"   GET /health - Health check")
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n🛑 Server stopped")
        httpd.server_close()

if __name__ == '__main__':
    run_server()