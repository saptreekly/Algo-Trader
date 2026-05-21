use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use csv::Reader;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

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
    symbol: String,
    close: f64,
    vol: f64,
    trade_count: u64,
}

#[derive(Debug, Deserialize)]
struct RawRow {
    timestamp: String,
    close: f64,
    vol: f64,
    trade_count: u64,
}

fn run_simulation(
    pairs: &[( &str, &str)],
    timeline: &BTreeMap<String, Vec<Row>>,
    z_threshold: f64,
    initial_vol: f64,
    loss_toxic: f64,
) -> HashMap<String, (f64, u64)> {
    let mut results = HashMap::new();
    let mut baselines: HashMap<String, (f64, f64)> = HashMap::new();
    let mut states: HashMap<String, PositionState> = HashMap::new();
    let mut balances: HashMap<String, f64> = HashMap::new();
    let mut trade_counts: HashMap<String, u64> = HashMap::new();
    let mut registry: HashMap<String, AdaptiveEngine> = HashMap::new();
    let mut latest_prices: HashMap<String, (f64, f64, u64)> = HashMap::new();

    for (a, b) in pairs {
        let pair_key = format!("{}_{}", a, b);
        registry.insert(pair_key.clone(), AdaptiveEngine::with_parameters(0.000002, 0.000002, 0.0001, z_threshold, loss_toxic, initial_vol, 0.1, 0.99));
        states.insert(pair_key.clone(), PositionState::Flat);
        balances.insert(pair_key.clone(), 100.0 / 21.0);
        trade_counts.insert(pair_key.clone(), 0);
    }

    for (_timestamp, rows_at_time) in timeline {
        for row in rows_at_time {
            latest_prices.insert(row.symbol.clone(), (row.close, row.vol, row.trade_count));
        }

        for (a, b) in pairs {
            let pair_key = format!("{}_{}", a, b);
            
            if let (Some(data_a), Some(data_b)) = (latest_prices.get(*a), latest_prices.get(*b)) {
                let baseline = baselines.entry(pair_key.clone()).or_insert((data_a.0, data_b.0));
                
                let norm_price_a = data_a.0 / baseline.0;
                let norm_price_b = data_b.0 / baseline.1;
                
                let engine = registry.get_mut(&pair_key).unwrap();
                let signal = engine.on_tick(
                    norm_price_a,
                    norm_price_b,
                    data_a.1,
                    data_b.1,
                    data_a.2,
                    data_b.2,
                );
                
                let state = states.get_mut(&pair_key).unwrap();
                let balance = balances.get_mut(&pair_key).unwrap();
                let trades = trade_counts.get_mut(&pair_key).unwrap();
                
                match (*state, signal) {
                    (PositionState::Flat, Signal::Buy) => {
                        *state = PositionState::LongSpread;
                        *trades += 1;
                        *balance -= 0.02;
                    }
                    (PositionState::Flat, Signal::Sell) => {
                        *state = PositionState::ShortSpread;
                        *trades += 1;
                        *balance -= 0.02;
                    }
                    (PositionState::LongSpread, Signal::Sell) => {
                        *state = PositionState::Flat;
                        *trades += 1;
                        *balance -= 0.02;
                        *balance += (norm_price_a - norm_price_b) * 1000.0;
                    }
                    (PositionState::ShortSpread, Signal::Buy) => {
                        *state = PositionState::Flat;
                        *trades += 1;
                        *balance -= 0.02;
                        *balance -= (norm_price_a - norm_price_b) * 1000.0;
                    }
                    _ => {}
                }
            }
        }
    }

    for (pair_key, balance) in balances {
        results.insert(pair_key.clone(), (balance, trade_counts[&pair_key]));
    }
    results
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = vec!["AAPL", "MSFT", "NVDA", "AMD", "GOOGL", "AMZN", "META"];
    let mut timeline: BTreeMap<String, Vec<Row>> = BTreeMap::new();

    for asset in &assets {
        let path = format!("data/{}.csv", asset);
        let mut rdr = Reader::from_path(path)?;
        for result in rdr.deserialize::<RawRow>() {
            let raw: RawRow = result?;
            timeline.entry(raw.timestamp.clone()).or_default().push(Row {
                timestamp: raw.timestamp,
                symbol: asset.to_string(),
                close: raw.close,
                vol: raw.vol,
                trade_count: raw.trade_count,
            });
        }
    }

    let mut pairs = Vec::new();
    for i in 0..assets.len() {
        for j in i + 1..assets.len() {
            pairs.push((assets[i], assets[j]));
        }
    }

    let z_thresholds = vec![0.30, 0.60, 1.00, 1.50];
    let size_thresholds = vec![500.0, 1500.0, 4000.0];
    let loss_toxics = vec![1.0, 3.0];

    let mut best_configs: HashMap<String, (f64, u64, f64, f64, f64)> = HashMap::new();

    for (a, b) in &pairs {
        let pair_key = format!("{}_{}", a, b);
        let mut best_balance = -f64::INFINITY;
        let mut best_config = (0.0, 0, 0.0, 0.0, 0.0);

        for &z in &z_thresholds {
            for &s in &size_thresholds {
                for &l in &loss_toxics {
                    let results = run_simulation(&[(a, b)], &timeline, z, s, l);
                    let (balance, trades) = results[&pair_key];
                    if balance > best_balance {
                        best_balance = balance;
                        best_config = (balance, trades, z, s, l);
                    }
                }
            }
        }
        best_configs.insert(pair_key, best_config);
    }

    println!("Optimization Report:");
    let mut global_balance = 0.0;
    for (pair_key, config) in best_configs {
        let (balance, trades, z, s, _l) = config;
        let pnl = balance - (100.0 / 21.0);
        global_balance += balance;
        println!("Pair: {} | Optimal Z: {:.2} | Size Thresh: {:.1} | Total Trades: {} | Net PnL: ${:.2}", pair_key, z, s, trades, pnl);
    }

    println!("Max Achieved Global Balance: ${:.2}", global_balance);

    Ok(())
}
