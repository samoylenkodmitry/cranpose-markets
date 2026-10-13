# Cranpose Markets

**[Open the app](https://samoylenkodmitry.github.io/cranpose-markets/)**

A crypto market screen in the browser, built with
[Cranpose](https://github.com/samoylenkodmitry/Cranpose) in Rust and
WebAssembly. It shows real market data as the exchange sends it: a watchlist of
every pair, the order book, the trade tape and a candle chart.

The frame rate overlay in the top right corner is Cranpose's own counter. It
shows frames per second, the frame interval percentiles and the work time of a
frame. The status line shows how many market data updates the app applied in
the last second.

## Data

The app reads Binance's public market data. It needs no account or API key.

| What | Source | Rate |
|---|---|---|
| Pair list | REST `GET /api/v3/ticker/24hr?type=MINI` | once per venue |
| 24 hour statistics of every pair | stream `!miniTicker@arr` | every second |
| Order book, best 20 levels each side | stream `<pair>@depth20@100ms` | every 100 ms |
| Trades | stream `<pair>@aggTrade` | each trade |
| Candles | REST `GET /api/v3/klines`, then stream `<pair>@kline_<interval>` | each change |

Binance serves its market data from `data-api.binance.vision` and
`data-stream.binance.vision`. Binance refuses visitors from some regions,
including the United States. When it refuses the pair list or the stream, the
app moves to Binance.US, which speaks the same protocol.

## How the screen updates

- Each watchlist row holds its quote in its own state. A ticker update
  recomposes that row alone, and only the rows in view exist.
- Each order book level is a state. A level whose price and quantity stay the
  same does not recompose.
- New trades push rows down the tape. Each row keeps its identity by trade id,
  so only the new row composes.
- The chart is one canvas. New candles, panning, zooming and the crosshair
  redraw it without a recomposition.

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
./build-web.sh
python3 -m http.server --directory dist 8000
```

Open `http://localhost:8000`. Cranpose draws with WebGPU where the browser
offers it and WebGL2 otherwise; add `?backend=gl` or `?backend=webgpu` to the
URL to force one.

## Release

Push a `v*` tag. The [Pages workflow](.github/workflows/pages.yml) builds the
bundle and publishes it to GitHub Pages.

## License

Apache-2.0. Market data belongs to its venues and is subject to their terms.
