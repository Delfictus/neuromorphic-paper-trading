//! Order management for paper trading

use crate::exchanges::{Symbol, Exchange, Side};
use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::sync::mpsc;

/// Order type
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    TakeProfit,
    StopLimit,
}

/// Order status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,
    Submitted,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

/// Time in force
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TimeInForce {
    GTC,  // Good Till Cancelled
    IOC,  // Immediate or Cancel
    FOK,  // Fill or Kill
    GTD(u64),  // Good Till Date (timestamp)
}

/// Order structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub symbol: Symbol,
    pub exchange: Exchange,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub price: Option<f64>,
    pub stop_price: Option<f64>,
    pub avg_fill_price: f64,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub created_time: u64,
    pub updated_time: u64,
    pub filled_time: Option<u64>,
    pub commission: f64,
    pub slippage: f64,
    pub position_id: Option<String>,
    pub parent_order_id: Option<String>,
    pub child_order_ids: Vec<String>,
}

impl Order {
    pub fn market(symbol: Symbol, exchange: Exchange, side: Side, quantity: f64) -> Self {
        Self::new(symbol, exchange, side, OrderType::Market, quantity, None, None)
    }
    
    pub fn limit(
        symbol: Symbol,
        exchange: Exchange,
        side: Side,
        quantity: f64,
        price: f64,
    ) -> Self {
        Self::new(symbol, exchange, side, OrderType::Limit, quantity, Some(price), None)
    }
    
    pub fn stop_loss(
        symbol: Symbol,
        exchange: Exchange,
        side: Side,
        quantity: f64,
        stop_price: f64,
    ) -> Self {
        Self::new(symbol, exchange, side, OrderType::StopLoss, quantity, None, Some(stop_price))
    }
    
    fn new(
        symbol: Symbol,
        exchange: Exchange,
        side: Side,
        order_type: OrderType,
        quantity: f64,
        price: Option<f64>,
        stop_price: Option<f64>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            id: format!("ORD_{}_{}", now, nanoid::nanoid!(8)),
            symbol,
            exchange,
            side,
            order_type,
            quantity,
            filled_quantity: 0.0,
            price,
            stop_price,
            avg_fill_price: 0.0,
            status: OrderStatus::Pending,
            time_in_force: TimeInForce::GTC,
            created_time: now,
            updated_time: now,
            filled_time: None,
            commission: 0.0,
            slippage: 0.0,
            position_id: None,
            parent_order_id: None,
            child_order_ids: Vec::new(),
        }
    }
    
    /// Check if order should trigger based on current price
    pub fn should_trigger(&self, current_price: f64) -> bool {
        match self.order_type {
            OrderType::Market => true,
            OrderType::Limit => {
                if let Some(limit_price) = self.price {
                    match self.side {
                        Side::Buy => current_price <= limit_price,
                        Side::Sell => current_price >= limit_price,
                    }
                } else {
                    false
                }
            }
            OrderType::StopLoss | OrderType::StopLimit => {
                if let Some(stop) = self.stop_price {
                    match self.side {
                        Side::Buy => current_price >= stop,
                        Side::Sell => current_price <= stop,
                    }
                } else {
                    false
                }
            }
            OrderType::TakeProfit => {
                if let Some(target) = self.price {
                    match self.side {
                        Side::Buy => current_price <= target,
                        Side::Sell => current_price >= target,
                    }
                } else {
                    false
                }
            }
        }
    }
    
    /// Check if order has expired
    pub fn is_expired(&self) -> bool {
        match self.time_in_force {
            TimeInForce::GTD(expiry) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                now > expiry
            }
            _ => false,
        }
    }
    
    /// Fill order partially or completely
    pub fn fill(&mut self, fill_quantity: f64, fill_price: f64, commission: f64, slippage: f64) {
        let prev_filled = self.filled_quantity;
        self.filled_quantity = (self.filled_quantity + fill_quantity).min(self.quantity);
        let actual_fill = self.filled_quantity - prev_filled;
        
        // Update average fill price
        if prev_filled > 0.0 {
            self.avg_fill_price = (self.avg_fill_price * prev_filled + fill_price * actual_fill) 
                / self.filled_quantity;
        } else {
            self.avg_fill_price = fill_price;
        }
        
        self.commission += commission;
        self.slippage += slippage;
        self.updated_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        if self.filled_quantity >= self.quantity {
            self.status = OrderStatus::Filled;
            self.filled_time = Some(self.updated_time);
        } else {
            self.status = OrderStatus::PartiallyFilled;
        }
    }
    
    /// Cancel order
    pub fn cancel(&mut self) {
        self.status = OrderStatus::Cancelled;
        self.updated_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }
    
    /// Reject order
    pub fn reject(&mut self, _reason: &str) {
        self.status = OrderStatus::Rejected;
        self.updated_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }
}

/// Order execution event
#[derive(Clone, Debug)]
pub enum OrderEvent {
    Submitted(Order),
    Filled { order_id: String, fill_price: f64, fill_quantity: f64 },
    PartiallyFilled { order_id: String, fill_price: f64, fill_quantity: f64 },
    Cancelled(String),
    Rejected { order_id: String, reason: String },
    Expired(String),
}

/// Order manager
pub struct OrderManager {
    orders: DashMap<String, Order>,
    pending_orders: DashMap<String, Order>,
    active_orders: DashMap<String, Order>,
    filled_orders: DashMap<String, Order>,
    orders_by_symbol: DashMap<Symbol, Vec<String>>,
    order_counter: AtomicU64,
    event_sender: mpsc::UnboundedSender<OrderEvent>,
    event_receiver: Option<mpsc::UnboundedReceiver<OrderEvent>>,
    commission_rate: f64,
    slippage_model: SlippageModel,
}

/// Slippage model for realistic execution
#[derive(Clone, Debug)]
pub enum SlippageModel {
    Fixed(f64),
    Percentage(f64),
    Dynamic { base: f64, impact: f64 },
}

impl OrderManager {
    pub fn new(commission_rate: f64, slippage_model: SlippageModel) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            orders: DashMap::new(),
            pending_orders: DashMap::new(),
            active_orders: DashMap::new(),
            filled_orders: DashMap::new(),
            orders_by_symbol: DashMap::new(),
            order_counter: AtomicU64::new(0),
            event_sender: tx,
            event_receiver: Some(rx),
            commission_rate,
            slippage_model,
        }
    }
    
    /// Submit a new order
    pub fn submit_order(&self, mut order: Order) -> Result<String> {
        let order_id = order.id.clone();
        order.status = OrderStatus::Submitted;
        
        // Store order
        self.orders.insert(order_id.clone(), order.clone());
        self.active_orders.insert(order_id.clone(), order.clone());
        
        // Track by symbol
        self.orders_by_symbol
            .entry(order.symbol.clone())
            .or_insert_with(Vec::new)
            .push(order_id.clone());
        
        // Send event
        self.event_sender.send(OrderEvent::Submitted(order))?;
        
        self.order_counter.fetch_add(1, Ordering::Relaxed);
        
        Ok(order_id)
    }
    
    /// Cancel an order
    pub fn cancel_order(&self, order_id: &str) -> Result<()> {
        if let Some(mut order) = self.active_orders.get_mut(order_id) {
            order.cancel();
            
            // Move to appropriate collection
            let cancelled_order = order.clone();
            drop(order); // Release lock
            
            self.active_orders.remove(order_id);
            self.orders.insert(order_id.to_string(), cancelled_order);
            
            // Send event
            self.event_sender.send(OrderEvent::Cancelled(order_id.to_string()))?;
        }
        
        Ok(())
    }
    
    /// Process orders based on current market prices
    pub fn process_orders(&self, prices: &DashMap<Symbol, f64>) -> Result<Vec<String>> {
        let mut filled_orders = Vec::new();
        
        for entry in self.active_orders.iter() {
            let mut order = entry.value().clone();
            
            if let Some(price) = prices.get(&order.symbol) {
                // Check if order should trigger
                if order.should_trigger(*price) {
                    // Calculate execution details
                    let (exec_price, slippage) = self.calculate_execution_price(
                        *price,
                        &order.side,
                        order.quantity
                    );
                    
                    let commission = self.calculate_commission(order.quantity, exec_price);
                    
                    // Fill the order
                    order.fill(order.quantity, exec_price, commission, slippage);
                    
                    // Update collections
                    self.active_orders.remove(&order.id);
                    self.filled_orders.insert(order.id.clone(), order.clone());
                    self.orders.insert(order.id.clone(), order.clone());
                    
                    // Send event
                    let event = if order.status == OrderStatus::Filled {
                        OrderEvent::Filled {
                            order_id: order.id.clone(),
                            fill_price: exec_price,
                            fill_quantity: order.quantity,
                        }
                    } else {
                        OrderEvent::PartiallyFilled {
                            order_id: order.id.clone(),
                            fill_price: exec_price,
                            fill_quantity: order.filled_quantity,
                        }
                    };
                    
                    self.event_sender.send(event)?;
                    filled_orders.push(order.id.clone());
                }
                
                // Check expiration
                if order.is_expired() {
                    order.status = OrderStatus::Expired;
                    
                    self.active_orders.remove(&order.id);
                    self.orders.insert(order.id.clone(), order);
                    
                    self.event_sender.send(OrderEvent::Expired(order.id.clone()))?;
                }
            }
        }
        
        Ok(filled_orders)
    }
    
    /// Calculate execution price with slippage
    fn calculate_execution_price(&self, market_price: f64, side: &Side, quantity: f64) -> (f64, f64) {
        let slippage = match &self.slippage_model {
            SlippageModel::Fixed(amount) => *amount,
            SlippageModel::Percentage(pct) => market_price * pct / 100.0,
            SlippageModel::Dynamic { base, impact } => {
                base + (impact * quantity.sqrt())
            }
        };
        
        let exec_price = match side {
            Side::Buy => market_price + slippage,
            Side::Sell => market_price - slippage,
        };
        
        (exec_price, slippage)
    }
    
    /// Calculate commission
    fn calculate_commission(&self, quantity: f64, price: f64) -> f64 {
        quantity * price * self.commission_rate / 100.0
    }
    
    /// Create bracket order (entry + stop loss + take profit)
    pub fn create_bracket_order(
        &self,
        symbol: Symbol,
        exchange: Exchange,
        side: Side,
        quantity: f64,
        entry_price: Option<f64>,
        stop_loss: f64,
        take_profit: f64,
    ) -> Result<(String, String, String)> {
        // Create main order
        let mut main_order = if let Some(price) = entry_price {
            Order::limit(symbol.clone(), exchange, side, quantity, price)
        } else {
            Order::market(symbol.clone(), exchange, side, quantity)
        };
        
        let main_id = self.submit_order(main_order.clone())?;
        
        // Create stop loss (opposite side)
        let stop_side = match side {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        };
        
        let mut stop_order = Order::stop_loss(
            symbol.clone(),
            exchange,
            stop_side,
            quantity,
            stop_loss
        );
        stop_order.parent_order_id = Some(main_id.clone());
        let stop_id = self.submit_order(stop_order)?;
        
        // Create take profit
        let mut tp_order = Order::new(
            symbol,
            exchange,
            stop_side,
            OrderType::TakeProfit,
            quantity,
            Some(take_profit),
            None
        );
        tp_order.parent_order_id = Some(main_id.clone());
        let tp_id = self.submit_order(tp_order)?;
        
        // Link orders
        if let Some(mut main) = self.orders.get_mut(&main_id) {
            main.child_order_ids.push(stop_id.clone());
            main.child_order_ids.push(tp_id.clone());
        }
        
        Ok((main_id, stop_id, tp_id))
    }
    
    /// Get order by ID
    pub fn get_order(&self, order_id: &str) -> Option<Order> {
        self.orders.get(order_id).map(|o| o.clone())
    }
    
    /// Get active orders
    pub fn get_active_orders(&self) -> Vec<Order> {
        self.active_orders
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Get orders by symbol
    pub fn get_orders_by_symbol(&self, symbol: &Symbol) -> Vec<Order> {
        self.orders_by_symbol
            .get(symbol)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.orders.get(id).map(|o| o.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Subscribe to order events
    pub fn subscribe(&mut self) -> Option<mpsc::UnboundedReceiver<OrderEvent>> {
        self.event_receiver.take()
    }
    
    /// Get order statistics
    pub fn get_statistics(&self) -> OrderStatistics {
        let mut stats = OrderStatistics::default();
        
        stats.total_orders = self.order_counter.load(Ordering::Relaxed);
        stats.pending_orders = self.pending_orders.len() as u64;
        stats.active_orders = self.active_orders.len() as u64;
        stats.filled_orders = self.filled_orders.len() as u64;
        
        // Calculate fill rate
        if stats.total_orders > 0 {
            stats.fill_rate = (stats.filled_orders as f64 / stats.total_orders as f64) * 100.0;
        }
        
        // Calculate average fill time
        let fill_times: Vec<u64> = self.filled_orders
            .iter()
            .filter_map(|e| {
                let order = e.value();
                order.filled_time.map(|ft| ft - order.created_time)
            })
            .collect();
        
        if !fill_times.is_empty() {
            stats.avg_fill_time_ms = fill_times.iter().sum::<u64>() as f64 / fill_times.len() as f64;
        }
        
        stats
    }
}

/// Order statistics
#[derive(Default, Clone, Debug)]
pub struct OrderStatistics {
    pub total_orders: u64,
    pub pending_orders: u64,
    pub active_orders: u64,
    pub filled_orders: u64,
    pub cancelled_orders: u64,
    pub rejected_orders: u64,
    pub fill_rate: f64,
    pub avg_fill_time_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_order_lifecycle() {
        let manager = OrderManager::new(0.1, SlippageModel::Fixed(0.01));
        
        // Create and submit order
        let order = Order::limit(
            Symbol::new("BTC-USD"),
            Exchange::Binance,
            Side::Buy,
            1.0,
            50000.0
        );
        
        let order_id = manager.submit_order(order).unwrap();
        
        // Process with price that triggers order
        let prices = DashMap::new();
        prices.insert(Symbol::new("BTC-USD"), 49999.0);
        
        let filled = manager.process_orders(&prices).unwrap();
        assert_eq!(filled.len(), 1);
        
        // Check order is filled
        let order = manager.get_order(&order_id).unwrap();
        assert_eq!(order.status, OrderStatus::Filled);
    }
    
    #[test]
    fn test_bracket_order() {
        let manager = OrderManager::new(0.1, SlippageModel::Percentage(0.01));
        
        let (main_id, stop_id, tp_id) = manager.create_bracket_order(
            Symbol::new("ETH-USD"),
            Exchange::Coinbase,
            Side::Buy,
            10.0,
            Some(3000.0),
            2900.0,
            3100.0
        ).unwrap();
        
        assert!(manager.get_order(&main_id).is_some());
        assert!(manager.get_order(&stop_id).is_some());
        assert!(manager.get_order(&tp_id).is_some());
    }
}