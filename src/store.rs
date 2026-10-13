//! The app's market state. Small values live in Compose state, so only the
//! composables that read one recompose when it changes. Collections change in
//! place behind a version state, which their readers subscribe to.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use cranpose_core::{MutableState, OwnedMutableState, mutableStateOf, ownedMutableStateOf};

use crate::feed::StreamEvent;
use crate::format::price_decimals;
use crate::market::{
    Candle, CandleSeries, Interval, Level, Pair, Quote, TradeTape, Venue, fill_levels,
};

/// How many levels of each book side the stream sends.
pub const BOOK_DEPTH: usize = 20;

/// A value changed in place. Reading it through [`Versioned::read`]
/// subscribes the reader, and each write notifies the readers once.
pub struct Versioned<T> {
    value: RefCell<T>,
    version: MutableState<u64>,
}

impl<T> Versioned<T> {
    fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
            version: mutableStateOf(0),
        }
    }

    pub fn read<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        self.version.with(|_| read(&self.value.borrow()))
    }

    fn write(&self, write: impl FnOnce(&mut T)) {
        write(&mut self.value.borrow_mut());
        self.version
            .update(|version| *version = version.wrapping_add(1));
    }
}

/// A watchlist row. Its quote is its own state, so a ticker update
/// recomposes that row alone.
pub struct WatchRow {
    pub pair: Pair,
    pub quote: OwnedMutableState<Quote>,
}

#[derive(Default)]
pub struct Watchlist {
    pub rows: Vec<WatchRow>,
    index: HashMap<String, usize>,
}

impl Watchlist {
    pub fn row(&self, symbol: &str) -> Option<&WatchRow> {
        self.index.get(symbol).map(|&index| &self.rows[index])
    }
}

/// One side of the book: a state per level, so a level whose price and
/// quantity stay the same does not recompose.
pub struct BookSide {
    pub levels: Vec<MutableState<Level>>,
    scratch: RefCell<Vec<Level>>,
}

impl BookSide {
    fn new() -> Self {
        Self {
            levels: (0..BOOK_DEPTH)
                .map(|_| mutableStateOf(Level::default()))
                .collect(),
            scratch: RefCell::new(Vec::with_capacity(BOOK_DEPTH)),
        }
    }

    fn set(&self, pairs: impl IntoIterator<Item = (f64, f64)>) -> f64 {
        let mut scratch = self.scratch.borrow_mut();
        fill_levels(&mut scratch, pairs.into_iter().take(BOOK_DEPTH));
        for (index, state) in self.levels.iter().enumerate() {
            state.set(scratch.get(index).copied().unwrap_or_default());
        }
        scratch.last().map_or(0.0, |level| level.total)
    }

    fn clear(&self) {
        self.set(std::iter::empty());
    }
}

/// The state of the feed's connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Connecting,
    Live,
    /// Every venue refused the visitor or could not be reached.
    Unavailable,
}

pub struct Market {
    pub venue: MutableState<Venue>,
    pub symbol: MutableState<String>,
    pub interval: MutableState<Interval>,
    /// The decimals the selected pair's prices show.
    pub decimals: MutableState<usize>,
    pub link: MutableState<Link>,
    /// Market data items applied in the last second: tickers, book levels,
    /// trades and candles.
    pub updates_per_second: MutableState<u32>,
    pub watchlist: Versioned<Watchlist>,
    pub bids: BookSide,
    pub asks: BookSide,
    /// The larger side's total quantity, which scales the depth bars.
    pub book_scale: MutableState<f64>,
    pub tape: Versioned<TradeTape>,
    pub candles: Versioned<CandleSeries>,
    updates: Cell<u32>,
}

impl Market {
    /// A market that starts on `venue`.
    pub fn new(venue: Venue) -> Self {
        Self {
            venue: mutableStateOf(venue),
            symbol: mutableStateOf(String::from("BTCUSDT")),
            interval: mutableStateOf(Interval::M1),
            decimals: mutableStateOf(2),
            link: mutableStateOf(Link::Connecting),
            updates_per_second: mutableStateOf(0),
            watchlist: Versioned::new(Watchlist::default()),
            bids: BookSide::new(),
            asks: BookSide::new(),
            book_scale: mutableStateOf(0.0),
            tape: Versioned::new(TradeTape::default()),
            candles: Versioned::new(CandleSeries::default()),
            updates: Cell::new(0),
        }
    }

    /// Shows `pairs` in the watchlist. When the selected symbol is not among
    /// them, the most traded pair takes its place.
    pub fn set_pairs(&self, pairs: Vec<Pair>) {
        self.watchlist.write(|watchlist| {
            watchlist.index = pairs
                .iter()
                .enumerate()
                .map(|(index, pair)| (pair.symbol.clone(), index))
                .collect();
            watchlist.rows = pairs
                .into_iter()
                .map(|pair| WatchRow {
                    quote: ownedMutableStateOf(pair.quote),
                    pair,
                })
                .collect();
        });
        let fallback = self.watchlist.read(|watchlist| {
            let selected = self.symbol.with(|symbol| watchlist.row(symbol).is_some());
            (!selected)
                .then(|| watchlist.rows.first().map(|row| row.pair.symbol.clone()))
                .flatten()
        });
        if let Some(symbol) = fallback {
            self.select(symbol);
        }
        self.refresh_decimals();
    }

    pub fn select(&self, symbol: String) {
        if self.symbol.with(|current| *current != symbol) {
            self.clear_pair();
            self.symbol.set(symbol);
            self.refresh_decimals();
        }
    }

    fn refresh_decimals(&self) {
        let last = self.symbol.with(|symbol| {
            self.watchlist
                .read(|watchlist| watchlist.row(symbol).map(|row| row.pair.quote.last))
        });
        if let Some(last) = last {
            self.decimals.set(price_decimals(last));
        }
    }

    pub fn select_interval(&self, interval: Interval) {
        if self.interval.get() != interval {
            self.candles.write(CandleSeries::clear);
            self.interval.set(interval);
        }
    }

    /// Moves to the next venue, or reports that none is left.
    pub fn fall_back(&self) {
        match self.venue.get().fallback() {
            Some(next) => {
                self.clear_pair();
                if !next.intervals().contains(&self.interval.get()) {
                    self.interval.set(Interval::M1);
                }
                self.venue.set(next);
            }
            None => self.link.set(Link::Unavailable),
        }
    }

    fn clear_pair(&self) {
        self.bids.clear();
        self.asks.clear();
        self.book_scale.set(0.0);
        self.tape.write(TradeTape::clear);
        self.candles.write(CandleSeries::clear);
    }

    pub fn set_history(&self, candles: Vec<Candle>) {
        self.candles.write(|series| series.replace_history(candles));
    }

    /// Applies one stream message.
    pub fn apply(&self, event: StreamEvent<'_>) {
        let items = match event {
            StreamEvent::Tickers(tickers) => {
                let count = tickers.len();
                self.watchlist.value.borrow().apply(&tickers);
                count
            }
            StreamEvent::Depth { bids, asks } => {
                let count = bids.len() + asks.len();
                let bid_total = self.bids.set(bids);
                let ask_total = self.asks.set(asks);
                self.book_scale.set(bid_total.max(ask_total));
                count
            }
            StreamEvent::Trade(trade) => {
                self.tape.write(|tape| tape.push(trade));
                1
            }
            StreamEvent::Candle(candle) => {
                self.candles.write(|series| series.apply(candle));
                1
            }
        };
        self.link.set(Link::Live);
        self.updates
            .set(self.updates.get().saturating_add(items as u32));
    }

    /// Publishes the count of the last second's updates and starts a new one.
    pub fn roll_rate(&self) {
        self.updates_per_second.set(self.updates.replace(0));
    }
}

impl Watchlist {
    fn apply(&self, tickers: &[(&str, Quote)]) {
        for (symbol, quote) in tickers {
            if let Some(row) = self.row(symbol) {
                row.quote.handle().set(*quote);
            }
        }
    }
}
