use cranpose_markets::feed::{StreamEvent, parse_candles, parse_stream, parse_tickers};
use cranpose_markets::market::{Candle, Trade, Venue};

#[test]
fn depth_message_reads_both_sides_best_first() {
    let text = r#"{"stream":"btcusdt@depth20@100ms","data":{"lastUpdateId":1,"bids":[["100.5","2"],["100.4","0.5"]],"asks":[["100.6","1.25"]]}}"#;
    assert_eq!(
        parse_stream(text),
        Some(StreamEvent::Depth {
            bids: vec![(100.5, 2.0), (100.4, 0.5)],
            asks: vec![(100.6, 1.25)],
        })
    );
}

#[test]
fn ticker_array_reads_every_pair() {
    let text = r#"{"stream":"!miniTicker@arr","data":[{"e":"24hrMiniTicker","E":1,"s":"BTCUSDT","c":"110","o":"100","h":"120","l":"90","v":"5","q":"550"},{"e":"24hrMiniTicker","E":1,"s":"ETHUSDT","c":"9","o":"10","h":"11","l":"8","v":"5","q":"45"}]}"#;
    let Some(StreamEvent::Tickers(tickers)) = parse_stream(text) else {
        panic!("a ticker array should decode");
    };
    assert_eq!(tickers.len(), 2);
    assert_eq!(tickers[0].0, "BTCUSDT");
    assert!((tickers[0].1.change_percent() - 10.0).abs() < 1e-9);
    assert!((tickers[1].1.change_percent() + 10.0).abs() < 1e-9);
}

#[test]
fn trade_and_candle_messages_decode() {
    let trade = r#"{"stream":"btcusdt@aggTrade","data":{"e":"aggTrade","E":2,"s":"BTCUSDT","a":42,"p":"101.5","q":"0.25","f":1,"l":2,"T":1700000000123,"m":true,"M":true}}"#;
    assert_eq!(
        parse_stream(trade),
        Some(StreamEvent::Trade(Trade {
            id: 42,
            price: 101.5,
            quantity: 0.25,
            time_ms: 1_700_000_000_123,
            buyer_maker: true,
        }))
    );
    let candle = r#"{"stream":"btcusdt@kline_1m","data":{"e":"kline","E":2,"s":"BTCUSDT","k":{"t":60000,"T":119999,"s":"BTCUSDT","i":"1m","o":"1","c":"2","h":"3","l":"0.5","v":"7","x":false}}}"#;
    assert_eq!(
        parse_stream(candle),
        Some(StreamEvent::Candle(Candle {
            open_time_ms: 60_000,
            open: 1.0,
            high: 3.0,
            low: 0.5,
            close: 2.0,
            volume: 7.0,
        }))
    );
}

#[test]
fn unknown_streams_and_broken_messages_are_skipped() {
    assert_eq!(
        parse_stream(r#"{"stream":"btcusdt@bookTicker","data":{}}"#),
        None
    );
    assert_eq!(parse_stream(r#"{"result":null,"id":1}"#), None);
    assert_eq!(parse_stream("not json"), None);
}

#[test]
fn tickers_keep_the_venues_active_pairs_most_traded_first() {
    let body = r#"[
        {"symbol":"ETHUSDT","lastPrice":"10","openPrice":"9","highPrice":"11","lowPrice":"8","quoteVolume":"500"},
        {"symbol":"BTCUSDT","lastPrice":"100","openPrice":"90","highPrice":"110","lowPrice":"80","quoteVolume":"9000"},
        {"symbol":"USDCUSDT","lastPrice":"1","openPrice":"1","highPrice":"1","lowPrice":"1","quoteVolume":"99999"},
        {"symbol":"OLDUSDT","lastPrice":"1","openPrice":"1","highPrice":"1","lowPrice":"1","quoteVolume":"0"},
        {"symbol":"ETHBTC","lastPrice":"0.1","openPrice":"0.1","highPrice":"0.1","lowPrice":"0.1","quoteVolume":"10"},
        {"symbol":"BTCUSD","lastPrice":"100","openPrice":"90","highPrice":"110","lowPrice":"80","quoteVolume":"700"}
    ]"#;
    let symbols = |venue| {
        parse_tickers(venue, body)
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|pair| format!("{}/{}", pair.base(), pair.quote_asset()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    assert_eq!(symbols(Venue::Binance), ["BTC/USDT", "ETH/USDT"]);
    assert_eq!(
        symbols(Venue::BinanceUs),
        ["BTC/USDT", "BTC/USD", "ETH/USDT"]
    );
}

#[test]
fn rest_candles_read_oldest_first() {
    let body = r#"[[60000,"1","3","0.5","2","7",119999,"14",3,"1","2","0"],[120000,"2","4","1","3","8",179999,"24",4,"1","2","0"]]"#;
    let candles = parse_candles(body).unwrap_or_default();
    assert_eq!(candles.len(), 2);
    assert_eq!(candles[1].open_time_ms, 120_000);
    assert_eq!(candles[1].close, 3.0);
}
