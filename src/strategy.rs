#[derive(Debug)]
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
    loss_toxic: f64,
    size_threshold: f64,
}

impl AdaptiveEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parameters(
        q_alpha: f64,
        q_beta: f64,
        r_noise: f64,
        z_threshold: f64,
        loss_toxic: f64,
        size_threshold: f64,
    ) -> Self {
        Self {
            alpha: 0.0,
            beta: 1.0,
            p00: 0.001,
            p01: 0.0,
            p10: 0.0,
            p11: 0.001,
            q_alpha,
            q_beta,
            r_noise,
            z_threshold,
            loss_toxic,
            size_threshold,
        }
    }
}

impl Default for AdaptiveEngine {
    fn default() -> Self {
        Self::with_parameters(0.0001, 0.0001, 0.01, 2.0, 0.05, 100.0)
    }
}

impl Strategy for AdaptiveEngine {
    fn on_tick(
        &mut self,
        price_a: f64,
        price_b: f64,
        vol_a: f64,
        vol_b: f64,
        trades_a: u64,
        trades_b: u64,
    ) -> Signal {
        // 1. Kalman Filter Update
        self.p00 += self.q_alpha;
        self.p11 += self.q_beta;
        let innovation = price_a - (self.alpha + self.beta * price_b);
        let dynamic_r = self.r_noise * (1.0 + (vol_a + vol_b).ln_1p());
        let s = self.p00 + price_b * (self.p01 + self.p10 + price_b * self.p11) + dynamic_r;
        let k0 = (self.p00 + self.p01 * price_b) / s;
        let k1 = (self.p10 + self.p11 * price_b) / s;
        self.alpha += k0 * innovation;
        self.beta += k1 * innovation;
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

        // 2. Game Theory Logic
        let live_payoff = innovation.abs();
        let avg_size_a = if trades_a > 0 {
            vol_a / trades_a as f64
        } else {
            0.0
        };
        let avg_size_b = if trades_b > 0 {
            vol_b / trades_b as f64
        } else {
            0.0
        };
        let p_toxic_a = (avg_size_a / self.size_threshold).min(1.0);
        let p_toxic_b = (avg_size_b / self.size_threshold).min(1.0);
        let p_toxic = p_toxic_a.max(p_toxic_b);

        let payoff_passive = (1.0 - p_toxic) * live_payoff + p_toxic * (-self.loss_toxic);
        let payoff_aggressive =
            (1.0 - p_toxic) * (live_payoff * 0.5) + p_toxic * (-self.loss_toxic * 0.2);

        if payoff_passive <= 0.0 && payoff_aggressive <= 0.0 {
            return Signal::Hold;
        }

        // 3. Directional Signal
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
