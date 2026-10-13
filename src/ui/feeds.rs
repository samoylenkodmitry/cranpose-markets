//! Connects the market state to the venue: REST for the pair list and
//! candle history, WebSocket streams for everything after.

use std::time::Duration;

use cranpose::prelude::*;
use cranpose_core::delay;
use cranpose_coroflow::Handle;

use crate::feed::{candles_url, parse_candles, parse_tickers, tickers_url};
use crate::store::Market;

/// Keeps the feeds of the selected venue, pair and interval open while it is
/// composed.
#[composable]
pub fn Feeds(market: Handle<Market>) {
    let state = market.get();
    let venue = state.venue.get();
    let symbol = state.symbol.get();
    let interval = state.interval.get();
    let client = local_http_client().current();

    LaunchedEffectAsync(venue, {
        let client = client.clone();
        move |_| {
            Box::pin(async move {
                let pairs = client
                    .get_text(&tickers_url(venue))
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|body| parse_tickers(venue, &body).map_err(|e| e.to_string()));
                match pairs {
                    Ok(pairs) => market.get().set_pairs(pairs),
                    Err(error) => {
                        log::warn!("{} pairs: {error}", venue.name());
                        market.get().fall_back();
                    }
                }
            })
        }
    });

    LaunchedEffectAsync((venue, symbol.clone(), interval), {
        let symbol = symbol.clone();
        move |_| {
            Box::pin(async move {
                let url = candles_url(venue, &symbol, interval);
                match client.get_text(&url).await {
                    Ok(body) => match parse_candles(&body) {
                        Ok(candles) => market.get().set_history(candles),
                        Err(error) => log::warn!("{symbol} candles: {error}"),
                    },
                    Err(error) => log::warn!("{symbol} candles: {error}"),
                }
            })
        }
    });

    LaunchedEffectAsync((), move |_| {
        Box::pin(async move {
            loop {
                delay(Duration::from_secs(1)).await;
                market.get().roll_rate();
            }
        })
    });

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    streams::Streams(market, venue, symbol, interval);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
mod streams {
    use std::rc::Rc;

    use cranpose::prelude::*;
    use cranpose_coroflow::Handle;

    use crate::feed::{ALL_TICKERS_STREAM, pair_streams, parse_stream};
    use crate::market::{Interval, Venue};
    use crate::socket::{Listener, Socket};
    use crate::store::Market;

    struct FeedListener {
        market: Rc<Market>,
        venue: Venue,
    }

    impl Listener for FeedListener {
        fn on_text(&self, text: &str) {
            if let Some(event) = parse_stream(text) {
                self.market.apply(event);
            }
        }

        fn on_refused(&self) {
            log::warn!("{} refused the stream", self.venue.name());
            if self.market.venue.get() == self.venue {
                self.market.fall_back();
            }
        }
    }

    fn open(market: Handle<Market>, venue: Venue, streams: &str) -> Socket {
        Socket::open(
            format!("{}{streams}", venue.stream_base()),
            FeedListener {
                market: market.get(),
                venue,
            },
        )
    }

    /// The all-pair ticker stream and the selected pair's book, trade and
    /// candle streams.
    #[composable]
    pub fn Streams(market: Handle<Market>, venue: Venue, symbol: Rc<str>, interval: Interval) {
        DisposableEffect(venue, move |_| {
            let socket = open(market, venue, ALL_TICKERS_STREAM);
            DisposableEffectResult::new(move || drop(socket))
        });
        DisposableEffect((venue, symbol.clone(), interval), move |_| {
            let socket = open(market, venue, &pair_streams(&symbol, interval));
            DisposableEffectResult::new(move || drop(socket))
        });
    }
}
