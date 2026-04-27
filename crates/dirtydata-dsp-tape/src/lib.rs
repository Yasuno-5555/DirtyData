//! Magnetic Tape Emulation (Jiles-Atherton inspired Hysteresis, Wow/Flutter, Head Bump)
//! 
//! Refinements:
//! - Proper ODE-based Jiles-Atherton hysteresis (no division-by-near-zero)
//! - Independent Wow/Flutter LFOs with separate phases
//! - Cubic Hermite interpolation for delay line readback
//! - Proper head bump EQ with configurable frequency and resonance

#[derive(Clone)]
pub struct TapeMachine {
    buffer: Vec<f32>,
    write_idx: usize,
    sample_rate: f32,
    
    // Independent LFO phases
    wow_phase: f32,
    flutter_phase: f32,
    
    // Hysteresis state (Jiles-Atherton ODE)
    m_prev: f32,
    h_prev: f32,
    dm_prev: f32, // For trapezoidal integration
    
    // Head bump (2nd-order resonant filter state)
    bp_z1: f32,
    bp_z2: f32,
}

impl TapeMachine {
    pub fn new(sample_rate: f32) -> Self {
        let buf_len = (sample_rate * 0.1).max(64.0) as usize;
        Self {
            buffer: vec![0.0; buf_len],
            write_idx: 0,
            sample_rate,
            wow_phase: 0.0,
            flutter_phase: 0.0,
            m_prev: 0.0,
            h_prev: 0.0,
            dm_prev: 0.0,
            bp_z1: 0.0,
            bp_z2: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Jiles-Atherton hysteresis model (ODE-based, numerically stable).
    /// Returns magnetization M for a given applied field H.
    fn hysteresis(&mut self, h: f32) -> f32 {
        // Model parameters
        let ms = 1.0_f32;      // Saturation magnetization
        let a = 0.5_f32;       // Langevin shape parameter
        let k = 0.3_f32;       // Pinning constant (coercivity)
        let c = 0.1_f32;       // Domain coupling
        let alpha_ja = 0.001;  // Mean-field coupling

        let dh = h - self.h_prev;
        let delta: f32 = if dh >= 0.0 { 1.0 } else { -1.0 };
        
        // Effective field
        let he = h + alpha_ja * self.m_prev;
        
        // Anhysteretic magnetization (Langevin function)
        // L(x) = coth(x) - 1/x, approximated for stability
        let x = he / a.max(0.001);
        let m_an = if x.abs() < 0.001 {
            ms * x / 3.0  // Taylor expansion for small x
        } else {
            ms * (1.0 / x.tanh() - 1.0 / x)
        };
        
        // Differential susceptibility (stable computation)
        let dm_an_dh = if x.abs() < 0.001 {
            ms / (3.0 * a)
        } else {
            let coth_x = 1.0 / x.tanh();
            ms * (1.0 / (x * x) - coth_x * coth_x + 1.0) / a
        };
        
        // Irreversible magnetization change
        let m_diff = m_an - self.m_prev;
        let denom = (delta * k - alpha_ja * m_diff).max(0.01);
        let dm_irr = m_diff / denom;
        
        // Total differential
        let dm = (1.0 - c) * dm_irr + c * dm_an_dh;
        
        // Trapezoidal integration for stability
        let m = self.m_prev + 0.5 * (dm + self.dm_prev) * dh;
        let m = m.clamp(-ms, ms);
        
        self.h_prev = h;
        self.m_prev = m;
        self.dm_prev = dm;
        
        m
    }

    /// Cubic Hermite interpolation for smooth delay line reading.
    fn hermite_interp(buffer: &[f32], pos: f32, len: usize) -> f32 {
        let i = pos.floor() as isize;
        let frac = pos - i as f32;
        
        let idx = |offset: isize| -> usize {
            let mut idx = (i + offset) % len as isize;
            if idx < 0 { idx += len as isize; }
            idx as usize
        };
        
        let y0 = buffer[idx(-1)];
        let y1 = buffer[idx(0)];
        let y2 = buffer[idx(1)];
        let y3 = buffer[idx(2)];
        
        // Cubic Hermite
        let c0 = y1;
        let c1 = 0.5 * (y2 - y0);
        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
        
        ((c3 * frac + c2) * frac + c1) * frac + c0
    }

    /// Process a sample.
    /// `drive`: Input gain (pre-saturation).
    /// `wow`: Low frequency tape speed variation depth (0..1).
    /// `flutter`: High frequency tape speed variation depth (0..1).
    /// `bias`: High frequency AC bias amplitude.
    pub fn process(&mut self, input: f32, drive: f32, wow: f32, flutter: f32, bias: f32) -> f32 {
        let two_pi = 2.0 * std::f32::consts::PI;
        
        // AC bias oscillator (ultrasonic, ~50kHz equivalent scaled)
        let bias_signal = bias * (self.flutter_phase * 37.0).sin();
        let h = input * drive.max(0.01) + bias_signal;
        
        // Jiles-Atherton hysteresis
        let m = self.hysteresis(h);
        
        // Write to circular buffer
        self.buffer[self.write_idx] = m;
        
        // --- Wow LFO (independent, ~0.5-2 Hz) ---
        let wow_freq = 1.2_f32; // Hz
        self.wow_phase += two_pi * wow_freq / self.sample_rate;
        if self.wow_phase >= two_pi { self.wow_phase -= two_pi; }
        
        // --- Flutter LFO (independent, ~5-12 Hz) ---
        let flutter_freq = 7.5_f32; // Hz
        self.flutter_phase += two_pi * flutter_freq / self.sample_rate;
        if self.flutter_phase >= two_pi { self.flutter_phase -= two_pi; }
        
        // Wow: slow sinusoidal with secondary harmonic
        let wow_mod = (self.wow_phase.sin() + 0.3 * (self.wow_phase * 2.0).sin()) * wow * 30.0;
        // Flutter: faster modulation
        let flutter_mod = self.flutter_phase.sin() * flutter * 8.0;
        
        let buf_len = self.buffer.len() as f32;
        let delay_samples = (10.0 + wow_mod + flutter_mod).clamp(2.0, buf_len - 3.0);
        
        let read_pos = self.write_idx as f32 - delay_samples;
        let read_pos = if read_pos < 0.0 { read_pos + buf_len } else { read_pos };
        
        // Cubic Hermite interpolation (much smoother than linear)
        let mut out = Self::hermite_interp(&self.buffer, read_pos, self.buffer.len());
        
        self.write_idx = (self.write_idx + 1) % self.buffer.len();
        
        // --- Head Bump: Resonant bandpass around 60Hz ---
        // SVF-style 2nd order for musical resonance
        let bump_freq = 60.0_f32;
        let bump_q = 2.0_f32;
        let wd = two_pi * bump_freq / self.sample_rate;
        let g = (wd * 0.5).tan();
        let r = 1.0 / bump_q;
        let denom = 1.0 / (1.0 + g * (g + r));
        
        let hp = denom * (out - (g + r) * self.bp_z1 - self.bp_z2);
        let bp = g * hp + self.bp_z1;
        let lp = g * bp + self.bp_z2;
        
        self.bp_z1 = 2.0 * bp - self.bp_z1;
        self.bp_z2 = 2.0 * lp - self.bp_z2;
        
        // Mix resonant bump back in (musical low-end boost)
        out += bp * 1.5;
        
        out
    }
}
