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
    vol_a: f64,
    vol_b: f64,
    trade_count_a: u64,
    trade_count_b: u64,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum PositionState {
    Flat,
    LongSpread,
    ShortSpread,
}

fn run_simulation(
    rows: &[Row],
    q_alpha: f64,
    q_beta: f64,
    r_noise: f64,
    z_threshold: f64,
    loss_toxic: f64,
    size_threshold: f64,
) -> (f64, i32) {
    if rows.is_empty() {
        return (100_000.0, 0);
    }

    let mut engine = AdaptiveEngine::with_parameters(
        q_alpha,
        q_beta,
        r_noise,
        z_threshold,
        loss_toxic,
        size_threshold,
    );
    
    let mut balance = 100_000.0;
    let mut state = PositionState::Flat;
    let mut entry_price_a = 0.0;
    let mut entry_price_b = 0.0;
    let mut total_trades = 0;

    for row in rows {
        // Strategy signal based on normalized prices and real volume data
        let signal = engine.on_tick(
            row.close_a,
            row.close_b,
            row.vol_a,
            row.vol_b,
            row.trade_count_a,
            row.trade_count_b,
        );

        match signal {
            Signal::Buy => {
                if state == PositionState::Flat {
                    state = PositionState::LongSpread;
                    entry_price_a = row.close_a;
                    entry_price_b = row.close_b;
                    total_trades += 1;
                }
            }
            Signal::Sell => {
                if state == PositionState::Flat {
                    state = PositionState::ShortSpread;
                    entry_price_a = row.close_a;
                    entry_price_b = row.close_b;
                    total_trades += 1;
                }
            }
            Signal::Hold => {
                if state == PositionState::LongSpread {
                    let pnl = (row.close_a - entry_price_a) - (row.close_b - entry_price_b) - 0.02;
                    balance += pnl;
                    state = PositionState::Flat;
                } else if state == PositionState::ShortSpread {
                    let pnl = (entry_price_a - row.close_a) - (entry_price_b - row.close_b) - 0.02;
                    balance += pnl;
                    state = PositionState::Flat;
                }
            }
        }
    }
    (balance, total_trades)
}

fn main() -> Result<(), Box<dyn Error>> {
    let file_path = "data/historical_pairs.csv";
    let mut rdr = Reader::from_path(file_path)?;
    let rows: Vec<Row> = rdr.deserialize().filter_map(Result::ok).collect();

    let mut best_balance = -1.0;
    let mut best_params = (0.0, 0.0, 0.0, 0.0);
    let mut best_trades = 0;

    let r_noise = 0.0001;
    let loss_toxic = 1.0;

    let mut q_val = 0.000002;
    while q_val <= 0.000022 {
        let q_alpha = q_val;
        let q_beta = q_val;

        for &z_threshold in &[0.3, 0.5, 0.7, 1.0] {
            for &size_threshold in &[500.0, 1000.0, 2000.0, 4000.0] {
                let (final_balance, total_trades) = run_simulation(
                    &rows,
                    q_alpha,
                    q_beta,
                    r_noise,
                    z_threshold,
                    loss_toxic,
                    size_threshold,
                );
                
                if final_balance > best_balance {
                    best_balance = final_balance;
                    best_params = (q_val, z_threshold, size_threshold, loss_toxic);
                    best_trades = total_trades;
                }
            }
        }
        q_val += 0.000005;
    }

    println!("--- Optimization Report ---");
    println!("Best Process Noise:   {:.6}", best_params.0);
    println!("Best Z-Threshold:     {:.2}", best_params.1);
    println!("Best Size Threshold:  {:.2}", best_params.2);
    println!("Best Loss Toxic:      {:.2}", best_params.3);
    println!("Max Achieved Balance: ${:.2}", best_balance);
    println!("Total Trades Executed: {}", best_trades);

    Ok(())
}
