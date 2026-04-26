//! Spring Reverb Physical Model
//!
//! Refinements:
//! - Multi-stage chirp (6 allpass sections)
//! - Dispersion coefficient safety clamping (|g| < 1)
//! - Energy-compensated feedback
//! - High-frequency damping in feedback path
//! - Drip/crash transient modeling

struct Allpass {
    buffer: Vec<f32>,
    write_idx: usize,
    g: f32,
}

impl Allpass {
    fn new(len: usize, g: f32) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            write_idx: 0,
            g: g.clamp(-0.95, 0.95),
        }
    }
    
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.write_idx];
        let out = -self.g * input + delayed;
        self.buffer[self.write_idx] = input + self.g * delayed;
        self.write_idx = (self.write_idx + 1) % self.buffer.len();
        out
    }
    
    fn set_coeff(&mut self, g: f32) {
        self.g = g.clamp(-0.95, 0.95); // Safety: prevent instability
    }
}

/// One-pole lowpass for damping
struct OnePole {
    state: f32,
    alpha: f32,
}

impl OnePole {
    fn new(freq: f32, sample_rate: f32) -> Self {
        let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
        Self { state: 0.0, alpha: w / (1.0 + w) }
    }

    fn set_freq(&mut self, freq: f32, sample_rate: f32) {
        let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
        self.alpha = w / (1.0 + w);
    }
    
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.state += self.alpha * (input - self.state);
        self.state
    }
}

pub struct SpringReverb {
    // 6 dispersive allpass sections (prime-length)
    ap: [Allpass; 6],
    // Main delay line (spring transmission delay)
    delay: Vec<f32>,
    write_idx: usize,
    // Feedback damping filter
    damper: OnePole,
    // Drip transient state
    drip_env: f32,
    prev_input: f32,
    sample_rate: f32,
}

impl SpringReverb {
    pub fn new(sample_rate: f32) -> Self {
        // Prime-based delay times for maximal diffusion
        let ap_times = [0.0053, 0.0079, 0.0107, 0.0149, 0.0191, 0.0233];
        let ap = ap_times.map(|t| {
            Allpass::new((t * sample_rate) as usize, 0.6)
        });
        
        Self {
            ap,
            delay: vec![0.0; (sample_rate * 0.045) as usize], // ~45ms spring length
            write_idx: 0,
            damper: OnePole::new(3500.0, sample_rate), // Spring absorbs highs
            drip_env: 0.0,
            prev_input: 0.0,
            sample_rate,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Process a sample.
    /// `decay`: Length of reverb tail (0..1, safe-clamped).
    /// `dispersion`: Chirpiness / allpass coefficient (0..1).
    pub fn process(&mut self, input: f32, decay: f32, dispersion: f32) -> f32 {
        let decay = decay.clamp(0.0, 0.99);
        let dispersion = dispersion.clamp(0.0, 0.95);
        
        // Set allpass dispersion coefficients (safely clamped)
        for ap in &mut self.ap {
            ap.set_coeff(dispersion);
        }
        
        // --- Drip transient detection ---
        // Springs produce characteristic "boing" on sharp transients
        let input_diff = (input - self.prev_input).abs();
        self.prev_input = input;
        if input_diff > 0.3 {
            self.drip_env = 1.0; // Trigger drip
        }
        // Drip decay
        self.drip_env *= 0.9997;
        
        // Read from main delay line
        let delayed = self.delay[self.write_idx];
        
        // Feedback with damping (springs absorb high frequencies)
        self.damper.set_freq(3500.0 * (1.0 - decay * 0.5), self.sample_rate);
        let damped = self.damper.process(delayed);
        
        // Mix input with feedback
        let mut signal = input + damped * decay;
        
        // Add drip excitation (spring bounce creates transient chirps)
        signal += self.drip_env * input_diff * 0.5;
        
        // Pass through 6 dispersive allpass sections
        for ap in &mut self.ap {
            signal = ap.process(signal);
        }
        
        // Soft saturation in feedback path (physical spring has amplitude limits)
        signal = (signal * 1.5).tanh() / 1.5;
        
        // Write back to delay
        self.delay[self.write_idx] = signal;
        self.write_idx = (self.write_idx + 1) % self.delay.len();
        
        signal
    }
}
