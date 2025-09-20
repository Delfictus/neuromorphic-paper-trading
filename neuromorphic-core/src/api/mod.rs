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
            .with(cors)
            .recover(handle_rejection);

        tracing::info!("Starting Metrics API server on port {}", self.port);
        warp::serve(routes)
            .run(([127, 0, 0, 1], self.port))
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