mod strategy;

use crate::strategy::{AdaptiveEngine, Strategy, Signal};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    
    // Boilerplate setup
    let mut engine = AdaptiveEngine::new();
    
    // Simulate incoming price stream
    let simulated_prices = [100.0, 101.0, 102.0, 101.0, 100.0];
    
    println!("Starting Trading Loop...");
    
    for price in simulated_prices {
        // Hot path: No heap allocations here
        let signal = engine.on_tick(price);
        
        match signal {
            Signal::Buy => println!("Price: {:.2} -> Signal: BUY", price),
            Signal::Sell => println!("Price: {:.2} -> Signal: SELL", price),
            Signal::Hold => println!("Price: {:.2} -> Signal: HOLD", price),
        }
    }

    Ok(())
}
