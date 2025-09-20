#!/bin/bash

# Neuromorphic Paper Trading System - Quick Start
echo "🚀 Starting Neuromorphic Autonomous Trading System"
echo "📊 This will launch the complete system with monitoring"
echo ""

# Check if docker compose is available
if command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
elif command -v docker &> /dev/null && docker compose version &> /dev/null; then
    COMPOSE_CMD="docker compose"
else
    echo "❌ Error: Neither 'docker-compose' nor 'docker compose' found"
    echo "Please install Docker and Docker Compose first"
    exit 1
fi

echo "🔧 Using: $COMPOSE_CMD"
echo ""

# Build and start all services
echo "🏗️  Building and starting services..."
$COMPOSE_CMD up -d --build

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ System started successfully!"
    echo ""
    echo "📊 Services Available:"
    echo "   🤖 Autonomous Trading System: Running (logs: $COMPOSE_CMD logs -f neuromorphic-trader)"
    echo "   📈 Grafana Dashboard: http://localhost:3000 (admin/admin)"
    echo "   📊 Metrics API: http://localhost:3002"
    echo "   🔍 Trading Metrics: http://localhost:3001"
    echo ""
    echo "📋 Useful Commands:"
    echo "   📊 View trading logs: $COMPOSE_CMD logs -f neuromorphic-trader"
    echo "   📈 View all logs: $COMPOSE_CMD logs -f"
    echo "   🛑 Stop system: $COMPOSE_CMD down"
    echo "   🔄 Restart: $COMPOSE_CMD restart neuromorphic-trader"
    echo ""
    echo "🎮 The autonomous trading system is now monitoring the entire stock market!"
    echo "   💰 Starting capital: $100,000"
    echo "   🎯 Min confidence threshold: 72%"
    echo "   ⚡ Risk per trade: 1.5%"
    echo "   📊 Max daily trades: 25"
    echo ""
    echo "Press Ctrl+C to stop, or run: $COMPOSE_CMD down"
    
    # Follow logs
    echo ""
    echo "📊 Following trading system logs (Ctrl+C to stop viewing logs):"
    $COMPOSE_CMD logs -f neuromorphic-trader
else
    echo "❌ Failed to start system"
    echo "📋 Check logs: $COMPOSE_CMD logs"
    exit 1
fi