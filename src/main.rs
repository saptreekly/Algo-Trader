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
    LongSpread { entry_spread: f64 },
    ShortSpread { entry_spread: f64 },
}

const PAIRS: &[(&str, &str)] = &[
    ("AAPL", "MSFT"), ("NVDA", "AMD"), ("MSFT", "NVDA"),
    ("AAPL", "META"), ("MSFT", "META")
];

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
        let engine = match key.as_str() {
            "AAPL_MSFT" => AdaptiveEngine::with_parameters(0.0001, 0.30, 1.00, 4000.00, 0.1, 0.99),
            "NVDA_AMD" => AdaptiveEngine::with_parameters(0.0001, 0.60, 1.00, 4000.00, 0.1, 0.99),
            "MSFT_NVDA" => AdaptiveEngine::with_parameters(0.0001, 0.30, 1.00, 1500.00, 0.1, 0.99),
            _ => AdaptiveEngine::with_parameters(0.0001, 0.30, 1.00, 500.0, 0.1, 0.99),
        };
        registry.insert(key.clone(), engine);
        states.insert(key, PositionState::Flat);
    }

    let client = reqwest::Client::new();
    let url = "https://data.alpaca.markets/v2/stocks/snapshots?symbols=AAPL,MSFT,NVDA,AMD,QQQ,SPY&feed=iex";

    println!("Starting high-frequency trading loop...");

    let mut tick_counter: u64 = 0;
    let mut active_portfolio_balance = 100.0;

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
                let raw_price_a = snap_a.latest_trade.p;
                let raw_price_b = snap_b.latest_trade.p;
                let current_spread = raw_price_a - raw_price_b;

                let engine = registry.get_mut(&pair_key).unwrap();
                let action = engine.on_tick(
                    norm_price_a,
                    norm_price_b,
                    raw_price_a,
                    raw_price_b,
                    snap_a.latest_trade.s,
                    snap_b.latest_trade.s,
                    1,
                    1,
                    active_portfolio_balance,
                );

                if tick_counter % 4 == 0 {
                    println!("[Live Summary] Pair {} | State: {:?} | Signal: {:?} | Balance: {:.2}", pair_key, state, action.signal, active_portfolio_balance);
                }

                match action.signal {
                    Signal::Buy => {
                        if let PositionState::Flat = *state {
                            if action.size > 0.0 {
                                println!("OPENING LONG SPREAD {} | SIGNAL: {:?}", pair_key, action.signal);
                                *state = PositionState::LongSpread { entry_spread: current_spread };
                            }
                        }
                    }
                    Signal::Sell => {
                        if let PositionState::Flat = *state {
                            if action.size > 0.0 {
                                println!("OPENING SHORT SPREAD {} | SIGNAL: {:?}", pair_key, action.signal);
                                *state = PositionState::ShortSpread { entry_spread: current_spread };
                            }
                        }
                    }
                    Signal::Hold => {
                        match *state {
                            PositionState::LongSpread { entry_spread } => {
                                let pnl = (current_spread - entry_spread) * action.size;
                                let slippage_cost = action.execution_slippage * (raw_price_a + raw_price_b) * action.size;
                                active_portfolio_balance += pnl - slippage_cost;
                                println!("CLOSING LONG SPREAD {} | PnL: {:.2} | Slippage: {:.2} | Balance: {:.2}", pair_key, pnl, slippage_cost, active_portfolio_balance);
                                *state = PositionState::Flat;
                            }
                            PositionState::ShortSpread { entry_spread } => {
                                let pnl = (entry_spread - current_spread) * action.size;
                                let slippage_cost = action.execution_slippage * (raw_price_a + raw_price_b) * action.size;
                                active_portfolio_balance += pnl - slippage_cost;
                                println!("CLOSING SHORT SPREAD {} | PnL: {:.2} | Slippage: {:.2} | Balance: {:.2}", pair_key, pnl, slippage_cost, active_portfolio_balance);
                                *state = PositionState::Flat;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        sleep(Duration::from_millis(250)).await;
    }
}
