use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,

    z1: f32, z2: f32,
}

impl Biquad {
    pub fn new() -> Self {
        Self {
            b0: 1.0, b1: 0.0, b2: 0.0,
            a1: 0.0, a2: 0.0,
            z1: 0.0, z2: 0.0,
        }
    }

    pub fn set_lowpass(&mut self, cutoff: f32, q : f32, sample_rate: f32) {
        let theta = TAU * cutoff / sample_rate;
        let beta = 0.5 * (1.0 - q * theta.sin()) / (1.0 + q * theta.sin());
        let gamma = (0.5 + beta) * theta.cos();
        
        self.b0 = (0.5 + beta - gamma) / 2.0;
        self.b1 = 0.5 + beta - gamma;
        self.b2 = (0.5 + beta - gamma) / 2.0;
        self.a1 = -2.0 * gamma;
        self.a2 = 2.0 * beta;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * out + self.z2;
        self.z2 = self.b2 * input - self.a2 * out;
        out
    }
}