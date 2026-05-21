use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use csv::Reader;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum PositionState {
    Flat,
    LongSpread,
    ShortSpread,
}

#[derive(Debug, Deserialize, Clone)]
struct Row {
    #[allow(dead_code)]
    timestamp: String,
    close: f64,
    vol: f64,
    trade_count: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = vec!["AAPL", "MSFT", "NVDA", "AMD", "GOOGL", "AMZN", "META"];
    let mut data: HashMap<String, Vec<Row>> = HashMap::new();

    // 2. Load CSVs
    for asset in &assets {
        let path = format!("data/{}.csv", asset);
        let mut rdr = Reader::from_path(path)?;
        let mut rows = Vec::new();
        for result in rdr.deserialize() {
            rows.push(result?);
        }
        data.insert(asset.to_string(), rows);
    }

    // 3. Pair Generation
    let mut pairs = Vec::new();
    for i in 0..assets.len() {
        for j in i + 1..assets.len() {
            pairs.push((assets[i], assets[j]));
        }
    }

    // 4. Instantiate Engines
    let mut engines = HashMap::new();
    let mut states = HashMap::new();
    let mut balances = HashMap::new();
    let mut trade_counts = HashMap::new();

    for (a, b) in &pairs {
        let pair_key = format!("{}_{}", a, b);
        let engine = AdaptiveEngine::with_parameters(0.000002, 0.30, 0.0001, 0.50, 1.00, 4000.00);
        engines.insert(pair_key.clone(), engine);
        states.insert(pair_key.clone(), PositionState::Flat);
        balances.insert(pair_key.clone(), 100000.0 / 21.0); // Initial balance split
        trade_counts.insert(pair_key.clone(), 0);
    }

    // 5. Chronological Loop
    // Determine the minimum length across all assets to safely loop
    let min_len = data.values().map(|v| v.len()).min().unwrap_or(0);
    
    for row_idx in 0..min_len {
        for (a, b) in &pairs {
            let pair_key = format!("{}_{}", a, b);
            let row_a = &data[*a][row_idx];
            let row_b = &data[*b][row_idx];

            let engine = engines.get_mut(&pair_key).unwrap();
            let signal = engine.on_tick(
                row_a.close,
                row_b.close,
                row_a.vol,
                row_b.vol,
                row_a.trade_count,
                row_b.trade_count,
            );

            let state = states.get_mut(&pair_key).unwrap();
            let balance = balances.get_mut(&pair_key).unwrap();
            let trades = trade_counts.get_mut(&pair_key).unwrap();

            match (*state, signal) {
                (PositionState::Flat, Signal::Buy) => {
                    *state = PositionState::LongSpread;
                    *trades += 1;
                    *balance -= 0.02; // Slippage
                }
                (PositionState::Flat, Signal::Sell) => {
                    *state = PositionState::ShortSpread;
                    *trades += 1;
                    *balance -= 0.02; // Slippage
                }
                (PositionState::LongSpread, Signal::Sell) => {
                    *state = PositionState::Flat;
                    *trades += 1;
                    *balance -= 0.02; // Slippage
                    // Simple profit calc based on price diff
                    *balance += row_a.close - row_b.close;
                }
                (PositionState::ShortSpread, Signal::Buy) => {
                    *state = PositionState::Flat;
                    *trades += 1;
                    *balance -= 0.02; // Slippage
                    // Simple profit calc
                    *balance -= row_a.close - row_b.close;
                }
                _ => {}
            }
        }
    }

    // 6. Terminal Summary
    println!("Backtest Summary:");
    let mut total_trades = 0;
    let mut total_balance = 0.0;
    let mut pair_profits = Vec::new();

    for (pair_key, balance) in &balances {
        total_balance += balance;
        total_trades += trade_counts[pair_key];
        pair_profits.push((pair_key, balance - (100000.0 / 21.0)));
    }

    pair_profits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("Total Trades: {}", total_trades);
    println!("Final Portfolio Balance: ${:.2}", total_balance);
    println!("Top Pair: {} Profit: ${:.2}", pair_profits[0].0, pair_profits[0].1);

    Ok(())
}
