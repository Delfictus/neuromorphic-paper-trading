# Podman Grafana Connectivity Fix

## Problem
Grafana running in Podman container cannot access `localhost:3002` because localhost refers to the container, not the host.

## Solution 1: Use host.containers.internal (Recommended)

### Update Grafana Data Source:
- **URL**: `http://host.containers.internal:3002`
- This special hostname allows containers to access the host machine

### Update Dashboard URLs:
Change all panel URLs from:
```
/api/v1/metrics/portfolio
```
To:
```
http://host.containers.internal:3002/api/v1/metrics/portfolio
```

## Solution 2: Use Host Networking

### Stop and restart Grafana with host networking:
```bash
# Stop current container
podman stop grafana

# Start with host networking
podman run -d \
  --name grafana \
  --network host \
  -v grafana-storage:/var/lib/grafana \
  grafana/grafana:latest
```

With host networking, use: `http://localhost:3002`

## Solution 3: Use Host IP Address

### Find your host IP:
```bash
ip route get 1.1.1.1 | awk '{print $7}'
```

### Use the IP in Grafana:
- **URL**: `http://YOUR_HOST_IP:3002`

## Verification Commands

### Test from host:
```bash
curl http://localhost:3002/api/v1/metrics/portfolio
```

### Test host.containers.internal from container:
```bash
podman exec grafana curl http://host.containers.internal:3002/api/v1/metrics/portfolio
```