//! REST API for Grafana integration
//! 
//! Provides HTTP endpoints that Grafana can consume for real-time dashboards

use std::sync::Arc;
use warp::{Filter, Rejection, Reply};
use serde_json::json;

use crate::metrics::MetricsCollector;

/// API error types
#[derive(Debug)]
pub struct ApiError {
    pub message: String,
}

impl warp::reject::Reject for ApiError {}

/// API server for metrics endpoints
pub struct MetricsApiServer {
    metrics_collector: Arc<MetricsCollector>,
    port: u16,
}

impl MetricsApiServer {
    pub fn new(metrics_collector: Arc<MetricsCollector>, port: u16) -> Self {
        Self {
            metrics_collector,
            port,
        }
    }

    /// Start the metrics API server
    pub async fn start(&self) {
        let metrics = self.metrics_collector.clone();

        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| {
                warp::reply::json(&json!({
                    "status": "ok",
                    "service": "neuromorphic-trading-metrics",
                    "timestamp": chrono::Utc::now()
                }))
            });

        // Portfolio metrics endpoint
        let portfolio_metrics = warp::path!("api" / "v1" / "metrics" / "portfolio")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_portfolio_metrics);

        // Signal metrics endpoint
        let signal_metrics = warp::path!("api" / "v1" / "metrics" / "signals")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_signal_metrics);

        // All metrics endpoint
        let all_metrics = warp::path!("api" / "v1" / "metrics" / "all")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_all_metrics);

        // Position metrics endpoint
        let position_metrics = warp::path!("api" / "v1" / "metrics" / "positions")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_position_metrics);

        // Market data endpoint
        let market_metrics = warp::path!("api" / "v1" / "metrics" / "market")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_market_metrics);

        // Risk metrics endpoint
        let risk_metrics = warp::path!("api" / "v1" / "metrics" / "risk")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_risk_metrics);

        // Time series endpoint for Grafana's JSON datasource
        let timeseries = warp::path!("api" / "v1" / "timeseries" / String)
            .and(warp::get())
            .and(warp::query::<TimeseriesQuery>())
            .and(with_metrics(metrics.clone()))
            .and_then(get_timeseries_data);

        // Simple metrics endpoint for Grafana Infinity datasource
        let simple_metrics = warp::path("metrics")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_simple_metrics);

        // Opportunities endpoint for Grafana tables
        let opportunities = warp::path("opportunities")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_opportunities);

        // Monitored stocks endpoint
        let monitored_stocks = warp::path("stocks")
            .and(warp::get())
            .and(with_metrics(metrics.clone()))
            .and_then(get_monitored_stocks);

        // Stock price history endpoint
        let stock_history = warp::path!(String / "history")
            .and(warp::get())
            .and(warp::query::<HistoryQuery>())
            .and(with_metrics(metrics.clone()))
            .and_then(get_stock_history);

        // CORS for Grafana
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type", "authorization"])
            .allow_methods(vec!["GET", "POST", "OPTIONS"]);

        let routes = health
            .or(portfolio_metrics)
            .or(signal_metrics)
            .or(all_metrics)
            .or(position_metrics)
            .or(market_metrics)
            .or(risk_metrics)
            .or(timeseries)
            .or(simple_metrics)
            .or(opportunities)
            .or(monitored_stocks)
            .or(stock_history)
            .with(cors)
            .recover(handle_rejection);

        tracing::info!("Starting Metrics API server on port {}", self.port);
        warp::serve(routes)
            .run(([0, 0, 0, 0], self.port))
            .await;
    }
}

// Helper function to inject metrics collector
fn with_metrics(
    metrics: Arc<MetricsCollector>,
) -> impl Filter<Extract = (Arc<MetricsCollector>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || metrics.clone())
}

// Query parameters for timeseries endpoint
#[derive(serde::Deserialize)]
struct TimeseriesQuery {
    from: Option<i64>,
    to: Option<i64>,
    interval: Option<String>,
}

// Query parameters for stock history endpoint
#[derive(serde::Deserialize)]
struct HistoryQuery {
    hours: Option<u64>,
}

/// Get portfolio metrics
async fn get_portfolio_metrics(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let portfolio_metrics = metrics.get_portfolio_metrics();
    Ok(warp::reply::json(&portfolio_metrics))
}

/// Get signal metrics
async fn get_signal_metrics(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let signal_metrics = metrics.get_signal_metrics();
    Ok(warp::reply::json(&signal_metrics))
}

/// Get all metrics
async fn get_all_metrics(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let all_metrics = metrics.get_all_metrics();
    Ok(warp::reply::json(&all_metrics))
}

/// Get position metrics
async fn get_position_metrics(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let all_metrics = metrics.get_all_metrics();
    Ok(warp::reply::json(&all_metrics.positions))
}

/// Get market metrics
async fn get_market_metrics(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let all_metrics = metrics.get_all_metrics();
    Ok(warp::reply::json(&all_metrics.market_data))
}

/// Get risk metrics
async fn get_risk_metrics(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let all_metrics = metrics.get_all_metrics();
    Ok(warp::reply::json(&all_metrics.risk))
}

/// Get timeseries data for Grafana's JSON datasource
async fn get_timeseries_data(
    metric_type: String,
    _query: TimeseriesQuery,
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    // Convert current metrics to timeseries format expected by Grafana
    let all_metrics = metrics.get_all_metrics();
    
    let timeseries_data = match metric_type.as_str() {
        "portfolio_pnl" => {
            vec![json!({
                "target": "Total P&L",
                "datapoints": [
                    [all_metrics.portfolio.total_pnl, all_metrics.portfolio.timestamp.timestamp_millis()]
                ]
            })]
        },
        "portfolio_capital" => {
            vec![json!({
                "target": "Total Capital",
                "datapoints": [
                    [all_metrics.portfolio.total_capital, all_metrics.portfolio.timestamp.timestamp_millis()]
                ]
            })]
        },
        "signals_per_minute" => {
            vec![json!({
                "target": "Signals/Min",
                "datapoints": [
                    [all_metrics.signals.signals_per_minute, all_metrics.signals.timestamp.timestamp_millis()]
                ]
            })]
        },
        "signal_confidence" => {
            vec![json!({
                "target": "Avg Confidence",
                "datapoints": [
                    [all_metrics.signals.avg_confidence * 100.0, all_metrics.signals.timestamp.timestamp_millis()]
                ]
            })]
        },
        _ => {
            return Err(warp::reject::custom(ApiError {
                message: format!("Unknown metric type: {}", metric_type),
            }));
        }
    };

    Ok(warp::reply::json(&timeseries_data))
}

/// Get simple metrics for Grafana Infinity datasource
async fn get_simple_metrics(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let all_metrics = metrics.get_all_metrics();
    
    // Generate demo stock data if no real market data exists
    let demo_stocks = if all_metrics.market_data.is_empty() {
        vec![
            ("AAPL", 175.50, 2.3, 15000000.0, 1.2),
            ("MSFT", 342.80, -0.8, 12000000.0, 0.9),
            ("GOOGL", 138.45, 1.7, 8500000.0, 1.5),
            ("TSLA", 242.80, 4.2, 25000000.0, 3.1),
            ("NVDA", 465.20, 3.8, 18000000.0, 2.7),
            ("META", 298.75, -1.2, 14000000.0, 1.8),
            ("AMZN", 127.35, 0.5, 22000000.0, 1.1),
            ("NFLX", 445.60, -2.1, 6500000.0, 2.3),
        ]
    } else {
        Vec::new()
    };

    let stocks_data: Vec<_> = if !demo_stocks.is_empty() {
        demo_stocks.iter().map(|(symbol, price, change, volume, volatility)| {
            json!({
                "symbol": symbol,
                "price": price,
                "change_24h": change,
                "volume": volume,
                "volatility": volatility,
                "trend": if *change > 0.0 { "up" } else if *change < 0.0 { "down" } else { "flat" }
            })
        }).collect()
    } else {
        all_metrics.market_data.iter().map(|stock| {
            json!({
                "symbol": stock.symbol,
                "price": stock.price,
                "change_24h": stock.price_change_pct_24h,
                "volume": stock.volume_24h,
                "volatility": stock.volatility,
                "trend": if stock.price_change_pct_24h > 0.0 { "up" } else if stock.price_change_pct_24h < 0.0 { "down" } else { "flat" }
            })
        }).collect()
    };

    // Return a simplified metrics structure for Grafana
    let simple_metrics = json!({
        "timestamp": chrono::Utc::now(),
        "total_capital": all_metrics.portfolio.total_capital,
        "total_pnl": all_metrics.portfolio.total_pnl,
        "portfolio_value": all_metrics.portfolio.total_capital + all_metrics.portfolio.total_pnl,
        "open_positions": all_metrics.positions.len(),
        "total_return_percent": all_metrics.portfolio.total_return_pct,
        "trades_executed": all_metrics.signals.signals_processed,
        "opportunities_today": all_metrics.signals.signals_per_minute * 60.0 * 8.0, // Rough estimate for 8-hour trading day
        "signals_per_minute": all_metrics.signals.signals_per_minute,
        "avg_confidence": all_metrics.signals.avg_confidence,
        "market_volatility": if all_metrics.market_data.is_empty() { 0.0 } else { all_metrics.market_data[0].volatility },
        "win_rate": all_metrics.portfolio.win_rate,
        "sharpe_ratio": all_metrics.portfolio.sharpe_ratio,
        "max_drawdown": all_metrics.portfolio.max_drawdown,
        "var_95": all_metrics.risk.portfolio_var_95,
        "portfolio_heat": all_metrics.risk.concentration_risk,
        "stocks": stocks_data,
        "monitored_stocks_count": stocks_data.len(),
        "price_history": generate_mock_price_history()
    });
    
    Ok(warp::reply::json(&simple_metrics))
}

/// Generate mock price history for dashboard demo
fn generate_mock_price_history() -> serde_json::Value {
    let current_time = chrono::Utc::now();
    let mut history = Vec::new();
    
    // Generate 24 hours of data points (every hour)
    for i in 0..24 {
        let timestamp = current_time - chrono::Duration::hours(24 - i);
        let base_price = 175.50; // AAPL base price
        let price_variation = (i as f64 * 0.2).sin() * 3.0 + ((i % 5) as f64 - 2.0) * 1.5;
        let price = base_price + price_variation;
        
        history.push(json!({
            "timestamp": timestamp.timestamp_millis(),
            "price": price.max(1.0),
        }));
    }
    
    json!(history)
}

/// Get trading opportunities for Grafana tables
async fn get_opportunities(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    // Generate mock opportunities for demonstration
    // In a real implementation, this would come from the trading system
    let opportunities = json!({
        "opportunities": [
            {
                "symbol": "AAPL",
                "strategy": "Neuromorphic Momentum",
                "confidence": 0.85,
                "expected_move": 2.5,
                "time_horizon": "4h",
                "entry_price": 175.50,
                "position_size": 0.02,
                "risk_score": 0.3
            },
            {
                "symbol": "TSLA", 
                "strategy": "Volume Spike",
                "confidence": 0.78,
                "expected_move": 4.2,
                "time_horizon": "2h",
                "entry_price": 242.80,
                "position_size": 0.015,
                "risk_score": 0.4
            },
            {
                "symbol": "NVDA",
                "strategy": "Breakout Pattern",
                "confidence": 0.73,
                "expected_move": 3.1,
                "time_horizon": "6h", 
                "entry_price": 118.45,
                "position_size": 0.025,
                "risk_score": 0.35
            }
        ]
    });
    
    Ok(warp::reply::json(&opportunities))
}

/// Get monitored stocks and their current data
async fn get_monitored_stocks(
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let all_metrics = metrics.get_all_metrics();
    
    // Transform market data into a more dashboard-friendly format
    let mut stocks: Vec<_> = all_metrics.market_data.iter().map(|stock| {
        json!({
            "symbol": stock.symbol,
            "price": stock.price,
            "change_24h": stock.price_change_pct_24h,
            "volume": stock.volume_24h,
            "volatility": stock.volatility,
            "last_updated": stock.last_update,
            "trend": if stock.price_change_pct_24h > 0.0 { "up" } else if stock.price_change_pct_24h < 0.0 { "down" } else { "flat" }
        })
    }).collect();
    
    // If no real market data, provide demo data for dashboard testing
    if stocks.is_empty() {
        let demo_stocks = vec![
            ("AAPL", 175.50, 2.3, 15000000.0, 1.2),
            ("MSFT", 342.80, -0.8, 12000000.0, 0.9),
            ("GOOGL", 138.45, 1.7, 8500000.0, 1.5),
            ("TSLA", 242.80, 4.2, 25000000.0, 3.1),
            ("NVDA", 465.20, 3.8, 18000000.0, 2.7),
            ("META", 298.75, -1.2, 14000000.0, 1.8),
            ("AMZN", 127.35, 0.5, 22000000.0, 1.1),
            ("NFLX", 445.60, -2.1, 6500000.0, 2.3),
        ];
        
        stocks = demo_stocks.iter().map(|(symbol, price, change, volume, volatility)| {
            json!({
                "symbol": symbol,
                "price": price,
                "change_24h": change,
                "volume": volume,
                "volatility": volatility,
                "last_updated": chrono::Utc::now(),
                "trend": if *change > 0.0 { "up" } else if *change < 0.0 { "down" } else { "flat" }
            })
        }).collect();
    }
    
    let response = json!({
        "stocks": stocks,
        "total_monitored": stocks.len(),
        "last_updated": chrono::Utc::now()
    });
    
    Ok(warp::reply::json(&response))
}

/// Get price history for a specific stock
async fn get_stock_history(
    symbol: String,
    query: HistoryQuery,
    metrics: Arc<MetricsCollector>,
) -> Result<impl Reply, Rejection> {
    let hours = query.hours.unwrap_or(24);
    
    // For demo purposes, generate some realistic price history
    // In a real implementation, this would come from stored historical data
    let current_time = chrono::Utc::now();
    let mut price_points = Vec::new();
    
    // Generate mock historical data based on current market data
    let all_metrics = metrics.get_all_metrics();
    let current_stock = all_metrics.market_data.iter()
        .find(|stock| stock.symbol == symbol);
    
    let base_price = current_stock.map(|s| s.price).unwrap_or(100.0);
    
    for i in 0..hours {
        let timestamp = current_time - chrono::Duration::hours(hours as i64 - i as i64);
        let price_variation = (i as f64 * 0.1).sin() * 2.0 + ((i % 7) as f64 - 3.0) * 1.5;
        let price = base_price + price_variation;
        
        price_points.push(json!({
            "timestamp": timestamp.timestamp_millis(),
            "price": price.max(1.0), // Ensure price doesn't go negative
            "volume": 1000000.0 + ((i % 10) as f64 * 50000.0)
        }));
    }
    
    let response = json!({
        "symbol": symbol,
        "timeframe_hours": hours,
        "data_points": price_points.len(),
        "price_history": price_points
    });
    
    Ok(warp::reply::json(&response))
}

/// Handle API errors
async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = warp::http::StatusCode::NOT_FOUND;
        message = "Endpoint not found";
    } else if let Some(api_error) = err.find::<ApiError>() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = &api_error.message;
    } else {
        tracing::error!("Unhandled rejection: {:?}", err);
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal server error";
    }

    let json = warp::reply::json(&json!({
        "error": message,
        "code": code.as_u16()
    }));

    Ok(warp::reply::with_status(json, code))
}