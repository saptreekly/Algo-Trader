pub struct Transition {
    pub state: Vec<f32>,
    pub action: usize,
    pub reward: f32,
    pub next_state: Vec<f32>,
    pub done: bool,
}

pub struct ReplayBuffer {
    pub buffer: Vec<Transition>,
}

impl ReplayBuffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }
    pub fn push(&mut self, t: Transition) {
        self.buffer.push(t);
    }
}
