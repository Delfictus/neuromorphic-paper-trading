//! Position management for paper trading

use crate::exchanges::{Symbol, Exchange, Side};
use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Position status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PositionStatus {
    Open,
    Closed,
    Partially Closed,
}

/// Individual position
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub side: Side,
    pub quantity: f64,
    pub entry_price: f64,
    pub entry_time: u64,
    pub exit_price: Option<f64>,
    pub exit_time: Option<u64>,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub status: PositionStatus,
    pub commission: f64,
    pub slippage: f64,
}

impl Position {
    pub fn new(
        symbol: Symbol,
        exchange: Exchange,
        side: Side,
        quantity: f64,
        entry_price: f64,
    ) -> Self {
        let id = format!("POS_{}_{}", 
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            nanoid::nanoid!(8)
        );
        
        Self {
            id,
            symbol,
            exchange,
            side,
            quantity,
            entry_price,
            entry_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            exit_price: None,
            exit_time: None,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            status: PositionStatus::Open,
            commission: 0.0,
            slippage: 0.0,
        }
    }
    
    /// Update unrealized P&L based on current price
    pub fn update_unrealized_pnl(&mut self, current_price: f64) {
        if self.status != PositionStatus::Open {
            return;
        }
        
        let price_diff = match self.side {
            Side::Buy => current_price - self.entry_price,
            Side::Sell => self.entry_price - current_price,
        };
        
        self.unrealized_pnl = price_diff * self.quantity - self.commission - self.slippage;
    }
    
    /// Close position at given price
    pub fn close(&mut self, exit_price: f64, commission: f64, slippage: f64) {
        self.exit_price = Some(exit_price);
        self.exit_time = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        );
        
        let price_diff = match self.side {
            Side::Buy => exit_price - self.entry_price,
            Side::Sell => self.entry_price - exit_price,
        };
        
        self.realized_pnl = price_diff * self.quantity - self.commission - commission - self.slippage - slippage;
        self.unrealized_pnl = 0.0;
        self.status = PositionStatus::Closed;
        self.commission += commission;
        self.slippage += slippage;
    }
    
    /// Partially close position
    pub fn partial_close(&mut self, quantity: f64, exit_price: f64, commission: f64, slippage: f64) -> f64 {
        if quantity >= self.quantity {
            self.close(exit_price, commission, slippage);
            return self.realized_pnl;
        }
        
        let close_ratio = quantity / self.quantity;
        
        let price_diff = match self.side {
            Side::Buy => exit_price - self.entry_price,
            Side::Sell => self.entry_price - exit_price,
        };
        
        let partial_pnl = price_diff * quantity - commission - slippage;
        self.realized_pnl += partial_pnl;
        self.quantity -= quantity;
        self.commission += commission;
        self.slippage += slippage;
        self.status = PositionStatus::PartialClosed;
        
        partial_pnl
    }
    
    /// Get total P&L (realized + unrealized)
    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl
    }
    
    /// Get position value at current price
    pub fn current_value(&self, current_price: f64) -> f64 {
        self.quantity * current_price
    }
    
    /// Calculate return on investment
    pub fn roi(&self) -> f64 {
        let initial_value = self.quantity * self.entry_price;
        if initial_value == 0.0 {
            return 0.0;
        }
        self.total_pnl() / initial_value * 100.0
    }
}

/// Position tracking statistics
#[derive(Default, Clone, Debug)]
pub struct PositionStatistics {
    pub total_positions: u64,
    pub open_positions: u64,
    pub winning_positions: u64,
    pub losing_positions: u64,
    pub total_realized_pnl: f64,
    pub total_unrealized_pnl: f64,
    pub total_commission: f64,
    pub total_slippage: f64,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
}

/// Position manager for paper trading
pub struct PositionManager {
    positions: DashMap<String, Position>,
    positions_by_symbol: DashMap<Symbol, Vec<String>>,
    open_positions: DashMap<String, Position>,
    closed_positions: DashMap<String, Position>,
    position_counter: AtomicU64,
    total_realized_pnl: AtomicI64, // Store as cents to avoid float atomics
    total_unrealized_pnl: AtomicI64,
    total_commission: AtomicI64,
    total_slippage: AtomicI64,
}

impl PositionManager {
    pub fn new() -> Self {
        Self {
            positions: DashMap::new(),
            positions_by_symbol: DashMap::new(),
            open_positions: DashMap::new(),
            closed_positions: DashMap::new(),
            position_counter: AtomicU64::new(0),
            total_realized_pnl: AtomicI64::new(0),
            total_unrealized_pnl: AtomicI64::new(0),
            total_commission: AtomicI64::new(0),
            total_slippage: AtomicI64::new(0),
        }
    }
    
    /// Open a new position
    pub fn open_position(
        &self,
        symbol: Symbol,
        exchange: Exchange,
        side: Side,
        quantity: f64,
        entry_price: f64,
        commission: f64,
        slippage: f64,
    ) -> Result<String> {
        let mut position = Position::new(symbol.clone(), exchange, side, quantity, entry_price);
        position.commission = commission;
        position.slippage = slippage;
        
        let position_id = position.id.clone();
        
        // Update tracking
        self.positions.insert(position_id.clone(), position.clone());
        self.open_positions.insert(position_id.clone(), position.clone());
        
        // Track by symbol
        self.positions_by_symbol
            .entry(symbol)
            .or_insert_with(Vec::new)
            .push(position_id.clone());
        
        // Update counters
        self.position_counter.fetch_add(1, Ordering::Relaxed);
        self.total_commission.fetch_add((commission * 100.0) as i64, Ordering::Relaxed);
        self.total_slippage.fetch_add((slippage * 100.0) as i64, Ordering::Relaxed);
        
        Ok(position_id)
    }
    
    /// Close a position
    pub fn close_position(
        &self,
        position_id: &str,
        exit_price: f64,
        commission: f64,
        slippage: f64,
    ) -> Result<f64> {
        let mut position = self.open_positions
            .remove(position_id)
            .ok_or_else(|| anyhow::anyhow!("Position {} not found or already closed", position_id))?
            .1;
        
        position.close(exit_price, commission, slippage);
        let pnl = position.realized_pnl;
        
        // Move to closed positions
        self.closed_positions.insert(position_id.to_string(), position.clone());
        self.positions.insert(position_id.to_string(), position);
        
        // Update totals
        self.total_realized_pnl.fetch_add((pnl * 100.0) as i64, Ordering::Relaxed);
        self.total_commission.fetch_add((commission * 100.0) as i64, Ordering::Relaxed);
        self.total_slippage.fetch_add((slippage * 100.0) as i64, Ordering::Relaxed);
        
        Ok(pnl)
    }
    
    /// Partially close a position
    pub fn partial_close_position(
        &self,
        position_id: &str,
        quantity: f64,
        exit_price: f64,
        commission: f64,
        slippage: f64,
    ) -> Result<f64> {
        let mut position = self.open_positions
            .get_mut(position_id)
            .ok_or_else(|| anyhow::anyhow!("Position {} not found", position_id))?;
        
        let pnl = position.partial_close(quantity, exit_price, commission, slippage);
        
        // If fully closed, move to closed positions
        if position.status == PositionStatus::Closed {
            let closed_position = position.clone();
            drop(position); // Release the lock
            
            self.open_positions.remove(position_id);
            self.closed_positions.insert(position_id.to_string(), closed_position);
        }
        
        // Update totals
        self.total_realized_pnl.fetch_add((pnl * 100.0) as i64, Ordering::Relaxed);
        self.total_commission.fetch_add((commission * 100.0) as i64, Ordering::Relaxed);
        self.total_slippage.fetch_add((slippage * 100.0) as i64, Ordering::Relaxed);
        
        Ok(pnl)
    }
    
    /// Update all open positions with current prices
    pub fn update_prices(&self, prices: &DashMap<Symbol, f64>) {
        let mut total_unrealized = 0i64;
        
        for mut entry in self.open_positions.iter_mut() {
            let position = entry.value_mut();
            
            if let Some(price) = prices.get(&position.symbol) {
                position.update_unrealized_pnl(*price);
                total_unrealized += (position.unrealized_pnl * 100.0) as i64;
            }
        }
        
        self.total_unrealized_pnl.store(total_unrealized, Ordering::Relaxed);
    }
    
    /// Get position by ID
    pub fn get_position(&self, position_id: &str) -> Option<Position> {
        self.positions.get(position_id).map(|p| p.clone())
    }
    
    /// Get all open positions
    pub fn get_open_positions(&self) -> Vec<Position> {
        self.open_positions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Get open positions for a symbol
    pub fn get_open_positions_by_symbol(&self, symbol: &Symbol) -> Vec<Position> {
        self.positions_by_symbol
            .get(symbol)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| {
                        self.open_positions.get(id).map(|p| p.clone())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get net position for a symbol
    pub fn get_net_position(&self, symbol: &Symbol) -> f64 {
        self.get_open_positions_by_symbol(symbol)
            .iter()
            .map(|p| match p.side {
                Side::Buy => p.quantity,
                Side::Sell => -p.quantity,
            })
            .sum()
    }
    
    /// Get position statistics
    pub fn get_statistics(&self) -> PositionStatistics {
        let mut stats = PositionStatistics::default();
        
        stats.total_positions = self.position_counter.load(Ordering::Relaxed);
        stats.open_positions = self.open_positions.len() as u64;
        
        let mut wins = Vec::new();
        let mut losses = Vec::new();
        
        for entry in self.closed_positions.iter() {
            let position = entry.value();
            if position.realized_pnl > 0.0 {
                stats.winning_positions += 1;
                wins.push(position.realized_pnl);
            } else if position.realized_pnl < 0.0 {
                stats.losing_positions += 1;
                losses.push(position.realized_pnl.abs());
            }
        }
        
        stats.total_realized_pnl = self.total_realized_pnl.load(Ordering::Relaxed) as f64 / 100.0;
        stats.total_unrealized_pnl = self.total_unrealized_pnl.load(Ordering::Relaxed) as f64 / 100.0;
        stats.total_commission = self.total_commission.load(Ordering::Relaxed) as f64 / 100.0;
        stats.total_slippage = self.total_slippage.load(Ordering::Relaxed) as f64 / 100.0;
        
        // Calculate win rate
        let total_closed = stats.winning_positions + stats.losing_positions;
        if total_closed > 0 {
            stats.win_rate = (stats.winning_positions as f64 / total_closed as f64) * 100.0;
        }
        
        // Calculate average win/loss
        if !wins.is_empty() {
            stats.avg_win = wins.iter().sum::<f64>() / wins.len() as f64;
        }
        if !losses.is_empty() {
            stats.avg_loss = losses.iter().sum::<f64>() / losses.len() as f64;
        }
        
        // Calculate profit factor
        if stats.avg_loss > 0.0 && stats.win_rate > 0.0 {
            let win_expectancy = stats.avg_win * (stats.win_rate / 100.0);
            let loss_expectancy = stats.avg_loss * (1.0 - stats.win_rate / 100.0);
            if loss_expectancy > 0.0 {
                stats.profit_factor = win_expectancy / loss_expectancy;
            }
        }
        
        // Simple Sharpe ratio calculation (would need returns history for accurate calculation)
        if total_closed > 0 {
            let returns: Vec<f64> = self.closed_positions
                .iter()
                .map(|e| e.value().roi())
                .collect();
            
            if returns.len() > 1 {
                let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
                let variance = returns.iter()
                    .map(|r| (r - mean_return).powi(2))
                    .sum::<f64>() / returns.len() as f64;
                let std_dev = variance.sqrt();
                
                if std_dev > 0.0 {
                    stats.sharpe_ratio = mean_return / std_dev;
                }
            }
        }
        
        stats
    }
    
    /// Reset all positions (for testing/reset)
    pub fn reset(&self) {
        self.positions.clear();
        self.positions_by_symbol.clear();
        self.open_positions.clear();
        self.closed_positions.clear();
        self.position_counter.store(0, Ordering::Relaxed);
        self.total_realized_pnl.store(0, Ordering::Relaxed);
        self.total_unrealized_pnl.store(0, Ordering::Relaxed);
        self.total_commission.store(0, Ordering::Relaxed);
        self.total_slippage.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_lifecycle() {
        let manager = PositionManager::new();
        
        // Open position
        let id = manager.open_position(
            Symbol::new("BTC-USD"),
            Exchange::Binance,
            Side::Buy,
            1.0,
            50000.0,
            10.0,
            5.0,
        ).unwrap();
        
        // Update price
        let prices = DashMap::new();
        prices.insert(Symbol::new("BTC-USD"), 51000.0);
        manager.update_prices(&prices);
        
        // Check unrealized P&L
        let position = manager.get_position(&id).unwrap();
        assert!(position.unrealized_pnl > 0.0);
        
        // Close position
        let pnl = manager.close_position(&id, 51000.0, 10.0, 5.0).unwrap();
        assert!(pnl > 0.0);
        
        // Check statistics
        let stats = manager.get_statistics();
        assert_eq!(stats.winning_positions, 1);
        assert_eq!(stats.win_rate, 100.0);
    }
}