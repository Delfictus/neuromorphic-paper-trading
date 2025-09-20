# Grafana TestData Setup for Neuromorphic Trading

## Using TestData DB (Built-in)

1. **Add TestData Data Source:**
   - Go to Configuration → Data Sources
   - Click "Add data source"
   - Select "TestData DB" (this is built into Grafana)
   - Name it "Neuromorphic Demo"

2. **Create Dashboard Panels:**

### Portfolio Value Panel (Stat visualization)
```
Query Type: Random Walk
Alias: Portfolio Value
Min: 100000
Max: 105000
Start Value: 102500
```

### Win Rate Panel (Gauge)
```
Query Type: Predictable Pulse
Alias: Win Rate
Min: 50
Max: 70
Start Value: 60
```

### Signals Processed Panel (Time Series)
```
Query Type: Random Walk
Alias: Signals Processed
Min: 120
Max: 130
Start Value: 127
```

### P&L Panel (Time Series)
```
Query Type: Random Walk
Alias: Total P&L
Min: 2000
Max: 3000
Start Value: 2500
```

## Simulated Neuromorphic Metrics

### Signal Confidence (Gauge)
```
Query Type: Predictable Pulse
Alias: Avg Confidence
Min: 65
Max: 80
Start Value: 72
```

### Pattern Strength (Time Series)
```
Query Type: Predictable Pulse
Alias: Pattern Strength
Min: 70
Max: 85
Start Value: 78
```