//! The screens. [`MarketsApp`] is the root composable.

mod app;
mod book;
mod chart;
mod feeds;
mod header;
mod tape;
mod theme;
mod watchlist;

pub use app::MarketsApp;

/// The browser's offset from UTC in minutes, for the times the app shows.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn local_offset_minutes() -> i64 {
    -(js_sys::Date::new_0().get_timezone_offset() as i64)
}

/// Outside the browser the app shows UTC.
#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
fn local_offset_minutes() -> i64 {
    0
}
