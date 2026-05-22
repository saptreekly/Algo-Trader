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
pub enum PositionState {
    Flat,
    LongSpread { entry_spread: f64, entry_size: f64, locked_margin: f64 },
    ShortSpread { entry_spread: f64, entry_size: f64, locked_margin: f64 },
}

const PAIRS: &[(&str, &str)] = &[
    ("AAPL", "MSFT"), ("NVDA", "AMD"), ("MSFT", "NVDA"),
    ("AAPL", "META"), ("MSFT", "META")
];

const ANNUAL_BORROW_RATE: f64 = 0.010;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY").expect("ALPACA_API_KEY must be set");
    let secret_key = env::var("ALPACA_SECRET_KEY").expect("ALPACA_SECRET_KEY must be set");

    let mut registry: HashMap<String, AdaptiveEngine> = HashMap::new();
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
    let mut active_portfolio_balance = 2500.0;

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

        let mut total_locked_margin = 0.0;
        for state in states.values() {
            match *state {
                PositionState::LongSpread { locked_margin, .. } => total_locked_margin += locked_margin,
                PositionState::ShortSpread { locked_margin, .. } => total_locked_margin += locked_margin,
                PositionState::Flat => {}
            }
        }
        let available_free_cash = (active_portfolio_balance - total_locked_margin).max(0.0);

        for (a, b) in PAIRS {
            let pair_key = format!("{}_{}", a, b);
            
            if let (Some(snap_a), Some(snap_b)) = (snapshots.get(*a), snapshots.get(*b)) {
                let state = states.get_mut(&pair_key).unwrap();

                let raw_price_a = snap_a.latest_trade.p;
                let raw_price_b = snap_b.latest_trade.p;
                let current_spread = raw_price_a - raw_price_b;

                // Borrow cost calculation
                let mut short_leg_value = 0.0;
                match *state {
                    PositionState::LongSpread { entry_size, .. } => {
                        let fee_shares = entry_size.max(100.0);
                        short_leg_value = raw_price_b * fee_shares;
                    }
                    PositionState::ShortSpread { entry_size, .. } => {
                        let fee_shares = entry_size.max(100.0);
                        short_leg_value = raw_price_a * fee_shares;
                    }
                    _ => {}
                }
                
                if short_leg_value > 0.0 {
                    let tick_borrow_cost = short_leg_value * (ANNUAL_BORROW_RATE / 3_931_200.0);
                    active_portfolio_balance -= tick_borrow_cost;
                }

                let engine = registry.get_mut(&pair_key).unwrap();
                let current_pos_i8 = match *state {
                    PositionState::Flat => 0,
                    PositionState::LongSpread { .. } => 1,
                    PositionState::ShortSpread { .. } => -1,
                };
                let action = engine.on_tick(
                    raw_price_a,
                    raw_price_b,
                    snap_a.latest_trade.s,
                    snap_b.latest_trade.s,
                    1,
                    1,
                    available_free_cash,
                    current_pos_i8,
                );

                if tick_counter % 4 == 0 {
                    println!("[Live Summary] Pair {} | State: {:?} | Signal: {:?} | Balance: {:.2}", pair_key, state, action.signal, active_portfolio_balance);
                }

                match action.signal {
                    Signal::Buy => {
                        if let PositionState::Flat = *state {
                            if action.size > 0.0 {
                                let total_slippage_cost = action.execution_slippage * (raw_price_a + raw_price_b) * action.size;
                                active_portfolio_balance -= total_slippage_cost;
                                
                                let initial_margin_rate = 0.50;
                                let margin_requirement = initial_margin_rate * ((action.size * raw_price_a) + (action.size * raw_price_b));
                                
                                println!("OPENING LONG SPREAD {} | SIGNAL: {:?}", pair_key, action.signal);
                                *state = PositionState::LongSpread { entry_spread: current_spread, entry_size: action.size, locked_margin: margin_requirement };
                            }
                        }
                    }
                    Signal::Sell => {
                        if let PositionState::Flat = *state {
                            if action.size > 0.0 {
                                let total_slippage_cost = action.execution_slippage * (raw_price_a + raw_price_b) * action.size;
                                active_portfolio_balance -= total_slippage_cost;
                                
                                let initial_margin_rate = 0.50;
                                let margin_requirement = initial_margin_rate * ((action.size * raw_price_a) + (action.size * raw_price_b));
                                
                                println!("OPENING SHORT SPREAD {} | SIGNAL: {:?}", pair_key, action.signal);
                                *state = PositionState::ShortSpread { entry_spread: current_spread, entry_size: action.size, locked_margin: margin_requirement };
                            }
                        }
                    }
                    Signal::Close => {
                        match *state {
                            PositionState::LongSpread { entry_spread, .. } => {
                                let pnl = (current_spread - entry_spread) * action.size;
                                let slippage_cost = action.execution_slippage * (raw_price_a + raw_price_b) * action.size;
                                active_portfolio_balance += pnl - slippage_cost;
                                println!("CLOSING LONG SPREAD {} | PnL: {:.2} | Slippage: {:.2} | Balance: {:.2}", pair_key, pnl, slippage_cost, active_portfolio_balance);
                                *state = PositionState::Flat;
                            }
                            PositionState::ShortSpread { entry_spread, .. } => {
                                let pnl = (entry_spread - current_spread) * action.size;
                                let slippage_cost = action.execution_slippage * (raw_price_a + raw_price_b) * action.size;
                                active_portfolio_balance += pnl - slippage_cost;
                                println!("CLOSING SHORT SPREAD {} | PnL: {:.2} | Slippage: {:.2} | Balance: {:.2}", pair_key, pnl, slippage_cost, active_portfolio_balance);
                                *state = PositionState::Flat;
                            }
                            _ => {}
                        }
                    }
                    Signal::Hold => {}
                }
            }
        }

        sleep(Duration::from_millis(250)).await;
    }
}
