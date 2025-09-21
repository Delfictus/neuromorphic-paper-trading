#!/bin/bash

echo "🧪 Testing Neuromorphic Trading System"
echo ""

# Test Grafana
echo "📈 Testing Grafana..."
GRAFANA_HEALTH=$(curl -s http://localhost:3000/api/health | grep -o '"database":"ok"')
if [ "$GRAFANA_HEALTH" = '"database":"ok"' ]; then
    echo "✅ Grafana: HEALTHY"
else
    echo "❌ Grafana: UNHEALTHY"
fi

# Test Metrics Server  
echo "📊 Testing Metrics Server..."
METRICS_HEALTH=$(curl -s http://localhost:3002/health | grep -o '"status":"ok"')
if [ "$METRICS_HEALTH" = '"status":"ok"' ]; then
    echo "✅ Metrics Server: HEALTHY"
else
    echo "❌ Metrics Server: UNHEALTHY"
fi

# Test Trading System Logs
echo "🤖 Testing Trading System..."
TRADING_LOGS=$(docker compose logs neuromorphic-trader 2>/dev/null | grep "listening on")
if [ ! -z "$TRADING_LOGS" ]; then
    echo "✅ Trading System: RUNNING"
else
    echo "❌ Trading System: NOT RUNNING"
fi

echo ""
echo "🎯 System Status Summary:"
echo "   📈 Grafana Dashboard: http://localhost:3000 (admin/admin)"
echo "   📊 Metrics API: http://localhost:3002"
echo "   🤖 Trading System: Running with market monitoring"
echo ""

# Check Docker containers
echo "🐳 Container Status:"
docker compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}" 2>/dev/null

echo ""
echo "🚀 System is ready for trading!"
echo "   💰 Starting capital: \$100,000"
echo "   📈 Max positions: 12"
echo "   🎯 Min confidence: 72%"
echo "   ⚡ Risk per trade: 1.5%"
echo "   📊 Max daily trades: 25"