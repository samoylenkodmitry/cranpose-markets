use cranpose::prelude::*;
use cranpose_coroflow::{Handle, rememberHandle};

use super::book::OrderBook;
use super::chart::{ChartToolbar, PriceChart};
use super::feeds::Feeds;
use super::header::{PairHeader, StatusLine};
use super::tape::TradeTape;
use super::theme::{self, BACKGROUND, DIVIDER, MUTED, PANEL, TEXT};
use super::watchlist::Watchlist;
use crate::store::Market;

/// From this width on, the watchlist, chart and book stand side by side.
const WIDE_MIN_WIDTH: f32 = 1_080.0;
const TOP_BAR_HEIGHT: f32 = 40.0;
const WATCHLIST_WIDTH: f32 = 300.0;
const BOOK_WIDTH: f32 = 320.0;
/// The height of a book, tape or watchlist row.
pub const ROW_HEIGHT: f32 = 18.0;

#[composable]
pub fn MarketsApp() {
    let market = rememberHandle(Market::default);
    Feeds(market);
    let size = rememberMutableStateOf(|| Size::new(0.0, 0.0));
    Box(
        Modifier::empty()
            .fill_max_size()
            .background(BACKGROUND)
            .report_size_state(size),
        BoxSpec::default(),
        move || {
            let Size { width, height } = size.get();
            if width >= WIDE_MIN_WIDTH {
                WideLayout(market);
            } else {
                NarrowLayout(market, height);
            }
        },
    );
}

/// How many levels of each book side fit in `height` under the column
/// titles and beside the spread row.
fn book_rows(height: f32) -> usize {
    let fixed = 24.0 + 26.0;
    (((height - fixed) / ROW_HEIGHT / 2.0).floor().max(0.0) as usize)
        .clamp(4, crate::store::BOOK_DEPTH)
}

#[composable]
fn WideLayout(market: Handle<Market>) {
    let side = rememberMutableStateOf(|| Size::new(0.0, 0.0));
    Column(
        Modifier::empty().fill_max_size(),
        ColumnSpec::default(),
        move || {
            TopBar(market);
            Row(
                Modifier::empty().fill_max_width().weight(1.0),
                RowSpec::default(),
                move || {
                    Panel(
                        Modifier::empty().width(WATCHLIST_WIDTH).fill_max_height(),
                        move || {
                            Watchlist(market);
                        },
                    );
                    Column(
                        Modifier::empty().weight(1.0).fill_max_height(),
                        ColumnSpec::default(),
                        move || {
                            PairHeader(market, true);
                            ChartToolbar(market);
                            PriceChart(market, Modifier::empty().fill_max_width().weight(1.0));
                        },
                    );
                    Panel(
                        Modifier::empty()
                            .width(BOOK_WIDTH)
                            .fill_max_height()
                            .report_size_state(side),
                        move || {
                            let rows = book_rows(side.get().height * 0.6);
                            Column(
                                Modifier::empty().fill_max_size(),
                                ColumnSpec::default(),
                                move || {
                                    OrderBook(market, rows);
                                    Divider();
                                    TradeTape(
                                        market,
                                        Modifier::empty().fill_max_width().weight(1.0),
                                    );
                                },
                            );
                        },
                    );
                },
            );
        },
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Book,
    Trades,
    Markets,
}

impl Tab {
    const ALL: [Self; 3] = [Self::Book, Self::Trades, Self::Markets];

    fn label(self) -> &'static str {
        match self {
            Self::Book => "Order book",
            Self::Trades => "Trades",
            Self::Markets => "Markets",
        }
    }
}

#[composable]
fn NarrowLayout(market: Handle<Market>, height: f32) {
    let tab = rememberMutableStateOf(|| Tab::Book);
    let area = rememberMutableStateOf(|| Size::new(0.0, 0.0));
    let chart_height = (height * 0.4).clamp(220.0, 420.0);
    Column(
        Modifier::empty().fill_max_size(),
        ColumnSpec::default(),
        move || {
            // The renderer draws the frame rate over this strip.
            Spacer(
                Modifier::empty()
                    .fill_max_width()
                    .height(34.0)
                    .background(PANEL),
            );
            StatusLine(market);
            PairHeader(market, false);
            ChartToolbar(market);
            PriceChart(
                market,
                Modifier::empty().fill_max_width().height(chart_height),
            );
            Tabs(tab);
            Box(
                Modifier::empty()
                    .fill_max_width()
                    .weight(1.0)
                    .background(PANEL)
                    .report_size_state(area),
                BoxSpec::default(),
                move || match tab.get() {
                    Tab::Book => OrderBook(market, book_rows(area.get().height)),
                    Tab::Trades => TradeTape(market, Modifier::empty().fill_max_size()),
                    Tab::Markets => Watchlist(market),
                },
            );
        },
    );
}

#[composable]
fn Tabs(tab: MutableState<Tab>) {
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(36.0)
            .background(PANEL),
        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            for item in Tab::ALL {
                let selected = tab.get() == item;
                Box(
                    Modifier::empty()
                        .weight(1.0)
                        .fill_max_height()
                        .clickable(move |_| tab.set(item))
                        .draw_behind(move |draw| {
                            if selected {
                                let size = draw.size();
                                draw.draw_rect_at(
                                    Rect {
                                        x: 0.0,
                                        y: size.height - 2.0,
                                        width: size.width,
                                        height: 2.0,
                                    },
                                    Brush::solid(theme::ACCENT),
                                );
                            }
                        }),
                    BoxSpec::default().content_alignment(Alignment::CENTER),
                    move || {
                        let color = if selected { TEXT } else { MUTED };
                        Text(item.label(), Modifier::empty(), theme::bold(13.0, color));
                    },
                );
            }
        },
    );
}

#[composable]
fn TopBar(market: Handle<Market>) {
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(TOP_BAR_HEIGHT)
            .background(PANEL)
            .padding_horizontal(14.0),
        RowSpec::default()
            .vertical_alignment(VerticalAlignment::CenterVertically)
            .horizontal_arrangement(LinearArrangement::SpacedBy(14.0)),
        move || {
            Logo();
            Text(
                "Cranpose Markets",
                Modifier::empty(),
                theme::bold(14.0, TEXT),
            );
            StatusLine(market);
        },
    );
}

/// Three candles: the app's mark.
#[composable]
fn Logo() {
    Canvas(Modifier::empty().size_points(18.0, 18.0), |draw| {
        let bars = [
            (1.0, 7.0, 9.0, theme::DOWN),
            (7.0, 3.0, 12.0, theme::UP),
            (13.0, 5.0, 8.0, theme::UP),
        ];
        for (x, y, height, color) in bars {
            draw.draw_rect_at(
                Rect {
                    x,
                    y,
                    width: 4.0,
                    height,
                },
                Brush::solid(color),
            );
        }
    });
}

#[composable]
pub fn Panel(modifier: Modifier, content: impl FnMut() + 'static) {
    Box(
        modifier
            .background(PANEL)
            .border(1.0, DIVIDER, RoundedCornerShape::uniform(0.0)),
        BoxSpec::default(),
        content,
    );
}

#[composable]
pub fn Divider() {
    Spacer(
        Modifier::empty()
            .fill_max_width()
            .height(1.0)
            .background(DIVIDER),
    );
}
