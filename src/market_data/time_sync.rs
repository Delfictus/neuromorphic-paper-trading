//! Time synchronization across exchanges

use crate::exchanges::Exchange;
use crate::time_source::HardwareClock;
use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Drift warning levels
#[derive(Clone, Debug)]
pub enum DriftWarning {
    Minor { offset_ms: f64 },
    Major { offset_ms: f64 },
    Critical { offset_ms: f64 },
}

/// Drift detector
pub struct DriftDetector {
    recent_offsets: RwLock<VecDeque<i64>>,
    max_samples: usize,
    warning_threshold_ms: f64,
    critical_threshold_ms: f64,
}

impl DriftDetector {
    pub fn new() -> Self {
        Self {
            recent_offsets: RwLock::new(VecDeque::with_capacity(100)),
            max_samples: 100,
            warning_threshold_ms: 5.0,
            critical_threshold_ms: 10.0,
        }
    }
    
    pub fn add_offset(&self, offset_us: i64) {
        let mut offsets = self.recent_offsets.write();
        if offsets.len() >= self.max_samples {
            offsets.pop_front();
        }
        offsets.push_back(offset_us);
    }
    
    pub fn detect_drift(&self) -> Option<DriftWarning> {
        let offsets = self.recent_offsets.read();
        if offsets.len() < 10 {
            return None;
        }
        
        // Calculate variance
        let mean: f64 = offsets.iter().map(|&x| x as f64).sum::<f64>() / offsets.len() as f64;
        let variance: f64 = offsets.iter()
            .map(|&x| {
                let diff = x as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / offsets.len() as f64;
        
        let std_dev = variance.sqrt();
        let drift_ms = std_dev / 1000.0;
        
        if drift_ms > self.critical_threshold_ms {
            Some(DriftWarning::Critical { offset_ms: drift_ms })
        } else if drift_ms > self.warning_threshold_ms {
            Some(DriftWarning::Major { offset_ms: drift_ms })
        } else if drift_ms > 1.0 {
            Some(DriftWarning::Minor { offset_ms: drift_ms })
        } else {
            None
        }
    }
}

/// Time synchronizer for multiple exchanges
pub struct TimeSynchronizer {
    exchange_offsets: DashMap<Exchange, i64>, // microseconds
    local_clock: Arc<HardwareClock>,
    drift_detector: Arc<DriftDetector>,
    last_calibration: DashMap<Exchange, Instant>,
}

impl TimeSynchronizer {
    pub fn new() -> Self {
        Self {
            exchange_offsets: DashMap::new(),
            local_clock: Arc::new(HardwareClock::new()),
            drift_detector: Arc::new(DriftDetector::new()),
            last_calibration: DashMap::new(),
        }
    }
    
    /// Calibrate time offset for an exchange
    pub async fn calibrate(&self, exchange: Exchange) -> Result<i64> {
        let mut offsets = Vec::new();
        
        // Make 10 requests to get median offset
        for _ in 0..10 {
            let local_before = self.local_clock.now_ns();
            let exchange_time = self.fetch_exchange_time(exchange).await?;
            let local_after = self.local_clock.now_ns();
            
            // Assume symmetric network delay
            let round_trip = (local_after - local_before) as i64;
            let local_mid = local_before as i64 + round_trip / 2;
            
            // Calculate offset in microseconds
            let offset_us = (exchange_time as i64 * 1000) - (local_mid / 1000);
            offsets.push(offset_us);
            
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        
        // Use median to reduce impact of outliers
        offsets.sort_unstable();
        let median_offset = offsets[offsets.len() / 2];
        
        // Store offset
        self.exchange_offsets.insert(exchange, median_offset);
        self.last_calibration.insert(exchange, Instant::now());
        
        // Update drift detector
        self.drift_detector.add_offset(median_offset);
        
        println!("Calibrated {} offset: {}μs", exchange, median_offset);
        
        Ok(median_offset)
    }
    
    /// Fetch current time from exchange
    async fn fetch_exchange_time(&self, exchange: Exchange) -> Result<u64> {
        match exchange {
            Exchange::Binance => {
                let url = "https://api.binance.com/api/v3/time";
                let response = reqwest::get(url).await?;
                let data: serde_json::Value = response.json().await?;
                Ok(data["serverTime"].as_u64().unwrap_or(0))
            }
            Exchange::Coinbase => {
                let url = "https://api.coinbase.com/v2/time";
                let response = reqwest::get(url).await?;
                let data: serde_json::Value = response.json().await?;
                let epoch = data["data"]["epoch"].as_u64().unwrap_or(0);
                Ok(epoch * 1000) // Convert to milliseconds
            }
            Exchange::Kraken => {
                let url = "https://api.kraken.com/0/public/Time";
                let response = reqwest::get(url).await?;
                let data: serde_json::Value = response.json().await?;
                let unixtime = data["result"]["unixtime"].as_u64().unwrap_or(0);
                Ok(unixtime * 1000) // Convert to milliseconds
            }
            _ => {
                // Default to system time for unsupported exchanges
                Ok(SystemTime::now()
                    .duration_since(UNIX_EPOCH)?
                    .as_millis() as u64)
            }
        }
    }
    
    /// Adjust exchange timestamp to local time
    pub fn adjust_timestamp(&self, exchange_time: u64, exchange: Exchange) -> u64 {
        let offset_us = self.exchange_offsets
            .get(&exchange)
            .map(|e| *e)
            .unwrap_or(0);
        
        // Convert to local time
        let local_time_us = (exchange_time as i64 * 1000) - offset_us;
        (local_time_us / 1000) as u64 // Convert back to milliseconds
    }
    
    /// Get local time
    pub fn get_local_time(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
    
    /// Detect time drift
    pub fn detect_drift(&self) -> Option<DriftWarning> {
        self.drift_detector.detect_drift()
    }
    
    /// Check if recalibration is needed
    pub fn needs_recalibration(&self, exchange: Exchange) -> bool {
        self.last_calibration
            .get(&exchange)
            .map(|last| last.elapsed() > Duration::from_secs(300)) // Recalibrate every 5 minutes
            .unwrap_or(true)
    }
    
    /// Get all offsets
    pub fn get_offsets(&self) -> Vec<(Exchange, i64)> {
        self.exchange_offsets
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect()
    }
}