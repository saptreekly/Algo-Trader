pub struct TradingEnvironment {
    pub current_step: usize,
    pub equity: f32,
    pub data: Vec<Vec<f32>>,
}

impl TradingEnvironment {
    pub fn new(data: Vec<Vec<f32>>) -> Self {
        Self {
            current_step: 0,
            equity: 10000.0,
            data,
        }
    }

    pub fn step(&mut self, _action: usize) -> (Vec<f32>, f32, bool) {
        let prev_equity = self.equity;
        self.current_step += 1;
        let is_done = self.current_step >= self.data.len() - 1;
        let reward = if is_done { 0.0 } else { (self.equity - prev_equity) - 0.01 };
        let next_state = if is_done { vec![0.0; 8] } else { self.data[self.current_step].clone() };
        (next_state, reward, is_done)
    }
}
