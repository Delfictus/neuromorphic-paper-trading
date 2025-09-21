use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use reqwest::Client;
use crate::exchanges::{Symbol, Exchange};
use super::{MarketData, ScannerConfig};
use chrono::Utc;

pub struct DataFeedManager {
    config: ScannerConfig,
    feeds: HashMap<Exchange, Box<dyn MarketDataFeed>>,
    client: Client,
    symbol_universe: Arc<RwLock<Vec<Symbol>>>,
}

#[async_trait::async_trait]
pub trait MarketDataFeed: Send + Sync {
    async fn connect(&self) -> Result<()>;
    async fn subscribe_symbols(&self, symbols: Vec<Symbol>) -> Result<()>;
    async fn get_market_data(&self) -> Result<Vec<MarketData>>;
    async fn get_symbol_universe(&self) -> Result<Vec<Symbol>>;
    fn get_exchange(&self) -> Exchange;
}

#[derive(Debug, Clone)]
pub struct PolygonFeed {
    api_key: String,
    client: Client,
    exchange: Exchange,
    websocket_url: String,
    rest_url: String,
}

#[derive(Debug, Clone)]
pub struct AlphaVantageFeed {
    api_key: String,
    client: Client,
    exchange: Exchange,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct YahooFinanceFeed {
    client: Client,
    exchange: Exchange,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct PolygonTickerResponse {
    results: Vec<PolygonTicker>,
    status: String,
    count: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct PolygonTicker {
    #[serde(rename = "T")]
    ticker: String,
    #[serde(rename = "c")]
    close: Option<f64>,
    #[serde(rename = "h")]
    high: Option<f64>,
    #[serde(rename = "l")]
    low: Option<f64>,
    #[serde(rename = "o")]
    open: Option<f64>,
    #[serde(rename = "v")]
    volume: Option<f64>,
    #[serde(rename = "vw")]
    volume_weighted_price: Option<f64>,
    #[serde(rename = "t")]
    timestamp: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct YahooQuoteResponse {
    #[serde(rename = "quoteResponse")]
    quote_response: YahooQuoteData,
}

#[derive(Debug, Deserialize)]
struct YahooQuoteData {
    result: Vec<YahooQuote>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YahooQuote {
    symbol: String,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketVolume")]
    regular_market_volume: Option<f64>,
    #[serde(rename = "regularMarketOpen")]
    regular_market_open: Option<f64>,
    #[serde(rename = "regularMarketDayHigh")]
    regular_market_day_high: Option<f64>,
    #[serde(rename = "regularMarketDayLow")]
    regular_market_day_low: Option<f64>,
    #[serde(rename = "regularMarketChangePercent")]
    regular_market_change_percent: Option<f64>,
    #[serde(rename = "bid")]
    bid: Option<f64>,
    #[serde(rename = "ask")]
    ask: Option<f64>,
}

impl DataFeedManager {
    pub fn new(config: ScannerConfig) -> Self {
        let client = Client::new();
        let mut feeds = HashMap::new();
        
        // Add Yahoo Finance feed by default (no API key required)
        let yahoo_feed = YahooFinanceFeed::new(client.clone());
        feeds.insert(Exchange::NYSE, Box::new(yahoo_feed.clone()) as Box<dyn MarketDataFeed>);
        feeds.insert(Exchange::NASDAQ, Box::new(yahoo_feed) as Box<dyn MarketDataFeed>);
        
        let symbol_universe = Arc::new(RwLock::new(Vec::new()));

        Self {
            config,
            feeds,
            client,
            symbol_universe,
        }
    }

    pub async fn add_polygon_feed(&mut self, api_key: String) {
        let feed = PolygonFeed::new(api_key, self.client.clone());
        self.feeds.insert(Exchange::NYSE, Box::new(feed.clone()));
        self.feeds.insert(Exchange::NASDAQ, Box::new(feed));
    }

    pub async fn add_yahoo_feed(&mut self) {
        let feed = YahooFinanceFeed::new(self.client.clone());
        self.feeds.insert(Exchange::NYSE, Box::new(feed.clone()));
        self.feeds.insert(Exchange::NASDAQ, Box::new(feed));
    }

    pub async fn start_all_feeds(&self) -> Result<broadcast::Receiver<MarketData>> {
        let (tx, rx) = broadcast::channel(10000);
        
        println!("🚀 Starting data feeds for {} exchanges", self.config.included_exchanges.len());
        for exchange in &self.config.included_exchanges {
            println!("🔍 Processing exchange: {:?}", exchange);
            match exchange {
                Exchange::NYSE | Exchange::NASDAQ => {
                    let client = self.client.clone();
                    let tx = tx.clone();
                    let symbol_universe = self.symbol_universe.clone();
                    let exchange_clone = exchange.clone();
                    
                    tokio::spawn(async move {
                        let feed = YahooFinanceFeed::new(client);
                        let mut interval = interval(Duration::from_millis(30000)); // 30 seconds instead of 1 second
                        
                        println!("📡 Starting Yahoo Finance data feed for {:?}", exchange_clone);
                        
                        loop {
                            interval.tick().await;
                            
                            match feed.get_market_data().await {
                                Ok(market_data) => {
                                    println!("📊 Received {} market data points from Yahoo Finance", market_data.len());
                                    for data in market_data {
                                        let _ = tx.send(data);
                                    }
                                }
                                Err(e) => {
                                    println!("⚠️  Yahoo Finance API error: {}", e);
                                    // Wait longer on error to avoid hitting rate limits
                                    tokio::time::sleep(Duration::from_millis(60000)).await;
                                }
                            }
                            
                            match feed.get_symbol_universe().await {
                                Ok(universe) => {
                                    let mut symbols = symbol_universe.write().await;
                                    let initial_count = symbols.len();
                                    for symbol in universe {
                                        if !symbols.contains(&symbol) {
                                            symbols.push(symbol);
                                        }
                                    }
                                    if symbols.len() > initial_count {
                                        println!("📈 Symbol universe updated: {} symbols tracked", symbols.len());
                                    }
                                }
                                Err(e) => {
                                    println!("⚠️  Error getting symbol universe: {}", e);
                                }
                            }
                        }
                    });
                }
                _ => {}
            }
        }
        
        Ok(rx)
    }

    pub async fn get_symbol_universe(&self) -> Vec<Symbol> {
        self.symbol_universe.read().await.clone()
    }
}

impl PolygonFeed {
    pub fn new(api_key: String, client: Client) -> Self {
        Self {
            api_key,
            client,
            exchange: Exchange::NYSE,
            websocket_url: "wss://socket.polygon.io/stocks".to_string(),
            rest_url: "https://api.polygon.io".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl MarketDataFeed for PolygonFeed {
    async fn connect(&self) -> Result<()> {
        Ok(())
    }

    async fn subscribe_symbols(&self, _symbols: Vec<Symbol>) -> Result<()> {
        Ok(())
    }

    async fn get_market_data(&self) -> Result<Vec<MarketData>> {
        let url = format!(
            "{}/v2/aggs/grouped/locale/us/market/stocks/{}?adjusted=true&apikey={}",
            self.rest_url,
            chrono::Utc::now().format("%Y-%m-%d"),
            self.api_key
        );

        let response: PolygonTickerResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let mut market_data = Vec::new();
        for ticker in response.results {
            if let (Some(close), Some(volume), Some(open), Some(high), Some(low)) = 
                (ticker.close, ticker.volume, ticker.open, ticker.high, ticker.low) {
                
                let symbol = Symbol::new(ticker.ticker.clone());

                let change_24h = ((close - open) / open) * 100.0;

                market_data.push(MarketData {
                    symbol,
                    price: close,
                    volume,
                    timestamp: Utc::now(),
                    bid: None,
                    ask: None,
                    open,
                    high,
                    low,
                    change_24h,
                    volume_24h: volume,
                });
            }
        }

        Ok(market_data)
    }

    async fn get_symbol_universe(&self) -> Result<Vec<Symbol>> {
        let url = format!(
            "{}/v3/reference/tickers?market=stocks&active=true&limit=5000&apikey={}",
            self.rest_url, self.api_key
        );

        #[derive(Deserialize)]
        struct TickerListResponse {
            results: Vec<TickerInfo>,
        }

        #[derive(Deserialize)]
        struct TickerInfo {
            ticker: String,
            market: String,
            locale: String,
            active: bool,
        }

        let response: TickerListResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let symbols = response.results
            .into_iter()
            .filter(|t| t.active && t.locale == "us")
            .map(|t| Symbol::new(t.ticker))
            .collect();

        Ok(symbols)
    }

    fn get_exchange(&self) -> Exchange {
        self.exchange.clone()
    }
}

impl YahooFinanceFeed {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            exchange: Exchange::NYSE,
            base_url: "https://query1.finance.yahoo.com".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl MarketDataFeed for YahooFinanceFeed {
    async fn connect(&self) -> Result<()> {
        Ok(())
    }

    async fn subscribe_symbols(&self, _symbols: Vec<Symbol>) -> Result<()> {
        Ok(())
    }

    async fn get_market_data(&self) -> Result<Vec<MarketData>> {
        let popular_symbols = vec![
            "AAPL", "MSFT", "GOOGL", "AMZN", "TSLA", "META", "NVDA", "NFLX", 
            "ORCL", "CRM", "PYPL", "ADBE", "INTC", "AMD", "SHOP", "ZOOM",
            "SQ", "ROKU", "SPOT", "UBER", "LYFT", "TWTR", "SNAP", "PINS"
        ];

        let symbols_str = popular_symbols.join(",");
        let url = format!(
            "{}/v7/finance/quote?symbols={}",
            self.base_url, symbols_str
        );

        let response: YahooQuoteResponse = self.client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        let mut market_data = Vec::new();
        for quote in response.quote_response.result {
            if let (Some(price), Some(volume), Some(open), Some(high), Some(low)) = (
                quote.regular_market_price,
                quote.regular_market_volume,
                quote.regular_market_open,
                quote.regular_market_day_high,
                quote.regular_market_day_low,
            ) {
                let symbol = Symbol::new(quote.symbol.clone());

                let change_24h = quote.regular_market_change_percent.unwrap_or(0.0);

                market_data.push(MarketData {
                    symbol,
                    price,
                    volume,
                    timestamp: Utc::now(),
                    bid: quote.bid,
                    ask: quote.ask,
                    open,
                    high,
                    low,
                    change_24h,
                    volume_24h: volume,
                });
            }
        }

        Ok(market_data)
    }

    async fn get_symbol_universe(&self) -> Result<Vec<Symbol>> {
        let symbols = vec![
            "AAPL", "MSFT", "GOOGL", "AMZN", "TSLA", "META", "NVDA", "NFLX", 
            "ORCL", "CRM", "PYPL", "ADBE", "INTC", "AMD", "SHOP", "ZOOM",
            "SQ", "ROKU", "SPOT", "UBER", "LYFT", "TWTR", "SNAP", "PINS",
            "BA", "DIS", "JPM", "V", "MA", "WMT", "PG", "JNJ", "UNH", "HD"
        ].iter()
        .map(|s| Symbol::new(s.to_string()))
        .collect();

        Ok(symbols)
    }

    fn get_exchange(&self) -> Exchange {
        self.exchange.clone()
    }
}