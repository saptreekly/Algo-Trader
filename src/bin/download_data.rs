use csv::Writer;
use dotenvy::dotenv;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Bar {
    t: String, // Timestamp
    c: f64,    // Close
    v: f64,    // Volume
    n: u64,    // Trade count
}

#[derive(Debug, Deserialize)]
struct BarsResponse {
    bars: HashMap<String, Vec<Bar>>,
    next_page_token: Option<String>,
}

async fn fetch_bars_paginated(
    symbol: &str,
    api_key: &str,
    api_secret: &str,
) -> Result<Vec<Bar>, Box<dyn Error>> {
    let mut all_bars = Vec::new();
    let mut next_page_token: Option<String> = None;
    let start = "2026-03-01T00:00:00Z";
    let end = "2026-05-15T00:00:00Z";

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("APCA-API-KEY-ID", HeaderValue::from_str(api_key)?);
    headers.insert("APCA-API-SECRET-KEY", HeaderValue::from_str(api_secret)?);

    loop {
        let mut url = format!(
            "https://data.alpaca.markets/v2/stocks/bars?symbols={}&timeframe=1Min&start={}&end={}&limit=10000&feed=sip",
            symbol, start, end
        );
        if let Some(token) = &next_page_token {
            url.push_str(&format!("&page_token={}", token));
        }

        let response = client
            .get(&url)
            .headers(headers.clone())
            .send()
            .await?
            .json::<BarsResponse>()
            .await?;

        if let Some(bars) = response.bars.get(symbol) {
            all_bars.extend(bars.clone());
        }

        next_page_token = response.next_page_token;
        if next_page_token.is_none() {
            break;
        }
    }

    Ok(all_bars)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY")?;
    let api_secret = env::var("ALPACA_SECRET_KEY")?;

    let bars_a = fetch_bars_paginated("AAPL", &api_key, &api_secret).await?;
    let bars_b = fetch_bars_paginated("MSFT", &api_key, &api_secret).await?;

    fs::create_dir_all("data")?;
    let file_path = "data/historical_pairs.csv";
    let mut wtr = Writer::from_path(file_path)?;
    wtr.write_record(&[
        "timestamp",
        "close_a",
        "close_b",
        "vol_a",
        "vol_b",
        "trade_count_a",
        "trade_count_b",
    ])?;

    let map_b: HashMap<String, (f64, f64, u64)> = bars_b
        .iter()
        .map(|b| (b.t.clone(), (b.c, b.v, b.n)))
        .collect();

    for bar_a in bars_a {
        if let Some(&(close_b, vol_b, n_b)) = map_b.get(&bar_a.t) {
            wtr.write_record(&[
                &bar_a.t,
                &bar_a.c.to_string(),
                &close_b.to_string(),
                &bar_a.v.to_string(),
                &vol_b.to_string(),
                &bar_a.n.to_string(),
                &n_b.to_string(),
            ])?;
        }
    }
    wtr.flush()?;
    println!("Successfully saved aligned pairs to {}", file_path);

    Ok(())
}
