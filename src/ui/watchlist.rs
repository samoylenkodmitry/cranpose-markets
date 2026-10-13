use std::rc::Rc;
use std::time::Duration;

use cranpose::prelude::*;
use cranpose_animation::prelude::{Easing, animateColorAsState, tween};
use cranpose_core::delay;
use cranpose_coroflow::Handle;

use super::book::ColumnHeader;
use super::theme::{self, ACCENT, MUTED, RAISED, TEXT};
use crate::format;
use crate::market::Quote;
use crate::store::Market;

const WATCH_ROW_HEIGHT: f32 = 30.0;

/// Every pair of the venue, most traded first. Only the rows in view
/// compose, and a ticker update recomposes the row it changed.
#[composable]
pub fn Watchlist(market: Handle<Market>) {
    let count = market
        .get()
        .watchlist
        .read(|watchlist| watchlist.rows.len());
    let list = rememberLazyListState();
    Column(
        Modifier::empty().fill_max_size(),
        ColumnSpec::default(),
        move || {
            ColumnHeader(["Pair", "Last", "24h"]);
            LazyColumn(
                Modifier::empty().fill_max_width().weight(1.0),
                list,
                LazyColumnSpec::default(),
                move |scope| {
                    scope.items(count, move |index| {
                        let row = market.get().watchlist.read(|watchlist| {
                            watchlist.rows.get(index).map(|row| {
                                (
                                    Rc::clone(&row.pair.symbol),
                                    row.pair.base_len,
                                    row.quote.handle(),
                                )
                            })
                        });
                        if let Some((symbol, base_len, quote)) = row {
                            WatchRow(market, symbol, base_len, quote);
                        }
                    });
                },
            );
        },
    );
}

#[composable]
fn WatchRow(market: Handle<Market>, symbol: Rc<str>, base_len: usize, quote: MutableState<Quote>) {
    let selected = market.get().symbol.with(|current| *current == symbol);
    let current = quote.get();
    let (last, change) = (current.last, current.change_percent());
    let flash = PriceFlash(last);
    let decimals = format::price_decimals(last);
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(WATCH_ROW_HEIGHT)
            .background(if selected { RAISED } else { Color::TRANSPARENT })
            .draw_behind(move |draw| {
                let flash = flash.get();
                if flash.3 > 0.0 {
                    draw.draw_rect(Brush::solid(flash));
                }
                if selected {
                    let height = draw.size().height;
                    draw.draw_rect_at(
                        Rect {
                            x: 0.0,
                            y: 0.0,
                            width: 2.0,
                            height,
                        },
                        Brush::solid(ACCENT),
                    );
                }
            })
            .clickable({
                let symbol = Rc::clone(&symbol);
                move |_| market.get().select(Rc::clone(&symbol))
            })
            .padding_horizontal(12.0),
        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            Row(
                Modifier::empty().weight(1.0),
                RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                {
                    let symbol = Rc::clone(&symbol);
                    move || {
                        let (base, quote_asset) = symbol.split_at(base_len);
                        Text(base.to_owned(), Modifier::empty(), theme::bold(13.0, TEXT));
                        Text(
                            format!("/{quote_asset}"),
                            Modifier::empty(),
                            theme::style(11.0, MUTED),
                        );
                    }
                },
            );
            Text(
                format::grouped(last, decimals),
                Modifier::empty().weight(1.0),
                theme::end(13.0, TEXT),
            );
            Text(
                format::percent(change),
                Modifier::empty().width(72.0),
                theme::end(13.0, theme::trend(change >= 0.0)),
            );
        },
    );
}

/// A tint that lights up when `price` moves, green up and red down, and
/// fades out.
#[composable]
fn PriceFlash(price: f64) -> State<Color> {
    let previous = rememberMutableStateOf(|| price);
    let lit = rememberMutableStateOf(|| None::<bool>);
    LaunchedEffectAsync(price.to_bits(), move |_| {
        Box::pin(async move {
            let before = previous.get();
            previous.set(price);
            if before == price {
                return;
            }
            lit.set(Some(price > before));
            delay(Duration::from_millis(250)).await;
            lit.set(None);
        })
    });
    let target = match lit.get() {
        Some(up) => theme::trend(up).with_alpha(0.22),
        None => Color::TRANSPARENT,
    };
    animateColorAsState(target, tween(450, Easing::EaseOut), "price flash")
}
