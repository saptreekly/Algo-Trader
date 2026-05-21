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
}

#[derive(Debug, Deserialize)]
struct BarsResponse {
    bars: HashMap<String, Vec<Bar>>,
}

async fn fetch_bars(symbol: &str, api_key: &str, api_secret: &str) -> Result<Vec<Bar>, Box<dyn Error>> {
    let start = "2026-04-15T00:00:00Z";
    let end = "2026-05-15T00:00:00Z";
    let url = format!(
        "https://data.alpaca.markets/v2/stocks/bars?symbols={}&timeframe=1Min&start={}&end={}&limit=1000&feed=sip",
        symbol, start, end
    );

    let mut headers = HeaderMap::new();
    headers.insert("APCA-API-KEY-ID", HeaderValue::from_str(api_key)?);
    headers.insert("APCA-API-SECRET-KEY", HeaderValue::from_str(api_secret)?);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await?
        .json::<BarsResponse>()
        .await?;

    Ok(response.bars.get(symbol).cloned().unwrap_or_default())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY")?;
    let api_secret = env::var("ALPACA_SECRET_KEY")?;

    let bars_a = fetch_bars("AAPL", &api_key, &api_secret).await?;
    let bars_b = fetch_bars("MSFT", &api_key, &api_secret).await?;

    fs::create_dir_all("data")?;
    let file_path = "data/historical_pairs.csv";
    let mut wtr = Writer::from_path(file_path)?;
    wtr.write_record(&["timestamp", "close_a", "close_b"])?;

    // Align by timestamp
    let mut map_b: HashMap<String, f64> = bars_b.iter().map(|b| (b.t.clone(), b.c)).collect();

    for bar_a in bars_a {
        if let Some(close_b) = map_b.remove(&bar_a.t) {
            wtr.write_record(&[&bar_a.t, &bar_a.c.to_string(), &close_b.to_string()])?;
        }
    }
    wtr.flush()?;
    println!("Successfully saved aligned pairs to {}", file_path);

    Ok(())
}
