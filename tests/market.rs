use cranpose_markets::format;
use cranpose_markets::market::{Candle, CandleSeries, Trade, TradeTape};

fn candle(open_time_ms: u64, close: f64) -> Candle {
    Candle {
        open_time_ms,
        open: close,
        high: close,
        low: close,
        close,
        volume: 1.0,
    }
}

fn trade(id: u64) -> Trade {
    Trade {
        id,
        price: 1.0,
        quantity: 1.0,
        time_ms: id,
        buyer_maker: false,
    }
}

#[test]
fn a_streamed_candle_survives_history_that_loads_after_it() {
    let mut series = CandleSeries::default();
    series.apply(candle(180_000, 9.0));
    series.replace_history([
        candle(60_000, 1.0),
        candle(120_000, 2.0),
        candle(180_000, 3.0),
    ]);
    let closes: Vec<f64> = series.candles().iter().map(|c| c.close).collect();
    assert_eq!(closes, [1.0, 2.0, 9.0]);
}

#[test]
fn a_candle_updates_its_slot_follows_the_newest_or_is_dropped() {
    let mut series = CandleSeries::default();
    series.replace_history([candle(60_000, 1.0), candle(120_000, 2.0)]);
    series.apply(candle(120_000, 5.0));
    series.apply(candle(180_000, 6.0));
    series.apply(candle(60_000, 7.0));
    let closes: Vec<f64> = series.candles().iter().map(|c| c.close).collect();
    assert_eq!(closes, [1.0, 5.0, 6.0]);
}

#[test]
fn the_series_keeps_its_newest_candles() {
    let mut series = CandleSeries::default();
    let count = CandleSeries::CAPACITY as u64 + 10;
    series.replace_history((0..count).map(|index| candle(index * 60_000, index as f64)));
    assert_eq!(series.candles().len(), CandleSeries::CAPACITY);
    assert_eq!(series.candles()[0].close, 10.0);
}

#[test]
fn the_tape_skips_repeated_trades_and_keeps_the_newest() {
    let mut tape = TradeTape::default();
    for id in 1..=(TradeTape::CAPACITY as u64 + 5) {
        tape.push(trade(id));
    }
    tape.push(trade(3));
    let ids: Vec<u64> = tape.trades().map(|t| t.id).collect();
    assert_eq!(ids.len(), TradeTape::CAPACITY);
    assert_eq!(ids[0], TradeTape::CAPACITY as u64 + 5);
    assert_eq!(*ids.last().unwrap_or(&0), 6);
}

#[test]
fn numbers_group_thousands_and_keep_their_sign() {
    assert_eq!(format::grouped(83_610.014, 2), "83,610.01");
    assert_eq!(format::grouped(-1_234_567.0, 0), "-1,234,567");
    assert_eq!(format::grouped(-0.001, 2), "0.00");
    assert_eq!(format::grouped(999.999, 2), "1,000.00");
    assert_eq!(format::compact(1_267_087_474.0), "1.27B");
    assert_eq!(format::percent(-0.456), "-0.46%");
}

#[test]
fn times_shift_into_the_local_day() {
    // 2026-10-13 23:30:05 UTC.
    let time_ms = 1_791_934_205_000;
    assert_eq!(format::clock(time_ms, 0, true), "23:30:05");
    assert_eq!(format::clock(time_ms, 120, false), "01:30");
    assert_eq!(format::day(time_ms, 0), "Oct 13");
    assert_eq!(format::day(time_ms, 120), "Oct 14");
}
