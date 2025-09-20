# Podman Compose Setup for Neuromorphic Trading Dashboard

## Overview
Complete containerized setup with:
- **Metrics Server**: Python API serving trading data
- **Grafana**: Dashboard visualization with auto-provisioning
- **Networking**: Bridge network for service communication
- **Volumes**: Persistent Grafana data storage

## Quick Start

### 1. Stop Existing Services
```bash
# Stop any running containers
podman stop grafana metrics-server prometheus-metrics 2>/dev/null || true
podman rm grafana metrics-server prometheus-metrics 2>/dev/null || true

# Stop background Python servers
pkill -f "python.*metrics_server"
```

### 2. Start with Podman Compose
```bash
# Start all services
podman-compose up -d

# View logs
podman-compose logs -f

# Check status
podman-compose ps
```

### 3. Access Dashboard
- **Grafana**: http://localhost:3000
- **Username**: `admin`
- **Password**: `admin`
- **Metrics API**: http://localhost:3002

## Services Included

### Metrics Server (`metrics-server`)
- **Port**: 3002
- **Endpoints**: 
  - `/health` - Health check
  - `/api/v1/metrics/portfolio` - Portfolio data
  - `/api/v1/metrics/signals` - Signal analytics
- **Health Check**: Automatic monitoring

### Grafana (`grafana`)
- **Port**: 3000
- **Features**:
  - Auto-installs Infinity plugin
  - Pre-configured data source
  - Auto-provisions dashboard
  - Persistent data storage

### Prometheus Metrics (Optional)
- **Port**: 9090
- **Profile**: `prometheus`
- **Start**: `podman-compose --profile prometheus up -d`

## Configuration Files

### Data Source Auto-Provisioning
- File: `grafana/provisioning/datasources/neuromorphic.yml`
- URL: `http://metrics-server:3002`
- Type: Infinity plugin

### Dashboard Auto-Provisioning
- File: `grafana/provisioning/dashboards/dashboard.yml`
- Dashboard: `grafana/neuromorphic-container-dashboard.json`

## Management Commands

```bash
# Start services
podman-compose up -d

# Stop services
podman-compose down

# Restart specific service
podman-compose restart grafana

# View service logs
podman-compose logs metrics-server
podman-compose logs grafana

# Scale services
podman-compose up -d --scale metrics-server=2

# Remove everything (including volumes)
podman-compose down -v
```

## Networking
- **Network**: `neuromorphic-net` (bridge)
- **Service Discovery**: Automatic DNS resolution
- **Inter-service URLs**: `http://metrics-server:3002`

## Volume Mounts
- **Grafana Data**: `grafana-data` volume → `/var/lib/grafana`
- **Provisioning**: `./grafana/provisioning` → `/etc/grafana/provisioning`
- **Dashboards**: `./grafana` → `/etc/grafana/provisioning/dashboards`

## Troubleshooting

### Service Won't Start
```bash
# Check logs
podman-compose logs <service-name>

# Check port conflicts
ss -tlnp | grep -E "(3000|3002|9090)"
```

### Dashboard Shows No Data
```bash
# Test metrics API from Grafana container
podman exec neuromorphic-paper-trader_grafana_1 curl http://metrics-server:3002/health

# Check data source configuration
# Go to Configuration → Data Sources → Neuromorphic Metrics
```

### Reset Everything
```bash
# Complete reset
podman-compose down -v
podman volume rm grafana-data
podman-compose up -d
```

## Security Notes
- Default admin password is `admin` - change in production
- Anonymous access enabled for demo purposes
- No SSL/TLS configured - add for production use

## Development
- Mount source code: Add volume mount for live development
- Debug mode: Set `RUST_LOG=debug` for Rust components
- Hot reload: Grafana auto-reloads provisioned dashboards