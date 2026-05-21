pub enum Signal {
    Buy,
    Sell,
    Hold,
}

pub trait Strategy {
    fn on_tick(&mut self, price: f64) -> Signal;
}

pub struct AdaptiveEngine {
    // Using a fixed-size array to avoid heap allocations in the hot path
    window: [f64; 10],
    index: usize,
}

impl AdaptiveEngine {
    pub fn new() -> Self {
        Self {
            window: [0.0; 10],
            index: 0,
        }
    }

    fn detect_regime(&self) -> bool {
        // Placeholder: Logic to detect expanding volatility
        false
    }
}

impl Strategy for AdaptiveEngine {
    fn on_tick(&mut self, price: f64) -> Signal {
        // Update rolling window
        self.window[self.index] = price;
        self.index = (self.index + 1) % 10;

        // Check regime detection
        if self.detect_regime() {
            Signal::Hold
        } else {
            // Simplified logic placeholder
            if price > self.window[0] {
                Signal::Buy
            } else {
                Signal::Sell
            }
        }
    }
}
