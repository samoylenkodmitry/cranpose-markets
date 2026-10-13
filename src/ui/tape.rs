use cranpose::prelude::*;
use cranpose_core::with_key;
use cranpose_coroflow::Handle;

use super::app::ROW_HEIGHT;
use super::book::ColumnHeader;
use super::theme::{self, MUTED, TEXT};
use crate::format;
use crate::market::Trade;
use crate::store::Market;

/// The newest trades, newest on top. Each row keeps its identity as new
/// trades push it down, so only the new row composes.
#[composable]
pub fn TradeTape(market: Handle<Market>, modifier: Modifier) {
    let state = market.get();
    let decimals = state.decimals.get();
    let offset = super::local_offset_minutes();
    Column(
        modifier.clip_to_bounds(),
        ColumnSpec::default(),
        move || {
            ColumnHeader(["Price", "Amount", "Time"]);
            market.get().tape.read(|tape| {
                for trade in tape.trades() {
                    let trade = *trade;
                    with_key(&trade.id, || TradeRow(trade, decimals, offset));
                }
            });
        },
    );
}

#[composable]
fn TradeRow(trade: Trade, decimals: usize, offset_minutes: i64) {
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(ROW_HEIGHT)
            .padding_horizontal(12.0),
        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            Text(
                format::grouped(trade.price, decimals),
                Modifier::empty().weight(1.0),
                theme::style(12.0, theme::trend(!trade.buyer_maker)),
            );
            Text(
                format::quantity(trade.quantity),
                Modifier::empty().weight(1.0),
                theme::end(12.0, TEXT),
            );
            Text(
                format::clock(trade.time_ms, offset_minutes, true),
                Modifier::empty().weight(1.0),
                theme::end(12.0, MUTED),
            );
        },
    );
}
