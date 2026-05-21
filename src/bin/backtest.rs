use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use csv::Reader;
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Deserialize)]
struct Row {
    #[allow(dead_code)]
    timestamp: String,
    close_a: f64,
    close_b: f64,
    trade_count_a: u64,
    trade_count_b: u64,
}

fn run_simulation(rows: &[Row], win_spread: f64, loss_toxic: f64, size_threshold: f64) -> (f64, i32) {
    let mut engine = AdaptiveEngine::with_parameters(win_spread, loss_toxic, size_threshold);
    let mut balance = 100_000.0;
    let mut position_active = false;
    let mut entry_price_a = 0.0;
    let mut entry_price_b = 0.0;
    let mut total_trades = 0;

    for row in rows {
        // Use placeholder 0.0 for volume, but pass real trade counts
        let signal = engine.on_tick(row.close_a, row.close_b, 0.0, 0.0, row.trade_count_a, row.trade_count_b);

        match signal {
            Signal::Buy => {
                if !position_active {
                    position_active = true;
                    entry_price_a = row.close_a;
                    entry_price_b = row.close_b;
                    total_trades += 1;
                }
            }
            Signal::Sell => {
                if position_active {
                    position_active = false;
                    let pnl = (row.close_a - entry_price_a) - (row.close_b - entry_price_b);
                    balance += pnl;
                }
            }
            Signal::Hold => {}
        }
    }
    (balance, total_trades)
}

fn main() -> Result<(), Box<dyn Error>> {
    let file_path = "data/historical_pairs.csv";
    let mut rdr = Reader::from_path(file_path)?;
    let rows: Vec<Row> = rdr.deserialize().filter_map(Result::ok).collect();

    let mut best_balance = -1.0;
    let mut best_params = (0.0, 0.0, 0.0);
    let mut best_trades = 0;

    for &win_spread in &[1.0, 2.0, 3.0, 4.0, 5.0] {
        for &loss_toxic in &[2.0, 4.0, 6.0, 8.0, 10.0] {
            for &size_threshold in &[10.0, 30.0, 50.0, 70.0, 90.0] {
                let (final_balance, total_trades) = run_simulation(&rows, win_spread, loss_toxic, size_threshold);
                if final_balance > best_balance {
                    best_balance = final_balance;
                    best_params = (win_spread, loss_toxic, size_threshold);
                    best_trades = total_trades;
                }
            }
        }
    }

    println!("--- Optimization Report ---");
    println!("Best Win Spread:      {:.2}", best_params.0);
    println!("Best Toxic Loss:      {:.2}", best_params.1);
    println!("Best Size Threshold:  {:.2}", best_params.2);
    println!("Max Achieved Balance: ${:.2}", best_balance);
    println!("Total Trades Executed: {}", best_trades);

    Ok(())
}
