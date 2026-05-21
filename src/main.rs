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

#[derive(Debug, PartialEq, Clone, Copy)]
enum PositionState {
    Flat,
    LongSpread,
    ShortSpread,
}

const PAIRS: &[(&str, &str)] = &[("AAPL", "MSFT"), ("NVDA", "AMD"), ("QQQ", "SPY")];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY").expect("ALPACA_API_KEY must be set");
    let secret_key = env::var("ALPACA_SECRET_KEY").expect("ALPACA_SECRET_KEY must be set");

    let mut registry: HashMap<String, AdaptiveEngine> = HashMap::new();
    let mut baselines: HashMap<String, (f64, f64)> = HashMap::new();
    let mut states: HashMap<String, PositionState> = HashMap::new();

    for (a, b) in PAIRS {
        let key = format!("{}_{}", a, b);
        registry.insert(key.clone(), AdaptiveEngine::with_parameters(0.000002, 0.30, 0.0001, 0.50, 1.00, 4000.00));
        states.insert(key, PositionState::Flat);
    }

    let client = reqwest::Client::new();
    let url = "https://data.alpaca.markets/v2/stocks/snapshots?symbols=AAPL,MSFT,NVDA,AMD,QQQ,SPY&feed=iex";

    println!("Starting high-frequency trading loop...");

    let mut tick_counter: u64 = 0;

    loop {
        let response = client
            .get(url)
            .header("APCA-API-KEY-ID", &api_key)
            .header("APCA-API-SECRET-KEY", &secret_key)
            .send()
            .await?;
        
        if !response.status().is_success() {
            println!("API Request Failed: Status {}", response.status());
            sleep(Duration::from_millis(250)).await;
            continue;
        }

        let snapshots: HashMap<String, Snapshot> = response.json().await?;

        tick_counter += 1;

        for (a, b) in PAIRS {
            let pair_key = format!("{}_{}", a, b);
            
            if let (Some(snap_a), Some(snap_b)) = (snapshots.get(*a), snapshots.get(*b)) {
                let baseline = baselines.entry(pair_key.clone()).or_insert((snap_a.latest_trade.p, snap_b.latest_trade.p));
                let state = states.get_mut(&pair_key).unwrap();

                let norm_price_a = snap_a.latest_trade.p / baseline.0;
                let norm_price_b = snap_b.latest_trade.p / baseline.1;

                let engine = registry.get_mut(&pair_key).unwrap();
                let signal = engine.on_tick(
                    norm_price_a,
                    norm_price_b,
                    snap_a.latest_trade.s,
                    snap_b.latest_trade.s,
                    1,
                    1,
                );

                if tick_counter % 4 == 0 {
                    println!("[Live Summary] Pair {} | State: {:?} | Signal: {:?}", pair_key, state, signal);
                }

                match signal {
                    Signal::Buy => {
                        if *state == PositionState::Flat {
                            println!("OPENING LONG SPREAD {} | SIGNAL: {:?}", pair_key, signal);
                            *state = PositionState::LongSpread;
                        }
                    }
                    Signal::Sell => {
                        if *state == PositionState::Flat {
                            println!("OPENING SHORT SPREAD {} | SIGNAL: {:?}", pair_key, signal);
                            *state = PositionState::ShortSpread;
                        }
                    }
                    Signal::Hold => {
                        if *state == PositionState::LongSpread {
                             println!("CLOSING LONG SPREAD {} | SIGNAL: {:?}", pair_key, signal);
                             *state = PositionState::Flat;
                        } else if *state == PositionState::ShortSpread {
                             println!("CLOSING SHORT SPREAD {} | SIGNAL: {:?}", pair_key, signal);
                             *state = PositionState::Flat;
                        }
                    }
                }
            }
        }

        sleep(Duration::from_millis(250)).await;
    }
}
