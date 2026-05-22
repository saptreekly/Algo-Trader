mod environment;
mod memory;
mod model;
mod agent;

use environment::TradingEnvironment;
use agent::DQNAgent;
use dfdx::prelude::*;

fn main() {
    let dev = Cpu::default();
    let model = dev.build_module::<model::QNetwork, f32>();
    let mut agent = DQNAgent::new(model);
    let mut env = TradingEnvironment::new(vec![vec![0.1; 8]; 10]);

    let action = agent.act(&vec![0.1; 8]);
    let (_next, reward, _done) = env.step(action);
    
    agent.train_step();
    println!("System operational, reward: {}", reward);
}
