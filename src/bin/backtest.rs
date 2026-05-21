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
}

fn run_simulation(rows: &[Row], q_alpha: f64, q_beta: f64, r_noise: f64, z_threshold: f64) -> (f64, i32) {
    let mut engine = AdaptiveEngine::with_parameters(q_alpha, q_beta, r_noise, z_threshold);
    let mut balance = 100_000.0;
    let mut position_active = false;
    let mut entry_price_a = 0.0;
    let mut entry_price_b = 0.0;
    let mut total_trades = 0;

    for row in rows {
        let current_beta = engine.get_beta(); 
        let signal = engine.on_tick(row.close_a, row.close_b);

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
                    // Dollar-neutral payout: Long A, Short current_beta * B
                    let pnl = (row.close_a - entry_price_a) - current_beta * (row.close_b - entry_price_b);
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
    let mut best_params = (0.0, 0.0, 0.0, 0.0);
    let mut best_trades = 0;

    let mut q_alpha = 0.0001;
    while q_alpha <= 0.001 {
        let mut q_beta = 0.0001;
        while q_beta <= 0.001 {
            let mut r_noise = 0.01;
            while r_noise <= 0.10 {
                for &z_threshold in &[0.5, 1.0, 1.5, 2.0] {
                    let (final_balance, total_trades) = run_simulation(&rows, q_alpha, q_beta, r_noise, z_threshold);
                    if final_balance > best_balance {
                        best_balance = final_balance;
                        best_params = (q_alpha, q_beta, r_noise, z_threshold);
                        best_trades = total_trades;
                    }
                }
                r_noise += 0.02;
            }
            q_beta += 0.0002;
        }
        q_alpha += 0.0002;
    }

    println!("--- Optimization Report ---");
    println!("Best Process Noise (alpha): {:.4}", best_params.0);
    println!("Best Process Noise (beta):  {:.4}", best_params.1);
    println!("Best Measurement Noise:     {:.4}", best_params.2);
    println!("Best Z-Threshold:           {:.2}", best_params.3);
    println!("Max Achieved Balance:       ${:.2}", best_balance);
    println!("Total Trades Executed:      {}", best_trades);

    Ok(())
}
