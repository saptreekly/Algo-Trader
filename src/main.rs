use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize, Debug, Clone)]
struct Trade {
    #[serde(alias = "p")]
    p: f64, // Price
    #[serde(alias = "s")]
    s: f64, // Size (Volume)
}

#[derive(Deserialize, Debug, Clone)]
struct Snapshot {
    #[serde(alias = "latestTrade", alias = "latest_trade")]
    latest_trade: Trade,
}

#[derive(Deserialize, Debug)]
struct SnapshotResponse {
    snapshots: HashMap<String, Snapshot>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY").expect("ALPACA_API_KEY must be set");
    let secret_key = env::var("ALPACA_SECRET_KEY").expect("ALPACA_SECRET_KEY must be set");
    let _base_url = env::var("ALPACA_BASE_URL")
        .unwrap_or_else(|_| "https://paper-api.alpaca.markets".to_string());

    // Optimized parameters
    let mut engine = AdaptiveEngine::with_parameters(0.000002, 0.000002, 0.0001, 0.50, 1.00, 4000.00);

    let client = reqwest::Client::new();
    let url = "https://data.alpaca.markets/v2/stocks/snapshots?symbols=AAPL,MSFT&feed=iex";

    println!("Starting high-frequency trading loop...");

    let mut position_active = false;
    let mut initial_price_a: Option<f64> = None;
    let mut initial_price_b: Option<f64> = None;

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
            sleep(Duration::from_millis(250)).await;
            continue;
        }

        let body = response.text().await?;
        let snap_response: SnapshotResponse = match serde_json::from_str(&body) {
            Ok(r) => r,
            Err(e) => {
                println!("Deserialization Failed: {}, Body: {}", e, body);
                sleep(Duration::from_millis(250)).await;
                continue;
            }
        };

        if let (Some(snap_a), Some(snap_b)) = (snap_response.snapshots.get("AAPL"), snap_response.snapshots.get("MSFT")) {
            if initial_price_a.is_none() || initial_price_b.is_none() {
                initial_price_a = Some(snap_a.latest_trade.p);
                initial_price_b = Some(snap_b.latest_trade.p);
                println!("Baseline prices set: AAPL={:.2}, MSFT={:.2}", snap_a.latest_trade.p, snap_b.latest_trade.p);
                sleep(Duration::from_millis(250)).await;
                continue;
            }

            let norm_price_a = snap_a.latest_trade.p / initial_price_a.unwrap();
            let norm_price_b = snap_b.latest_trade.p / initial_price_b.unwrap();

            let signal = engine.on_tick(
                norm_price_a,
                norm_price_b,
                snap_a.latest_trade.s,
                snap_b.latest_trade.s,
                1, // Discrete tick
                1, // Discrete tick
            );

            match signal {
                Signal::Buy => {
                    if !position_active {
                        println!("LIVE SIGNAL DETECTED: BUY SPREAD - OPENING ORDER FLIGHT");
                        // Order execution logic ...
                        position_active = true;
                    }
                }
                Signal::Sell => {
                    if position_active {
                        println!("LIVE SIGNAL DETECTED: SELL SPREAD - OPENING ORDER FLIGHT");
                        // Order execution logic ...
                        position_active = false;
                    }
                }
                Signal::Hold => {}
            }
        }

        sleep(Duration::from_millis(250)).await;
    }
}
