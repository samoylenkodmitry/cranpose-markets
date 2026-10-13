//! Cranpose Markets: live crypto market data in the browser, drawn with
//! Cranpose.

pub mod feed;
pub mod format;
pub mod market;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
mod socket;
pub mod store;
mod ui;

pub use ui::MarketsApp;

/// The launcher with the frame rate overlay on.
pub fn create_app() -> cranpose::AppLauncher {
    cranpose::AppLauncher::new()
        .with_title("Cranpose Markets")
        .with_fps_counter(true)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
mod web {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn web_init() {
        wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
        console_error_panic_hook::set_once();
    }

    #[wasm_bindgen]
    pub async fn run_app() -> Result<(), JsValue> {
        super::create_app()
            .with_web_fill_viewport(true)
            .run_web("markets-canvas", super::MarketsApp)
            .await
    }
}
