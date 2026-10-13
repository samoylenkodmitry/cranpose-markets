use cranpose::prelude::*;
use cranpose_coroflow::Handle;

use super::theme::{self, FAINT, MUTED, TEXT};
use crate::format;
use crate::market::Quote;
use crate::store::{Link, Market};

/// The feed's state and the rate of market data it applies.
#[composable]
pub fn StatusLine(market: Handle<Market>) {
    let state = market.get();
    let link = state.link.get();
    let venue = state.venue.get();
    let rate = state.updates_per_second.get();
    let (color, label) = match link {
        Link::Connecting => (FAINT, format!("Connecting to {}", venue.name())),
        Link::Live => (theme::UP, format!("Live from {}", venue.name())),
        Link::Unavailable => (theme::DOWN, String::from("No venue answered")),
    };
    Row(
        Modifier::empty().height(24.0).padding_horizontal(12.0),
        RowSpec::default()
            .vertical_alignment(VerticalAlignment::CenterVertically)
            .horizontal_arrangement(LinearArrangement::SpacedBy(8.0)),
        move || {
            Canvas(Modifier::empty().size_points(8.0, 8.0), move |draw| {
                draw.draw_circle(Brush::solid(color), Point::new(4.0, 4.0), 4.0);
            });
            Text(label.clone(), Modifier::empty(), theme::style(12.0, MUTED));
            Text(
                format!("{} updates/s", format::grouped(f64::from(rate), 0)),
                Modifier::empty(),
                theme::style(12.0, MUTED),
            );
        },
    );
}

/// The selected pair: its name, last price and 24 hour statistics.
#[composable]
pub fn PairHeader(market: Handle<Market>, wide: bool) {
    let state = market.get();
    let (base, quote_asset, quote) = state.symbol.with(|symbol| {
        state
            .watchlist
            .read(|watchlist| match watchlist.row(symbol) {
                Some(row) => (
                    row.pair.base().to_owned(),
                    row.pair.quote_asset().to_owned(),
                    Some(row.quote.handle()),
                ),
                None => (symbol.clone(), String::new(), None),
            })
    });
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(if wide { 64.0 } else { 56.0 })
            .padding_horizontal(16.0),
        RowSpec::default()
            .vertical_alignment(VerticalAlignment::CenterVertically)
            .horizontal_arrangement(LinearArrangement::SpacedBy(if wide { 28.0 } else { 16.0 })),
        move || {
            Column(Modifier::empty(), ColumnSpec::default(), {
                let base = base.clone();
                let quote_asset = quote_asset.clone();
                move || {
                    Text(base.clone(), Modifier::empty(), theme::bold(18.0, TEXT));
                    Text(
                        format!("/ {quote_asset}"),
                        Modifier::empty(),
                        theme::style(12.0, MUTED),
                    );
                }
            });
            LastPrice(market);
            if let Some(quote) = quote {
                Statistics(quote.get(), wide);
            }
        },
    );
}

/// The price of the newest trade, colored by its side.
#[composable]
fn LastPrice(market: Handle<Market>) {
    let state = market.get();
    let newest = state
        .tape
        .read(|tape| tape.newest().map(|trade| (trade.price, trade.buyer_maker)));
    let (text, color) = match newest {
        Some((price, buyer_maker)) => (
            format::grouped(price, format::price_decimals(price)),
            theme::trend(!buyer_maker),
        ),
        None => (String::from("--"), TEXT),
    };
    Text(text, Modifier::empty(), theme::bold(22.0, color));
}

#[composable]
fn Statistics(quote: Quote, wide: bool) {
    let change = quote.change_percent();
    let decimals = format::price_decimals(quote.last);
    Stat(
        "24h change",
        format::percent(change),
        theme::trend(change >= 0.0),
    );
    if wide {
        Stat("24h high", format::grouped(quote.high, decimals), TEXT);
        Stat("24h low", format::grouped(quote.low, decimals), TEXT);
        Stat("24h volume", format::compact(quote.quote_volume), TEXT);
    }
}

#[composable]
fn Stat(label: &'static str, value: String, color: Color) {
    Column(Modifier::empty(), ColumnSpec::default(), move || {
        Text(label, Modifier::empty(), theme::style(11.0, MUTED));
        Text(value.clone(), Modifier::empty(), theme::style(13.0, color));
    });
}
