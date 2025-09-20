#!/bin/bash

# Neuromorphic Trading System - Pre-flight Check
echo "🔍 Verifying Neuromorphic Trading System Setup"
echo ""

# Check Docker/Podman
echo "📦 Checking container runtime..."
if command -v docker &> /dev/null; then
    echo "✅ Docker found: $(docker --version)"
elif command -v podman &> /dev/null; then
    echo "✅ Podman found: $(podman --version)"
else
    echo "❌ No container runtime found (Docker/Podman required)"
    exit 1
fi

# Check Docker Compose
if command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
    echo "✅ Docker Compose found: $(docker-compose --version)"
elif command -v docker &> /dev/null && docker compose version &> /dev/null; then
    COMPOSE_CMD="docker compose"
    echo "✅ Docker Compose found: $(docker compose version)"
elif command -v podman-compose &> /dev/null; then
    COMPOSE_CMD="podman-compose"
    echo "✅ Podman Compose found: $(podman-compose --version)"
else
    echo "❌ No compose tool found"
    exit 1
fi

# Validate compose file
echo ""
echo "🔧 Validating docker-compose.yml..."
if $COMPOSE_CMD config --quiet; then
    echo "✅ Docker Compose configuration is valid"
else
    echo "❌ Docker Compose configuration has errors"
    exit 1
fi

# Check required files
echo ""
echo "📄 Checking required files..."
files=(
    "docker-compose.yml"
    "start-trading-system.sh"
    "stop-trading-system.sh"
    "metrics_server.py"
    "neuromorphic-core/examples/autonomous_trader.rs"
    "neuromorphic-core/Cargo.toml"
)

for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        echo "✅ $file"
    else
        echo "❌ Missing: $file"
        exit 1
    fi
done

# Check if ports are available
echo ""
echo "🔌 Checking port availability..."
ports=(3000 3001 3002 9090)
for port in "${ports[@]}"; do
    if command -v netstat &> /dev/null; then
        if netstat -tuln | grep -q ":$port "; then
            echo "⚠️  Port $port is in use (may need to stop existing service)"
        else
            echo "✅ Port $port available"
        fi
    elif command -v ss &> /dev/null; then
        if ss -tuln | grep -q ":$port "; then
            echo "⚠️  Port $port is in use (may need to stop existing service)"
        else
            echo "✅ Port $port available"
        fi
    else
        echo "🤷 Port $port status unknown (netstat/ss not available)"
    fi
done

echo ""
echo "🎯 System Verification Complete!"
echo ""
echo "🚀 To start the system:"
echo "   ./start-trading-system.sh"
echo ""
echo "📊 Once running, access:"
echo "   🤖 Trading System: Automatic (view logs: $COMPOSE_CMD logs -f neuromorphic-trader)"
echo "   📈 Grafana Dashboard: http://localhost:3000"
echo "   📊 Metrics API: http://localhost:3002"
echo "   🔍 Trading Metrics: http://localhost:3001"
echo ""
echo "🛑 To stop the system:"
echo "   ./stop-trading-system.sh"