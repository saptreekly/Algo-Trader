use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize, Debug, Clone)]
struct Trade {
    p: f64,
    s: f64,
}

#[derive(Deserialize, Debug, Clone)]
struct Snapshot {
    #[serde(rename = "latestTrade")]
    latest_trade: Trade,
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

        let snapshots: HashMap<String, Snapshot> = response.json().await?;

        if let (Some(snap_a), Some(snap_b)) = (snapshots.get("AAPL"), snapshots.get("MSFT")) {
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
