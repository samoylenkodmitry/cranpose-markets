//! Reads Binance's JSON: combined stream messages and REST responses. Prices
//! and quantities arrive as decimal strings.

use std::rc::Rc;

use serde::Deserialize;
use serde_json::value::RawValue;

use crate::market::{Candle, Interval, Pair, Quote, Trade, Venue, base_len};

/// One decoded stream message.
#[derive(Debug, PartialEq)]
pub enum StreamEvent<'a> {
    /// The 24 hour statistics of every pair that changed in the last second.
    Tickers(Vec<(&'a str, Quote)>),
    /// The best price levels of one pair, best first.
    Depth {
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
    },
    Trade(Trade),
    Candle(Candle),
}

/// The stream names for the pair `symbol`: its book, its trades and its
/// candles at `interval`.
pub fn pair_streams(symbol: &str, interval: Interval) -> String {
    let symbol = symbol.to_ascii_lowercase();
    format!(
        "{symbol}@depth20@100ms/{symbol}@aggTrade/{symbol}@kline_{}",
        interval.code()
    )
}

/// The stream of every pair's 24 hour statistics, sent once a second.
pub const ALL_TICKERS_STREAM: &str = "!miniTicker@arr";

pub fn tickers_url(venue: Venue) -> String {
    format!("{}/api/v3/ticker/24hr?type=MINI", venue.rest_base())
}

pub fn candles_url(venue: Venue, symbol: &str, interval: Interval) -> String {
    format!(
        "{}/api/v3/klines?symbol={symbol}&interval={}&limit=1000",
        venue.rest_base(),
        interval.code()
    )
}

#[derive(Deserialize)]
struct Envelope<'a> {
    #[serde(borrow)]
    stream: &'a str,
    #[serde(borrow)]
    data: &'a RawValue,
}

#[derive(Deserialize)]
struct MiniTicker<'a> {
    #[serde(rename = "s")]
    symbol: &'a str,
    #[serde(rename = "c")]
    close: &'a str,
    #[serde(rename = "o")]
    open: &'a str,
    #[serde(rename = "h")]
    high: &'a str,
    #[serde(rename = "l")]
    low: &'a str,
    #[serde(rename = "q")]
    quote_volume: &'a str,
}

#[derive(Deserialize)]
struct Depth<'a> {
    #[serde(borrow)]
    bids: Vec<[&'a str; 2]>,
    #[serde(borrow)]
    asks: Vec<[&'a str; 2]>,
}

#[derive(Deserialize)]
struct AggTrade<'a> {
    #[serde(rename = "a")]
    id: u64,
    #[serde(rename = "p")]
    price: &'a str,
    #[serde(rename = "q")]
    quantity: &'a str,
    #[serde(rename = "T")]
    time_ms: u64,
    #[serde(rename = "m")]
    buyer_maker: bool,
}

#[derive(Deserialize)]
struct KlineEvent<'a> {
    #[serde(rename = "k", borrow)]
    kline: Kline<'a>,
}

#[derive(Deserialize)]
struct Kline<'a> {
    #[serde(rename = "t")]
    open_time_ms: u64,
    #[serde(rename = "o")]
    open: &'a str,
    #[serde(rename = "h")]
    high: &'a str,
    #[serde(rename = "l")]
    low: &'a str,
    #[serde(rename = "c")]
    close: &'a str,
    #[serde(rename = "v")]
    volume: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestTicker<'a> {
    symbol: &'a str,
    last_price: &'a str,
    open_price: &'a str,
    high_price: &'a str,
    low_price: &'a str,
    quote_volume: &'a str,
}

/// A REST candle: open time, open, high, low, close, volume, then fields
/// the chart does not use.
type RestCandle<'a> = (
    u64,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
);

fn number(text: &str) -> f64 {
    text.parse().unwrap_or(0.0)
}

fn quote(close: &str, open: &str, high: &str, low: &str, quote_volume: &str) -> Quote {
    Quote {
        last: number(close),
        open: number(open),
        high: number(high),
        low: number(low),
        quote_volume: number(quote_volume),
    }
}

fn levels(pairs: &[[&str; 2]]) -> Vec<(f64, f64)> {
    pairs
        .iter()
        .map(|[price, quantity]| (number(price), number(quantity)))
        .collect()
}

/// Decodes a combined stream message, or `None` for a stream this app does
/// not subscribe to or a message it cannot read.
pub fn parse_stream(text: &str) -> Option<StreamEvent<'_>> {
    let envelope: Envelope = serde_json::from_str(text).ok()?;
    let data = envelope.data.get();
    let stream = envelope.stream;
    if stream == ALL_TICKERS_STREAM {
        let tickers: Vec<MiniTicker> = serde_json::from_str(data).ok()?;
        return Some(StreamEvent::Tickers(
            tickers
                .into_iter()
                .map(|t| {
                    let quote = quote(t.close, t.open, t.high, t.low, t.quote_volume);
                    (t.symbol, quote)
                })
                .collect(),
        ));
    }
    let (_, kind) = stream.split_once('@')?;
    if kind.starts_with("depth") {
        let depth: Depth = serde_json::from_str(data).ok()?;
        Some(StreamEvent::Depth {
            bids: levels(&depth.bids),
            asks: levels(&depth.asks),
        })
    } else if kind == "aggTrade" {
        let trade: AggTrade = serde_json::from_str(data).ok()?;
        Some(StreamEvent::Trade(Trade {
            id: trade.id,
            price: number(trade.price),
            quantity: number(trade.quantity),
            time_ms: trade.time_ms,
            buyer_maker: trade.buyer_maker,
        }))
    } else if kind.starts_with("kline_") {
        let event: KlineEvent = serde_json::from_str(data).ok()?;
        let k = event.kline;
        Some(StreamEvent::Candle(Candle {
            open_time_ms: k.open_time_ms,
            open: number(k.open),
            high: number(k.high),
            low: number(k.low),
            close: number(k.close),
            volume: number(k.volume),
        }))
    } else {
        None
    }
}

/// Reads the 24 hour tickers into the venue's watchlist pairs.
pub fn parse_tickers(venue: Venue, body: &str) -> Result<Vec<Pair>, serde_json::Error> {
    let tickers: Vec<RestTicker> = serde_json::from_str(body)?;
    Ok(crate::market::watchlist_pairs(
        venue,
        tickers.into_iter().filter_map(|t| {
            Some(Pair {
                symbol: Rc::from(t.symbol),
                base_len: base_len(venue, t.symbol)?,
                quote: quote(
                    t.last_price,
                    t.open_price,
                    t.high_price,
                    t.low_price,
                    t.quote_volume,
                ),
            })
        }),
    ))
}

/// Reads REST candles, oldest first.
pub fn parse_candles(body: &str) -> Result<Vec<Candle>, serde_json::Error> {
    let candles: Vec<RestCandle> = serde_json::from_str(body)?;
    Ok(candles
        .into_iter()
        .map(
            |(open_time_ms, open, high, low, close, volume, ..)| Candle {
                open_time_ms,
                open: number(open),
                high: number(high),
                low: number(low),
                close: number(close),
                volume: number(volume),
            },
        )
        .collect())
}
