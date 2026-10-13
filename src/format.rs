//! Number and time text for prices, quantities and volumes.

use std::fmt::Write;

/// Decimals that show a price's moves without noise: a pair near 80,000
/// moves in cents, one near 0.00002 in its eighth decimal.
pub fn price_decimals(price: f64) -> usize {
    match price.abs() {
        p if p >= 1_000.0 => 2,
        p if p >= 10.0 => 3,
        p if p >= 1.0 => 4,
        p if p >= 0.01 => 5,
        p if p >= 0.000_1 => 7,
        _ => 8,
    }
}

/// `value` with `decimals` decimals and a comma between thousands.
pub fn grouped(value: f64, decimals: usize) -> String {
    let mut text = String::with_capacity(24);
    write_grouped(&mut text, value, decimals);
    text
}

fn write_grouped(out: &mut String, value: f64, decimals: usize) {
    let plain = format!("{:.*}", decimals, value.abs());
    let (whole, fraction) = plain.split_once('.').unwrap_or((plain.as_str(), ""));
    if value < 0.0 && plain.bytes().any(|b| b.is_ascii_digit() && b != b'0') {
        out.push('-');
    }
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    if !fraction.is_empty() {
        out.push('.');
        out.push_str(fraction);
    }
}

/// A quantity with as many decimals as its size needs.
pub fn quantity(value: f64) -> String {
    let decimals = match value.abs() {
        v if v >= 1_000_000.0 => return compact(value),
        v if v >= 1_000.0 => 0,
        v if v >= 10.0 => 2,
        v if v >= 1.0 => 3,
        _ => 5,
    };
    grouped(value, decimals)
}

/// A large amount with a K, M or B suffix.
pub fn compact(value: f64) -> String {
    let (scaled, suffix) = match value.abs() {
        v if v >= 1e9 => (value / 1e9, "B"),
        v if v >= 1e6 => (value / 1e6, "M"),
        v if v >= 1e3 => (value / 1e3, "K"),
        _ => (value, ""),
    };
    let mut text = String::with_capacity(12);
    let _ = write!(text, "{scaled:.2}{suffix}");
    text
}

/// A change in percent with its sign.
pub fn percent(value: f64) -> String {
    format!("{value:+.2}%")
}

/// The time of day of `time_ms`, shifted by `offset_minutes` from UTC.
pub fn clock(time_ms: u64, offset_minutes: i64, with_seconds: bool) -> String {
    let minutes_per_day = 24 * 60;
    let total_seconds = (time_ms / 1_000) as i64 + offset_minutes * 60;
    let seconds_of_day = total_seconds.rem_euclid(minutes_per_day * 60);
    let (hours, minutes, seconds) = (
        seconds_of_day / 3_600,
        seconds_of_day / 60 % 60,
        seconds_of_day % 60,
    );
    if with_seconds {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}")
    }
}

/// The calendar day of `time_ms`, shifted by `offset_minutes` from UTC, as
/// month and day.
pub fn day(time_ms: u64, offset_minutes: i64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let days = ((time_ms / 1_000) as i64 + offset_minutes * 60).div_euclid(86_400);
    let (_, month, day) = civil_from_days(days);
    format!("{} {day}", MONTHS[(month - 1) as usize])
}

/// The proleptic Gregorian date of a day count from 1970-01-01, after
/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}
