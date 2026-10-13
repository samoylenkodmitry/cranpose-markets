use cranpose::prelude::*;
use cranpose_coroflow::Handle;
use cranpose_ui_graphics::{DrawTextStyle, TextAlign, TextVerticalAlign};

use super::theme::{self, ACCENT, DIVIDER, FAINT, MUTED, PANEL, RAISED, TEXT};
use crate::format;
use crate::market::{Candle, Interval};
use crate::store::Market;

const AXIS_WIDTH: f32 = 76.0;
const TIME_AXIS_HEIGHT: f32 = 22.0;
const TOP_GAP: f32 = 26.0;
const VOLUME_SHARE: f32 = 0.18;
const MIN_CANDLE_STEP: f32 = 2.0;
const MAX_CANDLE_STEP: f32 = 40.0;

/// How the chart frames the series: the width a candle takes and how many
/// candles the newest one sits left of the right edge.
#[derive(Clone, Copy, PartialEq)]
struct ChartView {
    step: f32,
    scroll: f32,
}

impl Default for ChartView {
    fn default() -> Self {
        Self {
            step: 8.0,
            scroll: 0.0,
        }
    }
}

/// The interval chips above the chart.
#[composable]
pub fn ChartToolbar(market: Handle<Market>) {
    let state = market.get();
    let venue = state.venue.get();
    let selected = state.interval.get();
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(36.0)
            .padding_horizontal(12.0),
        RowSpec::default()
            .vertical_alignment(VerticalAlignment::CenterVertically)
            .horizontal_arrangement(LinearArrangement::SpacedBy(4.0)),
        move || {
            for &interval in venue.intervals() {
                let active = interval == selected;
                Box(
                    Modifier::empty()
                        .height(26.0)
                        .padding_horizontal(10.0)
                        .background(if active { RAISED } else { Color::TRANSPARENT })
                        .rounded_corners(4.0)
                        .clickable(move |_| market.get().select_interval(interval)),
                    BoxSpec::default().content_alignment(Alignment::CENTER),
                    move || {
                        Text(
                            interval.code(),
                            Modifier::empty(),
                            theme::bold(12.0, if active { TEXT } else { MUTED }),
                        );
                    },
                );
            }
        },
    );
}

/// Candles with volume below, a price axis and a crosshair. Drag pans, the
/// wheel zooms. New data only redraws the canvas: nothing recomposes.
#[composable]
pub fn PriceChart(market: Handle<Market>, modifier: Modifier) {
    let view = rememberMutableStateOf(ChartView::default);
    let hover = rememberMutableStateOf(|| None::<Point>);
    let offset = super::local_offset_minutes();
    Canvas(
        modifier
            .background(PANEL)
            .clip_to_bounds()
            .pointer_input((), move |scope| async move {
                scope
                    .await_pointer_event_scope(|events| async move {
                        let mut drag: Option<(f32, f32)> = None;
                        loop {
                            let event = events.await_pointer_event().await;
                            match event.kind {
                                PointerEventKind::Down => {
                                    drag = Some((event.position.x, view.get().scroll));
                                    hover.set(Some(event.position));
                                }
                                PointerEventKind::Move => {
                                    hover.set(Some(event.position));
                                    if let Some((start_x, start_scroll)) = drag {
                                        let current = view.get();
                                        let scroll = start_scroll
                                            + (event.position.x - start_x) / current.step;
                                        view.set(ChartView {
                                            scroll: scroll.max(0.0),
                                            ..current
                                        });
                                        event.consume();
                                    }
                                }
                                PointerEventKind::Up | PointerEventKind::Cancel => drag = None,
                                PointerEventKind::Exit => hover.set(None),
                                PointerEventKind::Scroll => {
                                    zoom(view, (-event.scroll_delta.y * 0.002).exp());
                                    event.consume();
                                }
                                PointerEventKind::Zoom => {
                                    zoom(view, event.zoom_delta);
                                    event.consume();
                                }
                                _ => {}
                            }
                        }
                    })
                    .await;
            }),
        move |draw| {
            let state = market.get();
            let interval = state.interval.get();
            let decimals = state.decimals.get();
            state.candles.read(|series| {
                Plot {
                    candles: series.candles(),
                    view: view.get(),
                    hover: hover.get(),
                    decimals,
                    interval,
                    offset_minutes: offset,
                }
                .draw(draw);
            });
        },
    );
}

fn zoom(view: MutableState<ChartView>, factor: f32) {
    if !factor.is_finite() || factor <= 0.0 {
        return;
    }
    let current = view.get();
    view.set(ChartView {
        step: (current.step * factor).clamp(MIN_CANDLE_STEP, MAX_CANDLE_STEP),
        ..current
    });
}

struct Plot<'a> {
    candles: &'a [Candle],
    view: ChartView,
    hover: Option<Point>,
    decimals: usize,
    interval: Interval,
    offset_minutes: i64,
}

/// Where the visible candles sit and how prices map to heights.
struct Frame {
    left: f32,
    right: f32,
    top: f32,
    price_bottom: f32,
    volume_top: f32,
    volume_bottom: f32,
    step: f32,
    /// The x of the first visible candle's center.
    first_x: f32,
    low: f64,
    high: f64,
    max_volume: f64,
}

impl Frame {
    fn y(&self, price: f64) -> f32 {
        let span = (self.high - self.low).max(f64::EPSILON);
        self.top + ((self.high - price) / span) as f32 * (self.price_bottom - self.top)
    }

    fn price_at(&self, y: f32) -> f64 {
        let span = self.high - self.low;
        self.high - f64::from((y - self.top) / (self.price_bottom - self.top)) * span
    }

    fn x(&self, index: usize) -> f32 {
        self.first_x + index as f32 * self.step
    }
}

fn label_style(align: TextAlign) -> DrawTextStyle {
    DrawTextStyle {
        align,
        vertical_align: TextVerticalAlign::Center,
        ..DrawTextStyle::new(11.0)
    }
}

impl Plot<'_> {
    fn draw(&self, draw: &mut dyn DrawScope) {
        let size = draw.size();
        let right = size.width - AXIS_WIDTH;
        let bottom = size.height - TIME_AXIS_HEIGHT;
        if right <= 0.0 || bottom <= TOP_GAP {
            return;
        }
        if self.candles.is_empty() {
            draw.draw_text(
                Brush::solid(MUTED),
                "Loading candles",
                &label_style(TextAlign::Center),
            );
            return;
        }
        let step = self.view.step;
        let capacity = ((right / step).floor() as usize).max(1);
        let max_scroll = self.candles.len().saturating_sub(capacity) as f32;
        let scroll = self.view.scroll.clamp(0.0, max_scroll);
        let end = self.candles.len() - scroll.floor() as usize;
        let start = end.saturating_sub(capacity + 1);
        let visible = &self.candles[start..end];
        let (low, high, max_volume) = visible.iter().fold(
            (f64::MAX, f64::MIN, 0.0_f64),
            |(low, high, volume), candle| {
                (
                    low.min(candle.low),
                    high.max(candle.high),
                    volume.max(candle.volume),
                )
            },
        );
        let padding = (high - low).max(high.abs() * 1e-4) * 0.08;
        let volume_top = bottom - (bottom - TOP_GAP) * VOLUME_SHARE;
        let frame = Frame {
            left: 0.0,
            right,
            top: TOP_GAP,
            price_bottom: volume_top - 6.0,
            volume_top,
            volume_bottom: bottom,
            step,
            first_x: right - (visible.len() as f32 - 0.5) * step + scroll.fract() * step,
            low: low - padding,
            high: high + padding,
            max_volume,
        };
        self.draw_grid(draw, &frame, size);
        self.draw_time_axis(draw, &frame, visible, size);
        self.draw_candles(draw, &frame, visible);
        self.draw_last_price(draw, &frame);
        self.draw_crosshair(draw, &frame, visible, size);
    }

    fn draw_grid(&self, draw: &mut dyn DrawScope, frame: &Frame, size: Size) {
        let line = Brush::solid(DIVIDER);
        let tick = nice_step((frame.high - frame.low) / 6.0);
        let mut price = (frame.low / tick).ceil() * tick;
        let style = label_style(TextAlign::Right);
        while price <= frame.high {
            let y = frame.y(price);
            draw.draw_rect_at(
                Rect {
                    x: frame.left,
                    y,
                    width: frame.right - frame.left,
                    height: 1.0,
                },
                line.clone(),
            );
            draw.draw_text_at(
                Rect {
                    x: frame.right,
                    y: y - 8.0,
                    width: AXIS_WIDTH - 8.0,
                    height: 16.0,
                },
                Brush::solid(MUTED),
                &format::grouped(price, self.decimals),
                &style,
            );
            price += tick;
        }
        draw.draw_rect_at(
            Rect {
                x: frame.right,
                y: 0.0,
                width: 1.0,
                height: size.height,
            },
            line.clone(),
        );
        draw.draw_rect_at(
            Rect {
                x: 0.0,
                y: frame.volume_top - 3.0,
                width: frame.right,
                height: 1.0,
            },
            line,
        );
    }

    fn draw_time_axis(
        &self,
        draw: &mut dyn DrawScope,
        frame: &Frame,
        visible: &[Candle],
        size: Size,
    ) {
        let every = ((90.0 / frame.step).ceil() as usize).max(1);
        let style = label_style(TextAlign::Center);
        let daily = self.interval.millis() >= Interval::H4.millis();
        for (index, candle) in visible.iter().enumerate() {
            let slot = (candle.open_time_ms / self.interval.millis()) as usize;
            if !slot.is_multiple_of(every) {
                continue;
            }
            let x = frame.x(index);
            if x < 24.0 || x > frame.right - 24.0 {
                continue;
            }
            let label = if daily {
                format::day(candle.open_time_ms, self.offset_minutes)
            } else {
                format::clock(
                    candle.open_time_ms,
                    self.offset_minutes,
                    self.interval == Interval::S1,
                )
            };
            draw.draw_text_at(
                Rect {
                    x: x - 40.0,
                    y: size.height - TIME_AXIS_HEIGHT,
                    width: 80.0,
                    height: TIME_AXIS_HEIGHT,
                },
                Brush::solid(FAINT),
                &label,
                &style,
            );
        }
    }

    fn draw_candles(&self, draw: &mut dyn DrawScope, frame: &Frame, visible: &[Candle]) {
        let body = (frame.step * 0.7).max(1.0);
        let wick = if frame.step >= 4.0 { 1.0 } else { 0.5 };
        let volume_height = frame.volume_bottom - frame.volume_top;
        for (index, candle) in visible.iter().enumerate() {
            let x = frame.x(index);
            if x + body < frame.left || x - body > frame.right {
                continue;
            }
            let up = candle.close >= candle.open;
            let color = theme::trend(up);
            let open = frame.y(candle.open);
            let close = frame.y(candle.close);
            draw.draw_rect_at(
                Rect {
                    x: x - wick / 2.0,
                    y: frame.y(candle.high),
                    width: wick,
                    height: (frame.y(candle.low) - frame.y(candle.high)).max(1.0),
                },
                Brush::solid(color),
            );
            draw.draw_rect_at(
                Rect {
                    x: x - body / 2.0,
                    y: open.min(close),
                    width: body,
                    height: (open - close).abs().max(1.0),
                },
                Brush::solid(color),
            );
            if frame.max_volume > 0.0 {
                let height = (candle.volume / frame.max_volume) as f32 * volume_height;
                draw.draw_rect_at(
                    Rect {
                        x: x - body / 2.0,
                        y: frame.volume_bottom - height,
                        width: body,
                        height,
                    },
                    Brush::solid(color.with_alpha(0.35)),
                );
            }
        }
    }

    fn draw_last_price(&self, draw: &mut dyn DrawScope, frame: &Frame) {
        let Some(last) = self.candles.last() else {
            return;
        };
        let y = frame.y(last.close);
        if y < frame.top || y > frame.price_bottom {
            return;
        }
        let color = theme::trend(last.close >= last.open);
        let mut x = 0.0;
        while x < frame.right {
            draw.draw_rect_at(
                Rect {
                    x,
                    y,
                    width: 4.0,
                    height: 1.0,
                },
                Brush::solid(color.with_alpha(0.7)),
            );
            x += 8.0;
        }
        self.axis_tag(draw, frame, y, last.close, color);
    }

    fn axis_tag(&self, draw: &mut dyn DrawScope, frame: &Frame, y: f32, price: f64, color: Color) {
        let tag = Rect {
            x: frame.right + 1.0,
            y: y - 9.0,
            width: AXIS_WIDTH - 2.0,
            height: 18.0,
        };
        draw.draw_rect_at(tag, Brush::solid(color));
        draw.draw_text_at(
            Rect {
                width: tag.width - 6.0,
                ..tag
            },
            Brush::solid(Color::WHITE),
            &format::grouped(price, self.decimals),
            &label_style(TextAlign::Right),
        );
    }

    fn draw_crosshair(
        &self,
        draw: &mut dyn DrawScope,
        frame: &Frame,
        visible: &[Candle],
        size: Size,
    ) {
        let hovered = self.hover.filter(|point| {
            point.x >= frame.left && point.x < frame.right && point.y < size.height
        });
        let candle = match hovered {
            Some(point) => {
                let index = ((point.x - frame.first_x) / frame.step + 0.5).floor();
                let index = (index.max(0.0) as usize).min(visible.len() - 1);
                let x = frame.x(index);
                let guide = Brush::solid(TEXT.with_alpha(0.35));
                draw.draw_rect_at(
                    Rect {
                        x,
                        y: 0.0,
                        width: 1.0,
                        height: frame.volume_bottom,
                    },
                    guide.clone(),
                );
                if point.y >= frame.top && point.y <= frame.price_bottom {
                    draw.draw_rect_at(
                        Rect {
                            x: 0.0,
                            y: point.y,
                            width: frame.right,
                            height: 1.0,
                        },
                        guide,
                    );
                    self.axis_tag(draw, frame, point.y, frame.price_at(point.y), ACCENT);
                }
                visible[index]
            }
            None => match visible.last() {
                Some(candle) => *candle,
                None => return,
            },
        };
        let readout = format!(
            "O {}  H {}  L {}  C {}  V {}",
            format::grouped(candle.open, self.decimals),
            format::grouped(candle.high, self.decimals),
            format::grouped(candle.low, self.decimals),
            format::grouped(candle.close, self.decimals),
            format::quantity(candle.volume),
        );
        draw.draw_text_from(
            Point::new(10.0, 6.0),
            Brush::solid(theme::trend(candle.close >= candle.open)),
            &readout,
            &DrawTextStyle::new(12.0),
        );
    }
}

/// A round step near `raw`: 1, 2 or 5 times a power of ten.
fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let magnitude = 10f64.powf(raw.log10().floor());
    let fraction = raw / magnitude;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * magnitude
}
