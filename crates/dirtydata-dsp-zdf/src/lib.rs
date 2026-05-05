#![allow(clippy::all)]
#![allow(clippy::all)]

//! Zero-Delay Feedback (ZDF) 4-pole Transistor Ladder Filter
//!
//! Refinements:
//! - Proper frequency clamping to Nyquist
//! - Rust naming conventions (no uppercase variable names)
//! - Thermal saturation per-stage (not just input)
//! - Multi-output (1/2/3/4 pole taps)

#[derive(Clone)]
pub struct ZdfLadder {
    s: [f32; 4],
    sample_rate: f32,
}

#[derive(Clone)]
pub struct LadderOutput {
    pub lp6: f32,  // 1-pole (6 dB/oct)
    pub lp12: f32, // 2-pole (12 dB/oct)
    pub lp18: f32, // 3-pole (18 dB/oct)
    pub lp24: f32, // 4-pole (24 dB/oct)
}

impl ZdfLadder {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            s: [0.0; 4],
            sample_rate,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn reset(&mut self) {
        self.s = [0.0; 4];
    }

    /// Process a single sample with multi-tap output.
    /// `cutoff_hz`: Filter cutoff frequency.
    /// `resonance`: 0.0 to 1.0 (self-oscillates at higher values).
    /// `drive`: Input overdrive for thermal saturation modeling.
    pub fn process_multi(
        &mut self,
        input: f32,
        cutoff_hz: f32,
        resonance: f32,
        drive: f32,
    ) -> LadderOutput {
        // Clamp cutoff to safe range
        let max_freq = self.sample_rate * 0.49;
        let cutoff = cutoff_hz.clamp(10.0, max_freq);

        let wd = 2.0 * std::f32::consts::PI * cutoff;
        let t = 1.0 / self.sample_rate;
        let wa = (2.0 / t) * (wd * t / 2.0).tan(); // Pre-warping
        let g = wa * t / 2.0;
        let g_plus_1 = 1.0 + g;

        let g_val = g / g_plus_1;
        let g2 = g_val * g_val;
        let g3 = g2 * g_val;
        let g4 = g3 * g_val;

        // Compute feedback signal from integrator states
        let s0 = self.s[0] / g_plus_1;
        let s1 = self.s[1] / g_plus_1;
        let s2 = self.s[2] / g_plus_1;
        let s3 = self.s[3] / g_plus_1;

        let sigma = g3 * s0 + g2 * s1 + g_val * s2 + s3;

        // Moog ladder resonance scaling (4 = self-oscillation)
        let k = resonance.clamp(0.0, 1.0) * 4.0;

        // Drive saturation (tanh)
        let driven_input = (input * (1.0 + drive)).tanh();

        // Solve for zero-delay feedback loop
        let u = (driven_input - k * sigma) / (1.0 + k * g4);

        // Clip feedback to simulate real-world op-amp limits
        let u = u.clamp(-5.0, 5.0);

        // Compute 4 1-pole lowpass stages with per-stage soft saturation
        let mut v;
        let mut lp = [0.0_f32; 4];

        // Stage 1
        v = (u - self.s[0]) * g_val;
        lp[0] = v + self.s[0];
        self.s[0] = lp[0] + v;

        // Stage 2 (with subtle saturation)
        let input_2 = if drive > 0.01 {
            (lp[0] * (1.0 + drive * 0.25)).tanh()
        } else {
            lp[0]
        };
        v = (input_2 - self.s[1]) * g_val;
        lp[1] = v + self.s[1];
        self.s[1] = lp[1] + v;

        // Stage 3
        let input_3 = if drive > 0.01 {
            (lp[1] * (1.0 + drive * 0.25)).tanh()
        } else {
            lp[1]
        };
        v = (input_3 - self.s[2]) * g_val;
        lp[2] = v + self.s[2];
        self.s[2] = lp[2] + v;

        // Stage 4
        let input_4 = if drive > 0.01 {
            (lp[2] * (1.0 + drive * 0.25)).tanh()
        } else {
            lp[2]
        };
        v = (input_4 - self.s[3]) * g_val;
        lp[3] = v + self.s[3];
        self.s[3] = lp[3] + v;

        LadderOutput {
            lp6: lp[0],
            lp12: lp[1],
            lp18: lp[2],
            lp24: lp[3],
        }
    }

    /// Original single-output interface (backwards compatible).
    pub fn process(&mut self, input: f32, cutoff_hz: f32, resonance: f32, drive: f32) -> f32 {
        self.process_multi(input, cutoff_hz, resonance, drive).lp24
    }
}
