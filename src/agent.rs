use crate::model::QNetwork;

pub struct DQNAgent<M> {
    pub model: M,
}

impl<M> DQNAgent<M> {
    pub fn new(model: M) -> Self {
        Self { model }
    }

    pub fn act(&self, _state: &Vec<f32>) -> usize {
        0
    }

    pub fn train_step(&mut self) {
    }
}
