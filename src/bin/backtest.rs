use algo_trader::strategy::{AdaptiveEngine, Signal, Strategy};
use csv::Reader;
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct Bar {
    c: f64, // Close
}

fn run_simulation(bars: &[Bar], process_noise: f64, measurement_noise: f64) -> f64 {
    let mut engine = AdaptiveEngine::with_parameters(process_noise, measurement_noise);
    let mut balance = 100_000.0;
    let mut position_price: Option<f64> = None;

    for bar in bars {
        let signal = engine.on_tick(bar.c);
        match signal {
            Signal::Buy => {
                if position_price.is_none() {
                    position_price = Some(bar.c);
                }
            }
            Signal::Sell => {
                if let Some(buy_price) = position_price {
                    balance += bar.c - buy_price;
                    position_price = None;
                }
            }
            Signal::Hold => {}
        }
    }
    balance
}

fn main() -> Result<(), Box<dyn Error>> {
    let file_path = "data/historical_bars.csv";
    let mut rdr = Reader::from_path(file_path)?;

    // Pre-load bars for faster grid search
    let bars: Vec<Bar> = rdr.deserialize().filter_map(Result::ok).collect();

    let mut best_balance = -1.0;
    let mut best_params = (0.0, 0.0);

    let mut p_noise = 0.001;
    while p_noise <= 0.05 {
        let mut m_noise = 0.05;
        while m_noise <= 0.5 {
            let final_balance = run_simulation(&bars, p_noise, m_noise);
            if final_balance > best_balance {
                best_balance = final_balance;
                best_params = (p_noise, m_noise);
            }
            m_noise += 0.05;
        }
        p_noise += 0.005;
    }

    println!(
        "WINNING HYPERPARAMETERS -> Process Noise: {:.4}, Measurement Noise: {:.4} -> Yielded Final Balance: ${:.2}",
        best_params.0, best_params.1, best_balance
    );

    Ok(())
}
