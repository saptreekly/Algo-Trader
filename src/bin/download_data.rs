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

async fn fetch_bars(
    symbol: &str,
    api_key: &str,
    api_secret: &str,
) -> Result<Vec<Bar>, Box<dyn Error>> {
    let mut all_bars = Vec::new();
    let mut next_page_token: Option<String> = None;
    
    // For demonstration, fetch a small recent range if not specified
    let start = "2026-05-20T09:30:00Z";
    let end = "2026-05-22T16:00:00Z";

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("APCA-API-KEY-ID", HeaderValue::from_str(api_key)?);
    headers.insert("APCA-API-SECRET-KEY", HeaderValue::from_str(api_secret)?);

    loop {
        let mut url = format!(
            "https://data.alpaca.markets/v2/stocks/bars?symbols={}&timeframe=1Min&start={}&end={}&limit=1000&feed=iex",
            symbol, start, end
        );
        if let Some(token) = &next_page_token {
            url.push_str(&format!("&page_token={}", token));
        }

        let response = client
            .get(&url)
            .headers(headers.clone())
            .send()
            .await?;
        
        let status = response.status();
        let body_text = response.text().await?;
        
        if !status.is_success() {
            return Err(format!("API error: status {}, body: {}", status, body_text).into());
        }

        let bars_response: BarsResponse = serde_json::from_str(&body_text)
            .map_err(|e| format!("Failed to parse JSON: {}, body: {}", e, body_text))?;

        if let Some(bars) = bars_response.bars.get(symbol) {
            all_bars.extend(bars.clone());
        }

        next_page_token = bars_response.next_page_token;
        if next_page_token.is_none() {
            break;
        }
    }

    Ok(all_bars)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY").expect("ALPACA_API_KEY must be set");
    let api_secret = env::var("ALPACA_SECRET_KEY").expect("ALPACA_SECRET_KEY must be set");

    let assets = vec!["AAPL", "MSFT", "NVDA", "AMD", "GOOGL", "AMZN", "META"];

    fs::create_dir_all("data")?;

    for asset in assets {
        println!("Fetching data for {}", asset);
        let bars = fetch_bars(asset, &api_key, &api_secret).await?;
        
        let file_path = format!("data/{}.csv", asset);
        let mut wtr = Writer::from_path(&file_path)?;
        wtr.write_record(&["timestamp", "close", "vol", "trade_count"])?;

        for bar in bars {
            wtr.write_record(&[
                &bar.t,
                &bar.c.to_string(),
                &bar.v.to_string(),
                &bar.n.to_string(),
            ])?;
        }
        wtr.flush()?;
        println!("Saved data for {} to {}", asset, file_path);
    }

    Ok(())
}
