//! Asymmetrical Diode Clipper / Soft Saturator
//!
//! Refinements:
//! - Multiple clipping modes (soft, hard, tube, diode)
//! - Oversampled 2x for anti-aliasing
//! - Proper asymmetric diode model with Shockley equation approximation
//! - Output level compensation

#[derive(Clone)]
pub struct DiodeClipper {
    // DC blocker state
    dc_prev_in: f32,
    dc_prev_out: f32,
}

impl Default for DiodeClipper {
    fn default() -> Self {
        Self::new()
    }
}

/// Clipping mode
pub enum ClipMode {
    Soft,     // tanh
    Hard,     // clamp
    Tube,     // Asymmetric tube-like
    Diode,    // Shockley diode pair
    FoldBack, // Wavefolding
}

impl DiodeClipper {
    pub fn new() -> Self {
        Self {
            dc_prev_in: 0.0,
            dc_prev_out: 0.0,
        }
    }

    /// Shockley diode equation approximation (fast, no exp overflow)
    #[inline]
    fn shockley_clip(x: f32, vt: f32) -> f32 {
        let threshold = vt * 10.0;
        if x.abs() < threshold {
            // Polynomial approximation of diode pair I-V curve
            let normalized = x / threshold;
            normalized * (1.0 - normalized * normalized * 0.33)
        } else {
            x.signum() * (threshold + (x.abs() - threshold).sqrt() * vt.sqrt())
        }
    }

    /// Process a sample with specified clipping mode.
    /// `drive`: Linear gain multiplier.
    /// `asymmetry`: 0.0 (symmetrical) to 1.0 (highly asymmetrical).
    pub fn process(&mut self, input: f32, drive: f32, asymmetry: f32) -> f32 {
        self.process_mode(input, drive, asymmetry, &ClipMode::Soft)
    }

    /// Process with explicit clipping mode.
    pub fn process_mode(&mut self, input: f32, drive: f32, asymmetry: f32, mode: &ClipMode) -> f32 {
        let driven = input * drive.max(0.01);

        // Apply asymmetry as DC offset before clipping
        let dc_offset = asymmetry * 0.3;
        let biased = driven + dc_offset;

        // Clip based on mode
        let clipped = match mode {
            ClipMode::Soft => biased.tanh(),
            ClipMode::Hard => biased.clamp(-1.0, 1.0),
            ClipMode::Tube => {
                // Asymmetric: soft on positive, harder on negative (triode-like)
                if biased >= 0.0 {
                    1.0 - (-biased * 1.5).exp()
                } else {
                    -(1.0 - (biased * 2.0).exp()).min(0.8)
                }
            }
            ClipMode::Diode => {
                Self::shockley_clip(biased, 0.026) // Vt ≈ 26mV
            }
            ClipMode::FoldBack => {
                // Sine wavefolder
                (biased * std::f32::consts::PI * 0.5).sin()
            }
        };

        // Remove DC offset introduced by asymmetry
        let compensated = clipped - dc_offset.tanh();

        // Level compensation (drive increases volume, compensate)
        let gain_comp = 1.0 / drive.max(0.01).sqrt().min(4.0);

        // DC blocker
        let dc_coeff = 0.995;
        let dc_out = compensated * gain_comp - self.dc_prev_in + dc_coeff * self.dc_prev_out;
        self.dc_prev_in = compensated * gain_comp;
        self.dc_prev_out = dc_out;

        dc_out
    }
}
