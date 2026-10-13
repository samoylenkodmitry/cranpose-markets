//! Market data types and the rules that fold a stream of updates into them.
//! Nothing here touches the UI, so tests drive it directly.

use std::collections::VecDeque;

/// Where the data comes from. Binance serves most of the world; it refuses
/// visitors from the United States, who get Binance.US, which speaks the same
/// protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Venue {
    Binance,
    BinanceUs,
}

impl Venue {
    pub fn name(self) -> &'static str {
        match self {
            Self::Binance => "Binance",
            Self::BinanceUs => "Binance.US",
        }
    }

    /// The REST host. Binance's market data host takes no API key.
    pub fn rest_base(self) -> &'static str {
        match self {
            Self::Binance => "https://data-api.binance.vision",
            Self::BinanceUs => "https://api.binance.us",
        }
    }

    /// The combined stream endpoint; stream names follow `?streams=`.
    pub fn stream_base(self) -> &'static str {
        match self {
            Self::Binance => "wss://data-stream.binance.vision/stream?streams=",
            Self::BinanceUs => "wss://stream.binance.us:9443/stream?streams=",
        }
    }

    /// The quote assets whose pairs the watchlist shows.
    pub fn quote_assets(self) -> &'static [&'static str] {
        match self {
            Self::Binance => &["USDT"],
            Self::BinanceUs => &["USD", "USDT"],
        }
    }

    /// The venue a page query string names: `venue=us` picks Binance.US.
    pub fn from_query(query: &str) -> Self {
        let us = query
            .trim_start_matches('?')
            .split('&')
            .any(|pair| pair.eq_ignore_ascii_case("venue=us"));
        if us { Self::BinanceUs } else { Self::Binance }
    }

    /// The venue to try when this one refuses the visitor.
    pub fn fallback(self) -> Option<Self> {
        match self {
            Self::Binance => Some(Self::BinanceUs),
            Self::BinanceUs => None,
        }
    }

    /// The candle intervals the venue serves.
    pub fn intervals(self) -> &'static [Interval] {
        match self {
            Self::Binance => &Interval::ALL,
            Self::BinanceUs => &Interval::ALL[1..],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Interval {
    S1,
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

impl Interval {
    pub const ALL: [Self; 7] = [
        Self::S1,
        Self::M1,
        Self::M5,
        Self::M15,
        Self::H1,
        Self::H4,
        Self::D1,
    ];

    /// The name the exchange uses, which is also the label.
    pub fn code(self) -> &'static str {
        match self {
            Self::S1 => "1s",
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1h",
            Self::H4 => "4h",
            Self::D1 => "1d",
        }
    }

    pub fn millis(self) -> u64 {
        const SECOND: u64 = 1_000;
        match self {
            Self::S1 => SECOND,
            Self::M1 => 60 * SECOND,
            Self::M5 => 5 * 60 * SECOND,
            Self::M15 => 15 * 60 * SECOND,
            Self::H1 => 3_600 * SECOND,
            Self::H4 => 4 * 3_600 * SECOND,
            Self::D1 => 24 * 3_600 * SECOND,
        }
    }
}

/// A pair's rolling 24 hour statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quote {
    pub last: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    /// Traded value in the quote asset.
    pub quote_volume: f64,
}

impl Quote {
    /// The change over 24 hours, in percent.
    pub fn change_percent(&self) -> f64 {
        if self.open > 0.0 {
            (self.last - self.open) / self.open * 100.0
        } else {
            0.0
        }
    }
}

/// One order book price level. `total` is the quantity summed from the best
/// price out to this level.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Level {
    pub price: f64,
    pub quantity: f64,
    pub total: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trade {
    pub id: u64,
    pub price: f64,
    pub quantity: f64,
    pub time_ms: u64,
    /// Whether the buyer placed the resting order, so the seller took it.
    pub buyer_maker: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candle {
    pub open_time_ms: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// The candles of one pair and interval, oldest first.
#[derive(Debug, Default)]
pub struct CandleSeries {
    candles: Vec<Candle>,
    /// The newest candle the stream sent, kept while the history loads so
    /// the history does not hide it.
    live: Option<Candle>,
}

impl CandleSeries {
    /// The most candles a series keeps.
    pub const CAPACITY: usize = 1_500;

    pub fn candles(&self) -> &[Candle] {
        &self.candles
    }

    pub fn clear(&mut self) {
        self.candles.clear();
        self.live = None;
    }

    /// Replaces the series with loaded history, then lays the newest
    /// streamed candle over it.
    pub fn replace_history(&mut self, history: impl IntoIterator<Item = Candle>) {
        self.candles.clear();
        self.candles.extend(history);
        if let Some(live) = self.live {
            self.merge(live);
        }
        self.trim();
    }

    /// Folds in a streamed candle: it replaces the candle with the same open
    /// time, follows the newest one, or is dropped when older.
    pub fn apply(&mut self, candle: Candle) {
        if self
            .live
            .is_none_or(|live| live.open_time_ms <= candle.open_time_ms)
        {
            self.live = Some(candle);
        }
        self.merge(candle);
        self.trim();
    }

    fn merge(&mut self, candle: Candle) {
        match self.candles.last_mut() {
            Some(last) if last.open_time_ms == candle.open_time_ms => *last = candle,
            Some(last) if last.open_time_ms > candle.open_time_ms => {}
            _ => self.candles.push(candle),
        }
    }

    fn trim(&mut self) {
        let excess = self.candles.len().saturating_sub(Self::CAPACITY);
        if excess > 0 {
            self.candles.drain(..excess);
        }
    }
}

/// The newest trades of one pair, newest first.
#[derive(Debug, Default)]
pub struct TradeTape {
    trades: VecDeque<Trade>,
}

impl TradeTape {
    /// The most trades the tape keeps.
    pub const CAPACITY: usize = 64;

    pub fn trades(&self) -> impl Iterator<Item = &Trade> {
        self.trades.iter()
    }

    pub fn newest(&self) -> Option<&Trade> {
        self.trades.front()
    }

    pub fn clear(&mut self) {
        self.trades.clear();
    }

    /// Adds a trade unless the tape already holds it or a newer one: a
    /// reconnected stream can send a trade again.
    pub fn push(&mut self, trade: Trade) {
        if self
            .trades
            .front()
            .is_some_and(|newest| newest.id >= trade.id)
        {
            return;
        }
        if self.trades.len() == Self::CAPACITY {
            self.trades.pop_back();
        }
        self.trades.push_front(trade);
    }
}

/// Fills `levels` from price and quantity pairs, best price first, with the
/// running total of quantity.
pub fn fill_levels(levels: &mut Vec<Level>, pairs: impl IntoIterator<Item = (f64, f64)>) {
    levels.clear();
    let mut total = 0.0;
    for (price, quantity) in pairs {
        total += quantity;
        levels.push(Level {
            price,
            quantity,
            total,
        });
    }
}

/// A pair as the watchlist lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Pair {
    pub symbol: String,
    /// Where the base asset ends in `symbol`; the quote asset follows.
    pub base_len: usize,
    pub quote: Quote,
}

impl Pair {
    pub fn base(&self) -> &str {
        &self.symbol[..self.base_len]
    }

    pub fn quote_asset(&self) -> &str {
        &self.symbol[self.base_len..]
    }
}

/// Base assets pegged to a currency: their pairs barely move, so the
/// watchlist leaves them out.
const PEGGED_BASES: [&str; 10] = [
    "USDC", "USDT", "FDUSD", "TUSD", "USDP", "DAI", "EUR", "USD1", "BUSD", "AEUR",
];

/// Picks the venue's pairs from its 24 hour tickers, most traded first. A
/// pair that did not trade in the last day is gone from the venue.
pub fn watchlist_pairs(venue: Venue, tickers: impl IntoIterator<Item = Pair>) -> Vec<Pair> {
    let mut pairs: Vec<Pair> = tickers
        .into_iter()
        .filter(|pair| {
            pair.quote.quote_volume > 0.0
                && pair.quote.last > 0.0
                && pair.base_len > 0
                && venue.quote_assets().contains(&pair.quote_asset())
                && !PEGGED_BASES.contains(&pair.base())
        })
        .collect();
    pairs.sort_by(|a, b| b.quote.quote_volume.total_cmp(&a.quote.quote_volume));
    pairs
}

/// Quote assets whose names end in a venue quote: `ZILBUSD` is ZIL in BUSD,
/// not ZILB in USD.
const LOOKALIKE_QUOTES: [&str; 4] = ["BUSD", "TUSD", "FDUSD", "PYUSD"];

/// Splits `symbol` into base and quote with the venue's quote assets.
pub fn base_len(venue: Venue, symbol: &str) -> Option<usize> {
    if LOOKALIKE_QUOTES.iter().any(|quote| symbol.ends_with(quote)) {
        return None;
    }
    venue
        .quote_assets()
        .iter()
        .find(|quote| symbol.ends_with(*quote) && symbol.len() > quote.len())
        .map(|quote| symbol.len() - quote.len())
}
