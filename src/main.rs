use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::json;

#[derive(Deserialize, Debug, Clone)]
struct Trade {
    p: f64,
    s: f64,
    t: String,
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
    let mut snapshots: HashMap<String, Snapshot> = HashMap::new();
    let mut last_trade_timestamps: HashMap<String, (String, String)> = HashMap::new();

    for (a, b) in PAIRS {
        let key = format!("{}_{}", a, b);
        let engine = match key.as_str() {
            "AAPL_MSFT" => AdaptiveEngine::with_parameters(0.0001, 0.30, 1.00, 4000.00, 0.1, 0.99),
            "NVDA_AMD" => AdaptiveEngine::with_parameters(0.0001, 0.60, 1.00, 4000.00, 0.1, 0.99),
            "MSFT_NVDA" => AdaptiveEngine::with_parameters(0.0001, 0.30, 1.00, 1500.00, 0.1, 0.99),
            _ => AdaptiveEngine::with_parameters(0.0001, 0.30, 1.00, 500.0, 0.1, 0.99),
        };
        registry.insert(key.clone(), engine);
        states.insert(key.clone(), PositionState::Flat);
        last_trade_timestamps.insert(key, ("".to_string(), "".to_string()));
    }

    let url = "wss://stream.data.alpaca.markets/v2/iex";
    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect to Alpaca WebSocket");

    ws_stream.send(Message::Text(json!({
        "action": "auth", "key": api_key, "secret": secret_key
    }).to_string().into())).await?;

    let mut unique_symbols = std::collections::HashSet::new();
    for &(asset_a, asset_b) in PAIRS {
        unique_symbols.insert(asset_a);
        unique_symbols.insert(asset_b);
    }
    unique_symbols.insert("QQQ");
    unique_symbols.insert("SPY");

    ws_stream.send(Message::Text(json!({
        "action": "subscribe", "trades": unique_symbols
    }).to_string().into())).await?;

    let mut active_portfolio_balance = 2500.0;
    let mut tick_counter: u64 = 0;

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let data: Vec<serde_json::Value> = serde_json::from_str(&text)?;
            for event in data {
                if event["t"] == "t" {
                    let symbol = event["S"].as_str().unwrap().to_string();
                    snapshots.insert(symbol.clone(), Snapshot {
                        latest_trade: Trade {
                            p: event["p"].as_f64().unwrap(),
                            s: event["s"].as_f64().unwrap(),
                            t: event["t"].as_str().unwrap().to_string(),
                        }
                    });
                }
            }
        }
        
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
                let last_ts = last_trade_timestamps.get_mut(&pair_key).unwrap();

                let current_t_a = &snap_a.latest_trade.t;
                let current_t_b = &snap_b.latest_trade.t;

                let (trades_a, vol_a) = if current_t_a == &last_ts.0 { (0, 0.0) } else { last_ts.0 = current_t_a.clone(); (1, snap_a.latest_trade.s) };
                let (trades_b, vol_b) = if current_t_b == &last_ts.1 { (0, 0.0) } else { last_ts.1 = current_t_b.clone(); (1, snap_b.latest_trade.s) };

                let raw_price_a = snap_a.latest_trade.p;
                let raw_price_b = snap_b.latest_trade.p;
                let current_spread = raw_price_a - raw_price_b;
                
                let mut short_leg_value = 0.0;
                match *state {
                    PositionState::LongSpread { entry_size, .. } => { short_leg_value = raw_price_b * entry_size.max(100.0); }
                    PositionState::ShortSpread { entry_size, .. } => { short_leg_value = raw_price_a * entry_size.max(100.0); }
                    _ => {}
                }
                
                if short_leg_value > 0.0 { active_portfolio_balance -= short_leg_value * (ANNUAL_BORROW_RATE / 3_931_200.0); }

                let action = registry.get_mut(&pair_key).unwrap().on_tick(
                    raw_price_a, raw_price_b, vol_a, vol_b, trades_a, trades_b, available_free_cash,
                    match *state { PositionState::Flat => 0, PositionState::LongSpread { .. } => 1, PositionState::ShortSpread { .. } => -1 }
                );

                if tick_counter % 4 == 0 { println!("[Live] Pair {} | Bal: {:.2}", pair_key, active_portfolio_balance); }

                match action.signal {
                    Signal::Buy => {
                        if let PositionState::Flat = *state {
                            if action.size > 0.0 {
                                active_portfolio_balance -= action.execution_slippage * (raw_price_a + raw_price_b) * action.size;
                                *state = PositionState::LongSpread { entry_spread: current_spread, entry_size: action.size, locked_margin: 0.5 * ((action.size * raw_price_a) + (action.size * raw_price_b)) };
                            }
                        }
                    }
                    Signal::Sell => {
                        if let PositionState::Flat = *state {
                            if action.size > 0.0 {
                                active_portfolio_balance -= action.execution_slippage * (raw_price_a + raw_price_b) * action.size;
                                *state = PositionState::ShortSpread { entry_spread: current_spread, entry_size: action.size, locked_margin: 0.5 * ((action.size * raw_price_a) + (action.size * raw_price_b)) };
                            }
                        }
                    }
                    Signal::Close => {
                        match *state {
                            PositionState::LongSpread { entry_spread, entry_size, .. } => {
                                let pnl = (current_spread - entry_spread) * entry_size;
                                let slippage_cost = action.execution_slippage * (raw_price_a + raw_price_b) * entry_size;
                                active_portfolio_balance += pnl - slippage_cost;
                                println!("CLOSING LONG SPREAD {} | PnL: {:.2} | Slippage: {:.2} | Balance: {:.2}", pair_key, pnl, slippage_cost, active_portfolio_balance);
                                *state = PositionState::Flat;
                            }
                            PositionState::ShortSpread { entry_spread, entry_size, .. } => {
                                let pnl = (entry_spread - current_spread) * entry_size;
                                let slippage_cost = action.execution_slippage * (raw_price_a + raw_price_b) * entry_size;
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
    }
    Ok(())
}
