pub enum Signal {
    Buy,
    Sell,
    Hold,
}

pub trait Strategy {
    fn on_tick(&mut self, price: f64) -> Signal;
}

pub struct AdaptiveEngine {
    state_estimate: f64,
    error_covariance: f64,
    process_noise: f64,
    measurement_noise: f64,
}

impl AdaptiveEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parameters(process_noise: f64, measurement_noise: f64) -> Self {
        Self {
            state_estimate: 0.0,
            error_covariance: 1.0,
            process_noise,
            measurement_noise,
        }
    }

    fn detect_regime(&self) -> bool {
        // High covariance indicates high uncertainty/volatility
        self.error_covariance > 0.5
    }
}

impl Default for AdaptiveEngine {
    fn default() -> Self {
        Self {
            state_estimate: 0.0,
            error_covariance: 1.0,
            process_noise: 0.01,
            measurement_noise: 0.1,
        }
    }
}

impl Strategy for AdaptiveEngine {
    fn on_tick(&mut self, price: f64) -> Signal {
        // Initialize estimate on first tick
        if self.state_estimate == 0.0 {
            self.state_estimate = price;
        }

        // 1. Prediction Step
        // Predict next state estimate (no change expected) and increase uncertainty
        self.error_covariance += self.process_noise;

        // 2. Update Step
        // Calculate Kalman Gain: K = P / (P + R)
        let kalman_gain = self.error_covariance / (self.error_covariance + self.measurement_noise);

        // Innovation: z - Hx (assuming H=1)
        let innovation = price - self.state_estimate;

        // Update estimate
        self.state_estimate += kalman_gain * innovation;

        // Update covariance: P = (1 - K) * P
        self.error_covariance *= 1.0 - kalman_gain;

        // Regime detection
        if self.detect_regime() {
            return Signal::Hold;
        }

        // Signal generation: Mean reversion (Z-score > 2)
        let std_dev = self.error_covariance.sqrt();
        if innovation > 2.0 * std_dev {
            Signal::Sell // Price too high relative to estimate
        } else if innovation < -2.0 * std_dev {
            Signal::Buy // Price too low relative to estimate
        } else {
            Signal::Hold
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_engine_updates_state() {
        let mut engine = AdaptiveEngine::new();
        // First tick sets initial state
        let signal1 = engine.on_tick(100.0);
        assert!(matches!(signal1, Signal::Hold));

        // Innovation is 5. Standard deviation is sqrt(P) ~= 1.0.
        // 5 > 2 * 1.0, so it triggers Sell.
        let signal2 = engine.on_tick(105.0);
        assert!(matches!(signal2, Signal::Sell));
    }
}
