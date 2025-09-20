# Final Grafana Setup Steps

## Current Status
✅ Metrics server running on port 3002  
✅ Dashboard JSON file created with fixed URLs  
✅ API endpoints tested and working  

## Complete These Steps in Grafana:

### 1. Configure Data Source
- Go to **Configuration** → **Data sources**
- Click **Add data source**
- Select **Infinity** (should be already installed)
- Configure:
  - **Name**: `Neuromorphic Metrics`
  - **URL**: `http://127.0.0.1:3002`
  - **Auth**: None needed
- Click **Save & Test**

### 2. Import Dashboard
- Go to **Dashboards** → **Import**
- Upload the file: `grafana/neuromorphic-dashboard-fixed.json`
- Select data source: `Neuromorphic Metrics`
- Click **Import**

### 3. Verify Dashboard
You should see 7 panels displaying:
- 💰 Portfolio Value: $102,500
- 📈 Total P&L: $2,500  
- 🎯 Win Rate: 60%
- 🧠 Signals Processed: 127
- 🔮 Signal Confidence: 72%
- ⚡ Pattern Strength: 78%
- 📊 Portfolio Summary (table)

## Test API Endpoints
```bash
# Portfolio metrics
curl http://127.0.0.1:3002/api/v1/metrics/portfolio | jq

# Signal metrics  
curl http://127.0.0.1:3002/api/v1/metrics/signals | jq

# Health check
curl http://127.0.0.1:3002/health | jq
```

## Troubleshooting
- If panels show "No Data": Check data source URL is exactly `http://127.0.0.1:3002`
- If connection fails: Ensure metrics server is running on port 3002
- If URLs look malformed: Use relative paths in panel queries (already fixed in JSON)

## Auto-refresh
Dashboard is configured to refresh every 5 seconds for real-time updates.