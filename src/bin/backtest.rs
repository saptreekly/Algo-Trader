use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use csv::Reader;
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct Bar {
    t: String,
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    v: f64,
}

fn run_simulation(bars: &[Bar], process_noise: f64, measurement_noise: f64) -> (f64, i32) {
    let mut engine = AdaptiveEngine::with_parameters(process_noise, measurement_noise);
    let mut balance = 100_000.0;
    let mut position_active = false;
    let mut entry_price = 0.0;
    let mut total_trades = 0;

    for bar in bars {
        let signal = engine.on_tick(bar.c);
        match signal {
            Signal::Buy => {
                if !position_active {
                    position_active = true;
                    entry_price = bar.c;
                    total_trades += 1;
                }
            }
            Signal::Sell => {
                if position_active {
                    position_active = false;
                    balance += bar.c - entry_price;
                }
            }
            Signal::Hold => {}
        }
    }
    (balance, total_trades)
}

fn main() -> Result<(), Box<dyn Error>> {
    let file_path = "data/historical_bars.csv";
    let mut rdr = Reader::from_path(file_path)?;
    let bars: Vec<Bar> = rdr.deserialize().filter_map(Result::ok).collect();

    let mut best_balance = -1.0;
    let mut best_params = (0.0, 0.0);
    let mut best_trades = 0;

    let mut p_noise = 0.001;
    while p_noise <= 0.05 {
        let mut m_noise = 0.05;
        while m_noise <= 0.50 {
            let (final_balance, total_trades) = run_simulation(&bars, p_noise, m_noise);
            if final_balance > best_balance {
                best_balance = final_balance;
                best_params = (p_noise, m_noise);
                best_trades = total_trades;
            }
            m_noise += 0.05;
        }
        p_noise += 0.005;
    }

    println!("--- Optimization Report ---");
    println!("Best Process Noise:     {:.4}", best_params.0);
    println!("Best Measurement Noise: {:.4}", best_params.1);
    println!("Max Achieved Balance:   ${:.2}", best_balance);
    println!("Total Trades Executed:  {}", best_trades);

    Ok(())
}
