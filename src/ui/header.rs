use cranpose::prelude::*;
use cranpose::text::SharedText;
use cranpose_coroflow::Handle;

use super::theme::{self, FAINT, MUTED, TEXT};
use crate::format;
use crate::market::{Quote, base_len};
use crate::store::{Link, Market};

/// The feed's state and the rate of market data it applies.
#[composable]
pub fn StatusLine(market: Handle<Market>) {
    let state = market.get();
    let link = state.link.get();
    let venue = state.venue.get();
    let rate = state.updates_per_second.get();
    let (color, label) = match link {
        Link::Connecting => (FAINT, format!("Connecting to {}", venue.name()).into()),
        Link::Live => (theme::UP, format!("Live from {}", venue.name()).into()),
        Link::Unavailable => (theme::DOWN, SharedText::from("No venue answered")),
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
            SourceLink();
        },
    );
}

const SOURCE_URL: &str = "https://github.com/samoylenkodmitry/cranpose-markets";

/// Opens the app's source code.
#[composable]
fn SourceLink() {
    let uri = local_uri_handler().current();
    Text(
        "Source",
        Modifier::empty()
            .clickable(move |_| {
                if let Err(error) = uri.open_uri(SOURCE_URL) {
                    log::warn!("cannot open {SOURCE_URL}: {error}");
                }
            })
            .padding_horizontal(4.0),
        theme::style(12.0, theme::ACCENT),
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
                    SharedText::from(row.pair.base().to_owned()),
                    SharedText::from(format!("/ {}", row.pair.quote_asset())),
                    Some(row.quote.handle()),
                ),
                None => {
                    let split = base_len(state.venue.get(), symbol).unwrap_or(symbol.len());
                    let (base, quote_asset) = symbol.split_at(split);
                    (
                        SharedText::from(base.to_owned()),
                        SharedText::from(format!("/ {quote_asset}")),
                        None,
                    )
                }
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
            let (base, quote_asset) = (base.clone(), quote_asset.clone());
            Column(Modifier::empty(), ColumnSpec::default(), move || {
                Text(base.clone(), Modifier::empty(), theme::bold(18.0, TEXT));
                Text(
                    quote_asset.clone(),
                    Modifier::empty(),
                    theme::style(12.0, MUTED),
                );
            });
            LastPrice(market, quote);
            if let Some(quote) = quote {
                Statistics(quote.get(), wide);
            }
        },
    );
}

/// The price of the newest trade, colored by its side. Before the first
/// trade arrives, the ticker's last price.
#[composable]
fn LastPrice(market: Handle<Market>, quote: Option<MutableState<Quote>>) {
    let state = market.get();
    let decimals = state.decimals.get();
    let newest = state
        .tape
        .read(|tape| tape.newest().map(|trade| (trade.price, trade.buyer_maker)));
    let (text, color) = match (newest, quote) {
        (Some((price, buyer_maker)), _) => {
            (format::grouped(price, decimals), theme::trend(!buyer_maker))
        }
        (None, Some(quote)) => (format::grouped(quote.get().last, decimals), TEXT),
        (None, None) => (String::from("--"), TEXT),
    };
    Text(text, Modifier::empty(), theme::bold(22.0, color));
}

#[composable]
fn Statistics(quote: Quote, wide: bool) {
    let change = quote.change_percent();
    let decimals = format::price_decimals(quote.last);
    let trend = theme::trend(change >= 0.0);
    Stat("24h change", format::percent(change).into(), trend);
    if wide {
        Stat(
            "24h high",
            format::grouped(quote.high, decimals).into(),
            TEXT,
        );
        Stat("24h low", format::grouped(quote.low, decimals).into(), TEXT);
        Stat(
            "24h volume",
            format::compact(quote.quote_volume).into(),
            TEXT,
        );
    }
}

#[composable]
fn Stat(label: &'static str, value: SharedText, color: Color) {
    Column(Modifier::empty(), ColumnSpec::default(), move || {
        Text(label, Modifier::empty(), theme::style(11.0, MUTED));
        Text(value.clone(), Modifier::empty(), theme::style(13.0, color));
    });
}
