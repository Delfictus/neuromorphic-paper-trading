# Podman Host Networking Solution

## Problem Identified
- Metrics server is running correctly on `0.0.0.0:3002`
- Podman container networking prevents access to host services
- Bridge network gateway routing not working properly

## Recommended Solution: Use Host Networking

### Step 1: Stop Current Grafana Container
```bash
podman stop grafana
podman rm grafana
```

### Step 2: Start Grafana with Host Networking
```bash
podman run -d \
  --name grafana \
  --network host \
  -v grafana-storage:/var/lib/grafana \
  -e "GF_SECURITY_ADMIN_PASSWORD=admin" \
  docker.io/grafana/grafana:latest
```

### Step 3: Access Grafana
- URL: `http://localhost:3000`
- Username: `admin`
- Password: `admin`

### Step 4: Configure Data Source
With host networking, use:
- **Data Source URL**: `http://localhost:3002`
- **Test endpoints**: Will work directly from container

### Step 5: Import Dashboard
Use `neuromorphic-working-dashboard.json` with:
- Panel URLs: `/api/v1/metrics/portfolio`
- Data source: `http://localhost:3002`

## Verification
```bash
# Test from host
curl http://localhost:3002/health

# After host networking, test from container should work
podman exec grafana curl http://localhost:3002/health
```

## Alternative: Port Mapping Solution
If you prefer to keep bridge networking:

```bash
# Stop current container
podman stop grafana

# Start with port mapping and add-host
podman run -d \
  --name grafana \
  -p 3000:3000 \
  --add-host=host.docker.internal:host-gateway \
  -v grafana-storage:/var/lib/grafana \
  docker.io/grafana/grafana:latest
```

Then use URL: `http://host.docker.internal:3002`