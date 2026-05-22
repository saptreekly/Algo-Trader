mod environment;
mod model;
mod agent;
mod data_loader;

use environment::TradingEnvironment;
use agent::DQNAgent;
use dfdx::prelude::*;

fn main() {
    let dev = Cpu::default();
    let model = dev.build_module::<model::QNetwork, f32>();
    let agent = DQNAgent::new(model);

    let assets = data_loader::load_universe_data("data/mag7_alpaca_quotes.csv").expect("Failed to load data");
    let mut env = TradingEnvironment::new(assets);
    let tickers: Vec<String> = env.assets.keys().cloned().collect();

    for ticker in tickers {
        if env.load_ticker_session(&ticker).is_ok() {
            println!("Training on: {}", ticker);
            for _ in 0..100 {
                let action = agent.act(&[0.0; 8]);
                let (_, _, is_done) = env.step(action);
                if is_done { break; }
            }
        }
    }
}
