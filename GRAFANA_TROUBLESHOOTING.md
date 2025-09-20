# Grafana Dashboard Troubleshooting Guide

## Current Issue: Dashboard comes in empty / "cannot read property of undefined method"

### Step 1: Set Up Data Source First
Before importing any dashboard, set up the Infinity data source:

1. Go to **Configuration** → **Data sources**
2. Click **Add data source**
3. Select **Infinity**
4. Configure exactly:
   - **Name**: `Neuromorphic Metrics`
   - **URL**: `http://127.0.0.1:3002`
   - **Access**: Server (default)
   - **Auth**: None
5. Click **Save & Test** - should show green checkmark

### Step 2: Test API Manually
Verify the API is working:
```bash
curl http://127.0.0.1:3002/api/v1/metrics/portfolio
curl http://127.0.0.1:3002/api/v1/metrics/signals
```

### Step 3: Import Dashboard
Try these dashboards in order:

1. **First try**: `neuromorphic-working-dashboard.json` (simplified, 4 panels)
2. **If that fails**: `neuromorphic-dashboard-fixed.json` (full dashboard)

### Step 4: Dashboard Import Process
1. Go to **Dashboards** → **Import**
2. Click **Upload JSON file**
3. Select the dashboard file
4. **Important**: When prompted for data source, select "Neuromorphic Metrics"
5. Click **Import**

### Step 5: If Panels Show "No Data"
1. Click on a panel title → **Edit**
2. Check the **Query** tab:
   - URL should be relative: `/api/v1/metrics/portfolio`
   - Data source should be: `Neuromorphic Metrics`
3. Click **Apply**

### Step 6: Debug Panel Configuration
If panels are empty, manually configure one:

1. Create new panel
2. Select data source: **Neuromorphic Metrics**
3. Query type: **JSON**
4. URL: `/api/v1/metrics/portfolio`
5. Format: **Table**
6. Parser: **Backend**
7. Add column:
   - Selector: `total_capital`
   - Text: `Total Capital`
   - Type: `number`

### Common Issues:
- **Empty panels**: Data source not properly linked during import
- **"Cannot read property"**: Dashboard JSON structure issue
- **Connection refused**: API server not running or wrong URL
- **No data**: Wrong endpoint URLs or data source configuration

### API Endpoints Available:
- `/api/v1/metrics/portfolio` - Portfolio metrics
- `/api/v1/metrics/signals` - Signal processing metrics  
- `/health` - Health check