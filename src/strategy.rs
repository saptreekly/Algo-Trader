use std::collections::VecDeque;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Signal {
    Buy,
    Sell,
    Close,
    Hold,
}

#[derive(Debug)]
pub struct Action {
    pub signal: Signal,
    pub size: f64,
    pub execution_slippage: f64,
}

pub trait Strategy {
    fn on_tick(
        &mut self,
        raw_price_a: f64,
        raw_price_b: f64,
        vol_a: f64,
        vol_b: f64,
        trades_a: u64,
        trades_b: u64,
        account_balance: f64,
    ) -> Action;
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
    rolling_variance: f64,
    rolling_mean: f64,
    innovation_history: VecDeque<f64>,    r_noise: f64,
    z_threshold: f64,
    loss_toxic: f64,
    rolling_size_a: f64,
    rolling_size_b: f64,
    p_toxic_prior: f64,
    p_toxic_baseline: f64,
    decay_rate: f64,
    candidate_thresholds: [f64; 5],
    regrets: [f64; 5],
    virtual_states: [i8; 5],
    virtual_entries: [f64; 5],
    prev_innovation: f64,
    tick_count: u64,
    internal_state: i8, // 0 = Flat, 1 = Long, -1 = Short
    excursion_lock: bool,
}

impl AdaptiveEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parameters(
        r_noise: f64,
        z_threshold: f64,
        loss_toxic: f64,
        initial_vol: f64,
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
            q_alpha: 0.00001,
            q_beta: 0.00001,
            rolling_variance: 0.01,
            rolling_mean: 0.0,
            innovation_history: VecDeque::with_capacity(100),
            r_noise,
            z_threshold,
            loss_toxic,
            rolling_size_a: initial_vol,
            rolling_size_b: initial_vol,
            p_toxic_prior: p_toxic_baseline,
            p_toxic_baseline,
            decay_rate,
            candidate_thresholds: [1.0, 1.5, 2.0, 2.5, 3.0],
            regrets: [0.0; 5],
            virtual_states: [0; 5],
            virtual_entries: [0.0; 5],
            prev_innovation: 0.0,
            tick_count: 0,
            internal_state: 0,
            excursion_lock: false,
        }
    }
}

impl Default for AdaptiveEngine {
    fn default() -> Self {
        Self::with_parameters(0.01, 2.0, 0.05, 100.0, 0.1, 0.99)
    }
}

impl Strategy for AdaptiveEngine {
    fn on_tick(
        &mut self,
        raw_price_a: f64,
        raw_price_b: f64,
        vol_a: f64,
        vol_b: f64,
        trades_a: u64,
        trades_b: u64,
        account_balance: f64,
    ) -> Action {
        self.tick_count += 1;

        // Apply Process Noise
        self.p00 += self.q_alpha;
        self.p11 += self.q_beta;

        let available_buying_power = account_balance * 2.0;
        let max_shares = (available_buying_power / 2.0) / (raw_price_a + raw_price_b);
        let position_value = max_shares * (raw_price_a + raw_price_b);

        if position_value < 1.0 {
            self.internal_state = 0;
            self.prev_innovation = 0.0;
            return Action { signal: Signal::Hold, size: 0.0, execution_slippage: 0.0 };
        }

        // 1. Kalman Filter Update
        let innovation = raw_price_a - (self.alpha + self.beta * raw_price_b);
        
        // EWMA Variance Tracking for Z-Score
        self.rolling_mean = (self.rolling_mean * 0.995) + (innovation * 0.005);
        let demeaned_innovation = innovation - self.rolling_mean;
        self.rolling_variance = (self.rolling_variance * 0.995) + (demeaned_innovation * demeaned_innovation * 0.005);
        let statistical_std_dev = self.rolling_variance.sqrt().max(1e-6);
        let z = innovation / statistical_std_dev;

        // Reset Lock on Re-entry
        if z.abs() <= self.z_threshold { self.excursion_lock = false; }

        // Update history and stationarity check
        self.innovation_history.push_back(innovation);
        if self.innovation_history.len() > 100 {
            self.innovation_history.pop_front();
        }

        let mut variance_ratio = 1.0;
        if self.innovation_history.len() >= 100 {
            let n = self.innovation_history.len();
            let v1: f64 = self.innovation_history.iter()
                .zip(self.innovation_history.iter().skip(1))
                .map(|(a, b)| (a - b).powi(2)).sum::<f64>() / (n - 1) as f64;
            
            let v5: f64 = self.innovation_history.iter()
                .zip(self.innovation_history.iter().skip(5))
                .map(|(a, b)| (a - b).powi(2)).sum::<f64>() / (5.0 * (n - 5) as f64);
            
            variance_ratio = if v1 > 1e-9 { v5 / v1 } else { 1.0 };
        }

        let dynamic_r = self.r_noise * (1.0 + (vol_a + vol_b).ln_1p());
        let s = self.p00 + raw_price_b * (self.p01 + self.p10 + raw_price_b * self.p11) + dynamic_r;
        let k0 = (self.p00 + self.p01 * raw_price_b) / s;
        let k1 = (self.p10 + self.p11 * raw_price_b) / s;
        self.alpha += k0 * innovation;
        self.beta += k1 * innovation;
        let m00 = 1.0 - k0;
        let m01 = -k0 * raw_price_b;
        let m10 = -k1;
        let m11 = 1.0 - k1 * raw_price_b;

        let mp00 = m00 * self.p00 + m01 * self.p10;
        let mp01 = m00 * self.p01 + m01 * self.p11;
        let mp10 = m10 * self.p00 + m11 * self.p10;
        let mp11 = m10 * self.p01 + m11 * self.p11;

        let mpmt00 = mp00 * m00 + mp01 * m01;
        let mpmt01 = mp00 * m10 + mp01 * m11;
        let mpmt10 = mp10 * m00 + mp11 * m01;
        let mpmt11 = mp10 * m10 + mp11 * m11;

        let raw_j_p00 = mpmt00 + k0 * k0 * dynamic_r;
        let raw_j_p01 = mpmt01 + k0 * k1 * dynamic_r;
        let raw_j_p10 = mpmt10 + k1 * k0 * dynamic_r;
        let raw_j_p11 = mpmt11 + k1 * k1 * dynamic_r;

        let symmetric_cross = 0.5 * (raw_j_p01 + raw_j_p10);

        self.p00 = raw_j_p00;
        self.p01 = symmetric_cross;
        self.p10 = symmetric_cross;
        self.p11 = raw_j_p11;

        // 2. Regret Matching with Virtual Positions
        let raw_spread = raw_price_a - raw_price_b;
        let friction_cost = 0.00015;

        for (i, &cand_z) in self.candidate_thresholds.iter().enumerate() {
            match self.virtual_states[i] {
                0 => {
                    if z > cand_z {
                        self.virtual_states[i] = -1; // Trigger Short
                        self.virtual_entries[i] = raw_spread;
                    } else if z < -cand_z {
                        self.virtual_states[i] = 1; // Trigger Long
                        self.virtual_entries[i] = raw_spread;
                    }
                }
                1 => { // Long
                    if z >= 0.0 {
                        let realized_pnl = raw_spread - self.virtual_entries[i];
                        self.regrets[i] += realized_pnl - friction_cost;
                        self.virtual_states[i] = 0;
                    }
                }
                -1 => { // Short
                    if z <= 0.0 {
                        let realized_pnl = self.virtual_entries[i] - raw_spread;
                        self.regrets[i] += realized_pnl - friction_cost;
                        self.virtual_states[i] = 0;
                    }
                }
                _ => {}
            }
            self.regrets[i] = self.regrets[i].max(0.0);
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
        let expected_reversion_payoff = z.abs() * statistical_std_dev;
        let avg_size_a = if trades_a > 0 { vol_a / trades_a as f64 } else { 0.0 };
        let avg_size_b = if trades_b > 0 { vol_b / trades_b as f64 } else { 0.0 };

        // Update Dynamic Volume Baselines (EWMA)
        if avg_size_a > 0.0 {
            self.rolling_size_a = (self.rolling_size_a * 0.999) + (avg_size_a * 0.001);
        }
        if avg_size_b > 0.0 {
            self.rolling_size_b = (self.rolling_size_b * 0.999) + (avg_size_b * 0.001);
        }

        let dynamic_thresh_a = self.rolling_size_a * 3.0;
        let dynamic_thresh_b = self.rolling_size_b * 3.0;

        // Bayesian Signaling Update for Toxic Probability
        let signal = (avg_size_a / dynamic_thresh_a).max(avg_size_b / dynamic_thresh_b).clamp(0.01, 0.99);
        let likelihood_toxic = signal;
        let likelihood_noise = 1.0 - signal;

        let numerator = likelihood_toxic * self.p_toxic_prior;
        let denominator = numerator + likelihood_noise * (1.0 - self.p_toxic_prior);
        self.p_toxic_prior = numerator / denominator;
        self.p_toxic_prior = self.p_toxic_prior * self.decay_rate + self.p_toxic_baseline * (1.0 - self.decay_rate);

        let p_toxic = self.p_toxic_prior;

        // Asymmetric Payoffs
        let toxic_multiplier = if innovation > 0.0 { 1.5 } else { 0.8 };
        let effective_loss_toxic = self.loss_toxic * toxic_multiplier;

        let passive_friction = friction_cost * 0.5;

        let payoff_agg_noise = expected_reversion_payoff - friction_cost;
        let payoff_agg_toxic = (expected_reversion_payoff * 0.2) - (effective_loss_toxic * 0.5) - friction_cost;
        let payoff_pass_noise = (expected_reversion_payoff * 0.8) - passive_friction;
        let payoff_pass_toxic = -effective_loss_toxic - passive_friction;

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
            return Action { signal: Signal::Hold, size: 0.0, execution_slippage: 0.0 };
        }

        let signal_result = match self.internal_state {
            1 => {
                if z >= 0.0 { self.internal_state = 0; Signal::Close } else { Signal::Hold }
            }
            -1 => {
                if z <= 0.0 { self.internal_state = 0; Signal::Close } else { Signal::Hold }
            }
            _ => {
                if self.innovation_history.len() >= 100 && variance_ratio > 0.75 {
                    self.internal_state = 0;
                    Signal::Hold
                } else if self.excursion_lock { Signal::Hold }
                else if z > self.z_threshold { self.internal_state = -1; Signal::Sell }
                else if z < -self.z_threshold { self.internal_state = 1; Signal::Buy }
                else { Signal::Hold }
            }
        };

        let mut final_signal = signal_result;
        let mut slippage = 0.0;
        
        if signal_result != Signal::Hold && signal_result != Signal::Close {
            if q <= 0.5 { // Passive attempt
                let fill_prob = if p_toxic < 0.3 { 0.4 } else { 0.85 };
                if rand::random::<f64>() > fill_prob {
                    final_signal = Signal::Hold;
                    self.internal_state = 0; // REVERT STATE
                    self.excursion_lock = true;
                } else if p_toxic >= 0.3 {
                    slippage = 0.0001;
                }
            } else { // Aggressive attempt
                slippage = 0.00015;
            }
        }
        
        self.prev_innovation = innovation;
        Action { signal: final_signal, size: max_shares, execution_slippage: slippage }
    }
}
