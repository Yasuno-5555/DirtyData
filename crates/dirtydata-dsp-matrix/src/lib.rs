//! Matrix Mixing and Routing

#[derive(Clone)]
pub struct MatrixMixer {
    num_in: usize,
    num_out: usize,
    gains: Vec<f32>, // Flat MxN matrix
}

impl MatrixMixer {
    pub fn new(num_in: usize, num_out: usize) -> Self {
        Self {
            num_in,
            num_out,
            gains: vec![0.0; num_in * num_out],
        }
    }

    pub fn set_gain(&mut self, input: usize, output: usize, gain: f32) {
        if input < self.num_in && output < self.num_out {
            self.gains[output * self.num_in + input] = gain;
        }
    }

    pub fn process(&self, inputs: &[f32], outputs: &mut [f32]) {
        for (o_idx, out) in outputs.iter_mut().enumerate().take(self.num_out) {
            let mut sum = 0.0;
            for (i_idx, &val) in inputs.iter().enumerate().take(self.num_in) {
                sum += val * self.gains[o_idx * self.num_in + i_idx];
            }
            *out = sum;
        }
    }
}

#[derive(Clone)]
pub struct MorphMixer {
    pub x: f32,
    pub y: f32,
}

impl MorphMixer {
    pub fn new() -> Self {
        Self { x: 0.5, y: 0.5 }
    }
}

impl Default for MorphMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl MorphMixer {
    pub fn process(&self, a: f32, b: f32, c: f32, d: f32) -> f32 {
        let top = a * (1.0 - self.x) + b * self.x;
        let bottom = c * (1.0 - self.x) + d * self.x;
        top * (1.0 - self.y) + bottom * self.y
    }
}
