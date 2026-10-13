use cranpose::prelude::*;
use cranpose::text::SharedText;
use cranpose_coroflow::Handle;

use super::app::ROW_HEIGHT;
use super::theme::{self, DIVIDER, FAINT, MUTED, TEXT};
use crate::format;
use crate::market::Level;
use crate::store::Market;

/// Asks above, bids below, `rows` levels each, with bars for the quantity
/// summed from the best price.
#[composable]
pub fn OrderBook(market: Handle<Market>, rows: usize) {
    let state = market.get();
    let decimals = state.decimals.get();
    let scale = state.book_scale;
    Column(
        Modifier::empty().fill_max_width().clip_to_bounds(),
        ColumnSpec::default(),
        move || {
            let state = market.get();
            ColumnHeader(["Price", "Amount", "Total"]);
            for level in state.asks.levels[..rows].iter().rev() {
                BookRow(*level, false, decimals, scale);
            }
            Spread(market, decimals);
            for level in &state.bids.levels[..rows] {
                BookRow(*level, true, decimals, scale);
            }
        },
    );
}

/// Three column titles above a table.
#[composable]
pub fn ColumnHeader(titles: [&'static str; 3]) {
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(24.0)
            .padding_horizontal(12.0),
        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            let [first, second, third] = titles;
            Text(
                first,
                Modifier::empty().weight(1.0),
                theme::style(11.0, FAINT),
            );
            Text(
                second,
                Modifier::empty().weight(1.0),
                theme::end(11.0, FAINT),
            );
            Text(
                third,
                Modifier::empty().weight(1.0),
                theme::end(11.0, FAINT),
            );
        },
    );
}

#[composable]
fn BookRow(level: MutableState<Level>, bid: bool, decimals: usize, scale: MutableState<f64>) {
    let Level {
        price,
        quantity,
        total,
    } = level.get();
    let color = theme::trend(bid);
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(ROW_HEIGHT)
            .draw_behind(move |draw| {
                let scale = scale.get();
                if scale <= 0.0 {
                    return;
                }
                let size = draw.size();
                let width = size.width * (level.get().total / scale).min(1.0) as f32;
                draw.draw_rect_at(
                    Rect {
                        x: size.width - width,
                        y: 1.0,
                        width,
                        height: size.height - 2.0,
                    },
                    Brush::solid(color.with_alpha(0.13)),
                );
            })
            .padding_horizontal(12.0),
        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            if price <= 0.0 {
                return;
            }
            Text(
                format::grouped(price, decimals),
                Modifier::empty().weight(1.0),
                theme::style(12.0, color),
            );
            Text(
                format::quantity(quantity),
                Modifier::empty().weight(1.0),
                theme::end(12.0, TEXT),
            );
            Text(
                format::quantity(total),
                Modifier::empty().weight(1.0),
                theme::end(12.0, MUTED),
            );
        },
    );
}

/// The gap between the best bid and the best ask.
#[composable]
fn Spread(market: Handle<Market>, decimals: usize) {
    let state = market.get();
    let bid = state.bids.levels[0].get().price;
    let ask = state.asks.levels[0].get().price;
    let text = if bid > 0.0 && ask > 0.0 {
        let spread = ask - bid;
        SharedText::from(format!(
            "{}  spread {} ({:.3}%)",
            format::grouped((ask + bid) / 2.0, decimals),
            format::grouped(spread, decimals),
            spread / ask * 100.0
        ))
    } else {
        SharedText::from("Waiting for the book")
    };
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(26.0)
            .padding_horizontal(12.0)
            .border(1.0, DIVIDER, RoundedCornerShape::uniform(0.0)),
        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            Text(text.clone(), Modifier::empty(), theme::bold(13.0, TEXT));
        },
    );
}
