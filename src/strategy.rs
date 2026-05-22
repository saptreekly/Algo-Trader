#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Signal {
    Buy,
    Sell,
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
        price_a: f64,
        price_b: f64,
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
    r_noise: f64,
    z_threshold: f64,
    loss_toxic: f64,
    rolling_size_a: f64,
    rolling_size_b: f64,
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
            r_noise,
            z_threshold,
            loss_toxic,
            rolling_size_a: initial_vol,
            rolling_size_b: initial_vol,
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
        Self::with_parameters(0.01, 2.0, 0.05, 100.0, 0.1, 0.99)
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
        account_balance: f64,
    ) -> Action {
        self.tick_count += 1;

        let available_buying_power = account_balance * 2.0;
        let max_shares = (available_buying_power / 2.0) / (price_a + price_b);

        if max_shares < 1.0 {
            self.internal_state = 0;
            self.prev_innovation = 0.0;
            return Action { signal: Signal::Hold, size: 0.0, execution_slippage: 0.0 };
        }

        // 1. Kalman Filter Update
// ... (rest of implementation)
// 1. Kalman Filter Update
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

            let hypo_pnl = (cand_signal as f64) * delta_innovation;
            let active_pnl = (current_signal as f64) * delta_innovation;

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

        // Update Dynamic Volume Baselines (EWMA)
        if avg_size_a > 0.0 {
            self.rolling_size_a = (self.rolling_size_a * 0.99) + (avg_size_a * 0.01);
        }
        if avg_size_b > 0.0 {
            self.rolling_size_b = (self.rolling_size_b * 0.99) + (avg_size_b * 0.01);
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
        // innovation > 0: Shorting the spread (Short Squeeze risk -> higher toxic loss)
        // innovation < 0: Buying the spread (Panic liquidation -> lower toxic loss)
        let toxic_multiplier = if innovation > 0.0 { 1.5 } else { 0.8 };
        let effective_loss_toxic = self.loss_toxic * toxic_multiplier;

        let friction_cost = 0.00015;
        let passive_friction = friction_cost * 0.5;

        let payoff_agg_noise = live_payoff - friction_cost;
        let payoff_agg_toxic = (live_payoff * 0.2) - (effective_loss_toxic * 0.5) - friction_cost;
        let payoff_pass_noise = (live_payoff * 0.8) - passive_friction;
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

        let mut final_signal = signal_result;
        let mut slippage = 0.0;
        
        if signal_result != Signal::Hold {
            if q <= 0.5 { // Passive attempt
                let fill_prob = if p_toxic < 0.3 { 0.4 } else { 0.85 };
                if rand::random::<f64>() > fill_prob {
                    println!("Passive fill missed for {:?}", signal_result);
                    final_signal = Signal::Hold;
                    self.internal_state = 0; // REVERT STATE
                } else if p_toxic >= 0.3 {
                    println!("Passive fill achieved with adverse selection penalty (1bps)");
                    slippage = 0.0001;
                }
            } else { // Aggressive attempt
                slippage = 0.00015;
            }
        }

        if final_signal != Signal::Hold {
            let action_type = if q > 0.5 { "Aggressive Market Fill" } else { "Passive Limit Fill" };
            println!("Engine recommends: {} for {:?} | Size: {:.2} | Slippage: {:.5}", action_type, final_signal, max_shares, slippage);
        }
        
        self.prev_innovation = innovation;
        Action { signal: final_signal, size: max_shares, execution_slippage: slippage }
    }
}
