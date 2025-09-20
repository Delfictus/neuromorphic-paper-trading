#!/bin/bash

# Neuromorphic Paper Trading System - Stop Script
echo "🛑 Stopping Neuromorphic Autonomous Trading System"

# Check if docker compose is available
if command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
elif command -v docker &> /dev/null && docker compose version &> /dev/null; then
    COMPOSE_CMD="docker compose"
else
    echo "❌ Error: Neither 'docker-compose' nor 'docker compose' found"
    exit 1
fi

echo "🔧 Using: $COMPOSE_CMD"

# Stop all services
echo "🛑 Stopping all services..."
$COMPOSE_CMD down

if [ $? -eq 0 ]; then
    echo "✅ System stopped successfully!"
    echo ""
    echo "📊 To restart: ./start-trading-system.sh"
    echo "🗑️  To remove all data: $COMPOSE_CMD down -v"
else
    echo "❌ Error stopping system"
    exit 1
fi