//! Spectral Civilization

use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;

#[derive(Clone)]
pub struct SpectralGate {
    size: usize,
    fft: Arc<dyn rustfft::Fft<f32>>,
    ifft: Arc<dyn rustfft::Fft<f32>>,
}

impl SpectralGate {
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::new();
        Self {
            size,
            fft: planner.plan_fft_forward(size),
            ifft: planner.plan_fft_inverse(size),
        }
    }

    pub fn process(&mut self, samples: &mut [f32], threshold: f32) {
        let mut complex_samples: Vec<Complex<f32>> = samples.iter().map(|&s| Complex::new(s, 0.0)).collect();
        if complex_samples.len() < self.size { complex_samples.resize(self.size, Complex::default()); }
        
        self.fft.process(&mut complex_samples);
        
        for bin in complex_samples.iter_mut() {
            let mag = bin.norm();
            if mag < threshold {
                *bin = Complex::default();
            }
        }
        
        self.ifft.process(&mut complex_samples);
        
        for (s, c) in samples.iter_mut().zip(complex_samples.iter()) {
            *s = c.re / self.size as f32;
        }
    }
}
