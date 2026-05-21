pub enum Signal {
    Buy,
    Sell,
    Hold,
}

pub trait Strategy {
    fn on_tick(
        &mut self,
        price_a: f64,
        price_b: f64,
        vol_a: f64,
        vol_b: f64,
        trades_a: u64,
        trades_b: u64,
    ) -> Signal;
}

pub struct AdaptiveEngine {
    win_spread: f64,
    loss_toxic: f64,
    size_threshold: f64,
}

impl AdaptiveEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parameters(win_spread: f64, loss_toxic: f64, size_threshold: f64) -> Self {
        Self {
            win_spread,
            loss_toxic,
            size_threshold,
        }
    }
}

impl Default for AdaptiveEngine {
    fn default() -> Self {
        Self::with_parameters(0.01, 0.05, 100.0)
    }
}

impl Strategy for AdaptiveEngine {
    fn on_tick(
        &mut self,
        price_a: f64,
        price_b: f64,
        vol_a: f64,
        _vol_b: f64,
        trades_a: u64,
        _trades_b: u64,
    ) -> Signal {
        let avg_size_a = if trades_a > 0 {
            vol_a / trades_a as f64
        } else {
            0.0
        };

        let p_toxic = (avg_size_a / self.size_threshold).min(1.0);

        let payoff_passive = (1.0 - p_toxic) * self.win_spread + p_toxic * (-self.loss_toxic);
        let payoff_aggressive =
            (1.0 - p_toxic) * (self.win_spread * 0.5) + p_toxic * (-self.loss_toxic * 0.2);

        let spread = price_a - price_b;

        if payoff_passive > payoff_aggressive && payoff_passive > 0.0 {
            if spread < 0.0 {
                Signal::Buy
            } else {
                Signal::Sell
            }
        } else if payoff_aggressive > payoff_passive && payoff_aggressive > 0.0 {
            if spread < 0.0 {
                Signal::Buy
            } else {
                Signal::Sell
            }
        } else {
            Signal::Hold
        }
    }
}
