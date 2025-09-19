//! Market data normalization and processing

pub mod universal;
pub mod normalizers;
pub mod symbol_mapper;
pub mod time_sync;
pub mod unified_feed;
pub mod spike_bridge;

pub use universal::MarketDataNormalizer;
pub use normalizers::{BinanceNormalizer, CoinbaseNormalizer};
pub use symbol_mapper::SymbolMapper;
pub use time_sync::{TimeSynchronizer, DriftDetector, DriftWarning};
pub use unified_feed::{UnifiedMarketFeed, UnifiedMarketEvent, UnifiedFeedConfig, AggregatedMarketData};
pub use spike_bridge::{MarketDataSpikeBridge, SpikeBridgeConfig, MarketSpikeIntegration};