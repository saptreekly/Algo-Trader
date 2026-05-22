#[derive(Clone, Debug)]
pub struct Transition {
    pub state: [f32; 8],
    pub action: usize,
    pub reward: f32,
    pub next_state: [f32; 8],
    pub done: bool,
}

pub struct ReplayBuffer {
    pub buffer: Vec<Transition>,
    pub capacity: usize,
}

impl ReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { buffer: Vec::new(), capacity }
    }
    
    pub fn push(&mut self, t: Transition) {
        if self.buffer.len() >= self.capacity {
            self.buffer.remove(0);
        }
        self.buffer.push(t);
    }
}
