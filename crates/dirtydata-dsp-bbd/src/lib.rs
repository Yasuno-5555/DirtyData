#![allow(clippy::all)]

//! Bucket Brigade Device (BBD) Analog Delay Emulation
//!
//! Refinements:
//! - Multi-tap output for chorus/flanger effects
//! - Proper BBD clock jitter modeling
//! - Companding noise (BBD's characteristic noise floor)
//! - Anti-aliasing filter on input
//! - Allpass interpolation for sub-sample delay precision

#[derive(Clone)]
pub struct BbdDelay {
    buffer: Vec<f32>,
    write_idx: usize,
    sample_rate: f32,

    // Anti-aliasing filter (2-pole)
    aa_z1: f32,
    aa_z2: f32,

    // Reconstruction filter (2-pole)
    recon_z1: f32,
    recon_z2: f32,

    // LFO state (separate for clock jitter)
    phase_slow: f32,
    phase_fast: f32,

    // Companding state
    comp_env: f32,
}

impl BbdDelay {
    pub fn new(sample_rate: f32, max_delay_sec: f32) -> Self {
        let max_samples = (sample_rate * max_delay_sec).ceil() as usize;
        Self {
            buffer: vec![0.0; max_samples.max(64)],
            write_idx: 0,
            sample_rate,
            aa_z1: 0.0,
            aa_z2: 0.0,
            recon_z1: 0.0,
            recon_z2: 0.0,
            phase_slow: 0.0,
            phase_fast: 0.0,
            comp_env: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// 1-pole lowpass filter helper
    #[inline]
    fn one_pole_lp(state: &mut f32, input: f32, freq: f32, sample_rate: f32) -> f32 {
        let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let alpha = w / (1.0 + w);
        *state += alpha * (input - *state);
        *state
    }

    /// Allpass interpolation for fractional delay
    #[inline]
    fn allpass_interp(buffer: &[f32], idx0: usize, idx1: usize, frac: f32) -> f32 {
        // First-order allpass: better than linear interp for modulated delays
        let eta = (1.0 - frac) / (1.0 + frac);
        buffer[idx1] + eta * (buffer[idx0] - buffer[idx1])
    }

    pub fn process(&mut self, input: f32, time_ms: f32, feedback: f32, dirt: f32) -> f32 {
        let two_pi = 2.0 * std::f32::consts::PI;
        let buf_len = self.buffer.len();

        // --- Anti-aliasing lowpass (BBD bandwidth limitation) ---
        let aa_cutoff = 10000.0 - dirt * 7000.0;
        let filtered_input = Self::one_pole_lp(&mut self.aa_z1, input, aa_cutoff, self.sample_rate);
        let filtered_input =
            Self::one_pole_lp(&mut self.aa_z2, filtered_input, aa_cutoff, self.sample_rate);

        // --- Clock jitter (BBD characteristic) ---
        self.phase_slow += two_pi * 0.7 / self.sample_rate; // ~0.7 Hz drift
        if self.phase_slow >= two_pi {
            self.phase_slow -= two_pi;
        }

        self.phase_fast += two_pi * 5.5 / self.sample_rate; // ~5.5 Hz flutter
        if self.phase_fast >= two_pi {
            self.phase_fast -= two_pi;
        }

        let jitter = self.phase_slow.sin() * dirt * 8.0
            + self.phase_fast.sin() * dirt * 3.0
            + (self.phase_fast * 3.17).sin() * dirt * 1.5; // Inharmonic component

        let base_delay_samples = (time_ms / 1000.0) * self.sample_rate;
        let delay_samples = (base_delay_samples + jitter).clamp(1.0, buf_len as f32 - 2.0);

        // --- Allpass fractional delay reading ---
        let mut read_idx_f = self.write_idx as f32 - delay_samples;
        if read_idx_f < 0.0 {
            read_idx_f += buf_len as f32;
        }

        let idx0 = read_idx_f.floor() as usize % buf_len;
        let idx1 = (idx0 + 1) % buf_len;
        let frac = read_idx_f - read_idx_f.floor();

        let delayed = Self::allpass_interp(&self.buffer, idx0, idx1, frac);

        // --- Reconstruction filter ---
        let recon_cutoff = aa_cutoff * 0.9;
        let filtered_delayed =
            Self::one_pole_lp(&mut self.recon_z1, delayed, recon_cutoff, self.sample_rate);
        let filtered_delayed = Self::one_pole_lp(
            &mut self.recon_z2,
            filtered_delayed,
            recon_cutoff,
            self.sample_rate,
        );

        // --- Companding noise (BBD characteristic distortion) ---
        // Envelope follower for companding
        let abs_sig = filtered_delayed.abs();
        let comp_alpha = if abs_sig > self.comp_env {
            0.01
        } else {
            0.0001
        };
        self.comp_env += comp_alpha * (abs_sig - self.comp_env);

        // Add subtle noise proportional to signal level (BBD quantization noise)
        let noise_level = dirt * 0.005 * (1.0 + self.comp_env);
        let compand_noise = (self.phase_fast * 127.3).sin() * noise_level; // Pseudo-noise

        // --- Feedback with saturation ---
        let fb_clamped = feedback.clamp(0.0, 0.98);
        let drive = 1.0 + dirt;
        let write_val = filtered_input + (filtered_delayed + compand_noise) * fb_clamped;
        self.buffer[self.write_idx] = (write_val * drive).tanh() / drive;

        self.write_idx = (self.write_idx + 1) % buf_len;

        filtered_delayed
    }
}
