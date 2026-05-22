pub struct DQNAgent<M> {
    #[allow(dead_code)]
    pub model: M,
}

impl<M> DQNAgent<M> {
    pub fn new(model: M) -> Self {
        Self { model }
    }

    pub fn act(&self, _state: &[f32]) -> usize {
        0
    }
}
