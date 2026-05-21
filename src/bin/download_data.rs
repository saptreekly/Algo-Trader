use csv::Writer;
use dotenvy::dotenv;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
struct Bar {
    t: String, // Timestamp
    o: f64,    // Open
    h: f64,    // High
    l: f64,    // Low
    c: f64,    // Close
    v: f64,    // Volume
}

#[derive(Debug, Deserialize)]
struct BarsResponse {
    bars: HashMap<String, Vec<Bar>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY")?;
    let api_secret = env::var("ALPACA_SECRET_KEY")?;

    let symbol = "AAPL";
    let start = "2026-04-15T00:00:00Z";
    let end = "2026-05-15T00:00:00Z";
    let url = format!(
        "https://data.alpaca.markets/v2/stocks/bars?symbols={}&timeframe=1Min&start={}&end={}&limit=1000&feed=sip",
        symbol, start, end
    );

    let mut headers = HeaderMap::new();
    headers.insert("APCA-API-KEY-ID", HeaderValue::from_str(&api_key)?);
    headers.insert("APCA-API-SECRET-KEY", HeaderValue::from_str(&api_secret)?);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await?
        .json::<BarsResponse>()
        .await?;

    if let Some(bars) = response.bars.get(symbol) {
        fs::create_dir_all("data")?;
        let file_path = "data/historical_bars.csv";
        let mut wtr = Writer::from_path(file_path)?;

        for bar in bars {
            wtr.serialize(bar)?;
        }
        wtr.flush()?;
        println!("Successfully saved {} bars to {}", bars.len(), file_path);
    } else {
        println!("No data found for symbol {}", symbol);
    }

    Ok(())
}
