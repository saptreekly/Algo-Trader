use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use algo_trader::execution::submit_order;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize, Debug, Clone)]
struct Bar {
    c: f64, // Close
    v: f64, // Volume
    n: u64, // Trade count
}

#[derive(Deserialize, Debug)]
struct LatestBarsResponse {
    bars: HashMap<String, Bar>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY").expect("ALPACA_API_KEY must be set");
    let secret_key = env::var("ALPACA_SECRET_KEY").expect("ALPACA_SECRET_KEY must be set");
    let base_url = env::var("ALPACA_BASE_URL")
        .unwrap_or_else(|_| "https://paper-api.alpaca.markets".to_string());

    // Optimized parameters
    let mut engine = AdaptiveEngine::with_parameters(0.000002, 0.000002, 0.0001, 0.50, 1.00, 4000.00);

    let client = reqwest::Client::new();
    let url = "https://data.alpaca.markets/v2/stocks/bars/latest?symbols=AAPL,MSFT&feed=iex";

    println!("Starting live trading loop...");

    let mut position_active = false;

    loop {
        let response = client
            .get(url)
            .header("APCA-API-KEY-ID", &api_key)
            .header("APCA-API-SECRET-KEY", &secret_key)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            println!("API Request Failed: Status {}, Body: {}", status, error_text);
            continue; // Skip this iteration
        }

        let bars_response: LatestBarsResponse = response.json().await?;

        if let (Some(bar_a), Some(bar_b)) = (bars_response.bars.get("AAPL"), bars_response.bars.get("MSFT")) {
            let signal = engine.on_tick(
                bar_a.c,
                bar_b.c,
                bar_a.v,
                bar_b.v,
                bar_a.n,
                bar_b.n,
            );

            match signal {
                Signal::Buy => {
                    if !position_active {
                        println!("LIVE SIGNAL DETECTED: BUY SPREAD - OPENING ORDER FLIGHT");
                        let _ = submit_order("AAPL", 10.0, "buy", &client, &api_key, &secret_key, &base_url).await;
                        let _ = submit_order("MSFT", 15.0, "sell", &client, &api_key, &secret_key, &base_url).await;
                        position_active = true;
                    }
                }
                Signal::Sell => {
                    if position_active {
                        println!("LIVE SIGNAL DETECTED: SELL SPREAD - OPENING ORDER FLIGHT");
                        let _ = submit_order("AAPL", 10.0, "sell", &client, &api_key, &secret_key, &base_url).await;
                        let _ = submit_order("MSFT", 15.0, "buy", &client, &api_key, &secret_key, &base_url).await;
                        position_active = false;
                    }
                }
                Signal::Hold => {}
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}
