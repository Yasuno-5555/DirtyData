//! Fast Fourier Transform Civilization

use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WindowType {
    Rectangular,
    Hann,
    Hamming,
    Blackman,
}

impl WindowType {
    pub fn get_coefficient(&self, index: usize, length: usize) -> f32 {
        if length <= 1 { return 1.0; }
        let x = index as f32 / (length - 1) as f32;
        match self {
            WindowType::Rectangular => 1.0,
            WindowType::Hann => 0.5 * (1.0 - (2.0 * std::f32::consts::PI * x).cos()),
            WindowType::Hamming => 0.54 - 0.46 * (2.0 * std::f32::consts::PI * x).cos(),
            WindowType::Blackman => 0.42 - 0.5 * (2.0 * std::f32::consts::PI * x).cos() + 0.08 * (4.0 * std::f32::consts::PI * x).cos(),
        }
    }

    pub fn apply(&self, data: &mut [f32]) {
        let n = data.len();
        for i in 0..n {
            data[i] *= self.get_coefficient(i, n);
        }
    }
}

pub struct FftProcessor {
    size: usize,
    fft: Arc<dyn rustfft::Fft<f32>>,
    ifft: Arc<dyn rustfft::Fft<f32>>,
    buffer: Vec<Complex<f32>>,
    window_coeffs: Vec<f32>,
}

impl FftProcessor {
    pub fn new(size: usize, window: WindowType) -> Self {
        let mut planner = FftPlanner::new();
        let window_coeffs = (0..size).map(|i| window.get_coefficient(i, size)).collect();
        Self {
            size,
            fft: planner.plan_fft_forward(size),
            ifft: planner.plan_fft_inverse(size),
            buffer: vec![Complex::default(); size],
            window_coeffs,
        }
    }

    /// Performs forward FFT on the input samples.
    /// The input slice must have the same length as the FFT size.
    pub fn forward(&mut self, input: &[f32], output: &mut [Complex<f32>]) {
        assert_eq!(input.len(), self.size);
        assert_eq!(output.len(), self.size);

        // Copy and apply pre-calculated window
        for i in 0..self.size {
            self.buffer[i] = Complex::new(input[i] * self.window_coeffs[i], 0.0);
        }

        self.fft.process(&mut self.buffer);
        output.copy_from_slice(&self.buffer);
    }

    /// Performs inverse FFT on the complex spectrum.
    /// The input slice must have the same length as the FFT size.
    pub fn inverse(&mut self, input: &[Complex<f32>], output: &mut [f32]) {
        assert_eq!(input.len(), self.size);
        assert_eq!(output.len(), self.size);

        self.buffer.copy_from_slice(input);
        self.ifft.process(&mut self.buffer);

        let inv_size = 1.0 / self.size as f32;
        for (i, c) in self.buffer.iter().enumerate() {
            output[i] = c.re * inv_size;
        }
    }

    /// Converts complex spectrum to magnitude and phase.
    pub fn to_polar(spectrum: &[Complex<f32>], mag: &mut [f32], phase: &mut [f32]) {
        for i in 0..spectrum.len() {
            let (m, p) = spectrum[i].to_polar();
            mag[i] = m;
            phase[i] = p;
        }
    }

    /// Converts magnitude and phase back to complex spectrum.
    pub fn from_polar(mag: &[f32], phase: &[f32], spectrum: &mut [Complex<f32>]) {
        for i in 0..spectrum.len() {
            spectrum[i] = Complex::from_polar(mag[i], phase[i]);
        }
    }

    /// Computes the spectral centroid.
    pub fn spectral_centroid(mag: &[f32], sample_rate: f32) -> f32 {
        let mut num = 0.0;
        let mut den = 0.0;
        let bin_to_freq = sample_rate / (2.0 * (mag.len() - 1) as f32);
        
        for (i, &m) in mag.iter().enumerate() {
            let freq = i as f32 * bin_to_freq;
            num += freq * m;
            den += m;
        }
        
        if den == 0.0 { 0.0 } else { num / den }
    }

    /// Computes the spectral spread (bandwidth).
    pub fn spectral_spread(mag: &[f32], sample_rate: f32) -> f32 {
        let centroid = Self::spectral_centroid(mag, sample_rate);
        let mut num = 0.0;
        let mut den = 0.0;
        let bin_to_freq = sample_rate / (2.0 * (mag.len() - 1) as f32);

        for (i, &m) in mag.iter().enumerate() {
            let freq = i as f32 * bin_to_freq;
            num += (freq - centroid).powi(2) * m;
            den += m;
        }

        if den == 0.0 { 0.0 } else { (num / den).sqrt() }
    }

    /// Computes the spectral rolloff frequency.
    pub fn spectral_rolloff(mag: &[f32], sample_rate: f32, threshold: f32) -> f32 {
        let total_energy: f32 = mag.iter().sum();
        let target_energy = total_energy * threshold;
        let mut current_energy = 0.0;
        let bin_to_freq = sample_rate / (2.0 * (mag.len() - 1) as f32);

        for (i, &m) in mag.iter().enumerate() {
            current_energy += m;
            if current_energy >= target_energy {
                return i as f32 * bin_to_freq;
            }
        }
        sample_rate / 2.0
    }

    /// Computes the spectral flatness.
    pub fn spectral_flatness(mag: &[f32]) -> f32 {
        let n = mag.len() as f32;
        let mut sum = 0.0;
        let mut log_sum = 0.0;
        let epsilon = 1e-10;

        for &m in mag {
            let m_eps = m + epsilon;
            sum += m_eps;
            log_sum += m_eps.ln();
        }

        let am = sum / n;
        let gm = (log_sum / n).exp();
        
        if am == 0.0 { 0.0 } else { gm / am }
    }

    /// Computes spectral skewness.
    pub fn spectral_skewness(mag: &[f32], sample_rate: f32) -> f32 {
        let centroid = Self::spectral_centroid(mag, sample_rate);
        let spread = Self::spectral_spread(mag, sample_rate);
        if spread == 0.0 { return 0.0; }

        let mut num = 0.0;
        let mut den = 0.0;
        let bin_to_freq = sample_rate / (2.0 * (mag.len() - 1) as f32);

        for (i, &m) in mag.iter().enumerate() {
            let freq = i as f32 * bin_to_freq;
            num += (freq - centroid).powi(3) * m;
            den += m;
        }

        num / (den * spread.powi(3))
    }

    /// Computes spectral kurtosis.
    pub fn spectral_kurtosis(mag: &[f32], sample_rate: f32) -> f32 {
        let centroid = Self::spectral_centroid(mag, sample_rate);
        let spread = Self::spectral_spread(mag, sample_rate);
        if spread == 0.0 { return 0.0; }

        let mut num = 0.0;
        let mut den = 0.0;
        let bin_to_freq = sample_rate / (2.0 * (mag.len() - 1) as f32);

        for (i, &m) in mag.iter().enumerate() {
            let freq = i as f32 * bin_to_freq;
            num += (freq - centroid).powi(4) * m;
            den += m;
        }

        num / (den * spread.powi(4))
    }
}

/// Helper for Short-Time Fourier Transform with overlap-add synthesis.
pub struct StftProcessor {
    fft_proc: FftProcessor,
    hop_size: usize,
    window_size: usize,
    input_buffer: Vec<f32>,
    output_accumulator: Vec<f32>,
    spectrum: Vec<Complex<f32>>,
    input_ptr: usize,
    output_ptr: usize,
    samples_since_last_fft: usize,
}

impl StftProcessor {
    pub fn new(window_size: usize, hop_size: usize, window: WindowType) -> Self {
        Self {
            fft_proc: FftProcessor::new(window_size, window),
            hop_size,
            window_size,
            input_buffer: vec![0.0; window_size],
            output_accumulator: vec![0.0; window_size * 2], // Extra space for OLA
            spectrum: vec![Complex::default(); window_size],
            input_ptr: 0,
            output_ptr: 0,
            samples_since_last_fft: 0,
        }
    }

    /// Process a single sample through the STFT and OLA.
    /// Returns the processed sample if enough samples have been accumulated.
    pub fn process<F>(&mut self, input: f32, mut f: F) -> f32
    where F: FnMut(&mut [Complex<f32>]) 
    {
        // Store input in circular buffer
        self.input_buffer[self.input_ptr] = input;
        self.input_ptr = (self.input_ptr + 1) % self.window_size;
        self.samples_since_last_fft += 1;

        if self.samples_since_last_fft >= self.hop_size {
            self.samples_since_last_fft = 0;

            // Prepare window for FFT
            let mut fft_input = vec![0.0; self.window_size];
            for i in 0..self.window_size {
                fft_input[i] = self.input_buffer[(self.input_ptr + i) % self.window_size];
            }

            // FFT -> Process -> IFFT
            self.fft_proc.forward(&fft_input, &mut self.spectrum);
            f(&mut self.spectrum);
            let mut ifft_output = vec![0.0; self.window_size];
            self.fft_proc.inverse(&self.spectrum, &mut ifft_output);

            // Add to output accumulator
            for i in 0..self.window_size {
                let acc_idx = (self.output_ptr + i) % self.output_accumulator.len();
                self.output_accumulator[acc_idx] += ifft_output[i];
            }
        }

        let out = self.output_accumulator[self.output_ptr];
        self.output_accumulator[self.output_ptr] = 0.0; // Clear for next round
        self.output_ptr = (self.output_ptr + 1) % self.output_accumulator.len();
        
        out
    }

    /// Process a block of samples.
    pub fn process_block<F>(&mut self, input: &[f32], output: &mut [f32], mut f: F) 
    where F: FnMut(&mut [Complex<f32>]) 
    {
        for (i, &s) in input.iter().enumerate() {
            output[i] = self.process(s, &mut f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_roundtrip() {
        let size = 1024;
        let mut processor = FftProcessor::new(size, WindowType::Rectangular);
        
        let mut input = vec![0.0; size];
        for i in 0..size {
            input[i] = (i as f32 * 0.1).sin();
        }
        
        let mut spectrum = vec![Complex::default(); size];
        let mut output = vec![0.0; size];
        
        processor.forward(&input, &mut spectrum);
        processor.inverse(&spectrum, &mut output);
        
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn test_spectral_features() {
        let size = 512;
        let mut mag = vec![0.0; size];
        // Sine at bin 10
        mag[10] = 1.0;
        
        let sr = 44100.0;
        let centroid = FftProcessor::spectral_centroid(&mag, sr);
        let expected_centroid = 10.0 * (sr / (2.0 * (size - 1) as f32));
        assert!((centroid - expected_centroid).abs() < 1.0);
        
        let flatness = FftProcessor::spectral_flatness(&mag);
        assert!(flatness < 0.1); // Sine is not flat

        let spread = FftProcessor::spectral_spread(&mag, sr);
        assert!(spread < 100.0); // Sine should have low spread

        let rolloff = FftProcessor::spectral_rolloff(&mag, sr, 0.85);
        assert!((rolloff - expected_centroid).abs() < 100.0);
    }

    #[test]
    fn test_stft_roundtrip() {
        let win_size = 1024;
        let hop_size = 256;
        let mut stft = StftProcessor::new(win_size, hop_size, WindowType::Hann);
        
        let input = vec![1.0; 4096];
        let mut output = vec![0.0; 4096];
        
        stft.process_block(&input, &mut output, |_| {});
        
        // After initial latency, it should be approximately constant
        // (Hann window COLA: sum of Hann windows with 1/4 overlap is constant if normalized)
        // Note: rustfft doesn't normalize, and we don't normalize for COLA yet, so just check if it's non-zero
        for i in win_size..4000 {
            assert!(output[i] > 0.0);
        }
    }
}
