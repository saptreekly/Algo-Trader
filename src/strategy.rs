pub enum Signal {
    Buy,  // Spread too low: Long A, Short Beta * B
    Sell, // Spread too high: Short A, Long Beta * B
    Hold,
}

pub trait Strategy {
    fn on_tick(&mut self, price_a: f64, price_b: f64) -> Signal;
}

pub struct AdaptiveEngine {
    alpha: f64,
    beta: f64,
    p00: f64,
    p01: f64,
    p10: f64,
    p11: f64,
    q_alpha: f64,
    q_beta: f64,
    r_noise: f64,
    z_threshold: f64,
}

impl AdaptiveEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parameters(q_alpha: f64, q_beta: f64, r_noise: f64, z_threshold: f64) -> Self {
        Self {
            alpha: 0.0,
            beta: 1.0,
            p00: 1.0,
            p01: 0.0,
            p10: 0.0,
            p11: 1.0,
            q_alpha,
            q_beta,
            r_noise,
            z_threshold,
        }
    }

    pub fn get_beta(&self) -> f64 {
        self.beta
    }
}

impl Default for AdaptiveEngine {
    fn default() -> Self {
        Self::with_parameters(0.0001, 0.0001, 0.01, 2.0)
    }
}

impl Strategy for AdaptiveEngine {
    fn on_tick(&mut self, price_a: f64, price_b: f64) -> Signal {
        // Prediction Step
        self.p00 += self.q_alpha;
        self.p11 += self.q_beta;

        // Innovation
        let innovation = price_a - (self.alpha + self.beta * price_b);

        // Innovation Variance (S = H*P*H^T + R)
        let s = self.p00 + price_b * (self.p01 + self.p10 + price_b * self.p11) + self.r_noise;

        // Kalman Gain (K = P * H^T / S)
        let k0 = (self.p00 + self.p01 * price_b) / s;
        let k1 = (self.p10 + self.p11 * price_b) / s;

        // State Update
        self.alpha += k0 * innovation;
        self.beta += k1 * innovation;

        // Covariance Matrix Update (P = (I - K * H) * P)
        let m00 = 1.0 - k0;
        let m01 = -k0 * price_b;
        let m10 = -k1;
        let m11 = 1.0 - k1 * price_b;

        let new_p00 = m00 * self.p00 + m01 * self.p10;
        let new_p01 = m00 * self.p01 + m01 * self.p11;
        let new_p10 = m10 * self.p00 + m11 * self.p10;
        let new_p11 = m10 * self.p01 + m11 * self.p11;

        self.p00 = new_p00;
        self.p01 = new_p01;
        self.p10 = new_p10;
        self.p11 = new_p11;

        // Signal generation
        let std_dev = s.sqrt();
        if innovation > self.z_threshold * std_dev {
            Signal::Sell
        } else if innovation < -self.z_threshold * std_dev {
            Signal::Buy
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
        let signal1 = engine.on_tick(100.0, 100.0);
        assert!(matches!(signal1, Signal::Hold));

        // Spread 105 - 100 = 5. Signal triggers based on innovation vs std_dev
        let signal2 = engine.on_tick(105.0, 100.0);
        assert!(matches!(signal2, Signal::Sell));
    }
}
