use dfdx::prelude::*;

pub type QNetwork = (
    Linear<8, 64>,
    ReLU,
    Linear<64, 32>,
    ReLU,
    Linear<32, 3>,
);
