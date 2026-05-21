#[derive(Debug, PartialEq)]
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
    p_toxic_prior: f64,
    p_toxic_baseline: f64,
    decay_rate: f64,
    candidate_thresholds: [f64; 5],
    regrets: [f64; 5],
    prev_innovation: f64,
    tick_count: u64,
    internal_state: i8, // 0 = Flat, 1 = Long, -1 = Short
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
        p_toxic_baseline: f64,
        decay_rate: f64,
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
            p_toxic_prior: p_toxic_baseline,
            p_toxic_baseline,
            decay_rate,
            candidate_thresholds: [0.3, 0.6, 1.0, 1.5, 2.0],
            regrets: [0.0; 5],
            prev_innovation: 0.0,
            tick_count: 0,
            internal_state: 0,
        }
    }
}

impl Default for AdaptiveEngine {
    fn default() -> Self {
        Self::with_parameters(0.0001, 0.0001, 0.01, 2.0, 0.05, 100.0, 0.1, 0.99)
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
        self.tick_count += 1;

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

        // 2. Regret Matching for z_threshold tuning
        let std_dev = s.sqrt();
        let z = innovation / std_dev;
        let delta_innovation = innovation - self.prev_innovation;
        
        // Track regrets based on hypothetical PnL
        for (i, &cand_z) in self.candidate_thresholds.iter().enumerate() {
            let active_z = self.z_threshold;
            
            // Calculate hypothetical signal
            let get_hypo_signal = |z_val: f64| -> i8 {
                if z > z_val { -1 } else if z < -z_val { 1 } else { 0 }
            };
            
            let current_signal = get_hypo_signal(active_z);
            let cand_signal = get_hypo_signal(cand_z);
            
            let hypo_pnl = -(cand_signal as f64) * delta_innovation;
            let active_pnl = -(current_signal as f64) * delta_innovation;
            
            self.regrets[i] += (hypo_pnl - active_pnl).max(0.0);
        }

        // Periodic update
        if self.tick_count % 100 == 0 {
            let total_regret: f64 = self.regrets.iter().sum();
            if total_regret > 0.0 {
                let mut rng = rand::random::<f64>() * total_regret;
                for (i, &regret) in self.regrets.iter().enumerate() {
                    rng -= regret;
                    if rng <= 0.0 {
                        self.z_threshold = self.candidate_thresholds[i];
                        break;
                    }
                }
            }
            // Reset regrets partially to allow adaptation
            for r in self.regrets.iter_mut() { *r *= 0.5; }
        }

        // 3. Game Theory Logic (using updated z_threshold)
        let live_payoff = innovation.abs();
        let avg_size_a = if trades_a > 0 { vol_a / trades_a as f64 } else { 0.0 };
        let avg_size_b = if trades_b > 0 { vol_b / trades_b as f64 } else { 0.0 };

        let signal = (avg_size_a / self.size_threshold).max(avg_size_b / self.size_threshold).clamp(0.01, 0.99);
        let likelihood_toxic = signal;
        let likelihood_noise = 1.0 - signal;

        let numerator = likelihood_toxic * self.p_toxic_prior;
        let denominator = numerator + likelihood_noise * (1.0 - self.p_toxic_prior);
        self.p_toxic_prior = numerator / denominator;
        self.p_toxic_prior = self.p_toxic_prior * self.decay_rate + self.p_toxic_baseline * (1.0 - self.decay_rate);
        
        let p_toxic = self.p_toxic_prior;

        let payoff_agg_noise = live_payoff;
        let payoff_agg_toxic = (live_payoff * 0.2) - (self.loss_toxic * 0.5);
        let payoff_pass_noise = live_payoff * 0.8;
        let payoff_pass_toxic = -self.loss_toxic;

        let market_payoff_agg_noise = -payoff_agg_noise;
        let market_payoff_agg_toxic = -payoff_agg_toxic;
        let market_payoff_pass_noise = -payoff_pass_noise;
        let market_payoff_pass_toxic = -payoff_pass_toxic;

        let eu_agg = p_toxic * payoff_agg_toxic + (1.0 - p_toxic) * payoff_agg_noise;
        let eu_pass = p_toxic * payoff_pass_toxic + (1.0 - p_toxic) * payoff_pass_noise;

        let denominator = market_payoff_agg_toxic - market_payoff_pass_toxic - market_payoff_agg_noise + market_payoff_pass_noise;
        let q = if denominator.abs() > 1e-9 {
            (market_payoff_pass_noise - market_payoff_pass_toxic) / denominator
        } else {
            0.5
        };
        let q = q.clamp(0.0, 1.0);

        if eu_agg <= 0.0 && eu_pass <= 0.0 {
            self.internal_state = 0;
            self.prev_innovation = innovation;
            return Signal::Hold;
        }

        let signal_result = match self.internal_state {
            1 => {
                if z >= 0.0 { self.internal_state = 0; Signal::Hold } else { Signal::Buy }
            }
            -1 => {
                if z <= 0.0 { self.internal_state = 0; Signal::Hold } else { Signal::Sell }
            }
            _ => {
                if z > self.z_threshold { self.internal_state = -1; Signal::Sell }
                else if z < -self.z_threshold { self.internal_state = 1; Signal::Buy }
                else { Signal::Hold }
            }
        };

        if signal_result != Signal::Hold {
            let action = if q > 0.5 { "Aggressive Market Fill" } else { "Passive Limit Fill" };
            println!("Engine recommends: {} for {:?}", action, signal_result);
        }
        
        self.prev_innovation = innovation;
        signal_result
    }
}
