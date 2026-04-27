//! Topology-Preserving Transform (TPT) State Variable Filter
//!
//! Refinements:
//! - Proper cutoff frequency clamping to prevent instability
//! - Optional nonlinear (saturating) SVF mode for analog warmth
//! - All-pass output added
//! - Peak/Bell filter mode

#[derive(Clone)]
pub struct Svf {
    ic1eq: f32,
    ic2eq: f32,
    sample_rate: f32,
}

#[derive(Clone)]
pub struct SvfOutput {
    pub lp: f32,
    pub hp: f32,
    pub bp: f32,
    pub notch: f32,
    pub ap: f32,
    pub peak: f32,
}

impl Svf {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            ic1eq: 0.0,
            ic2eq: 0.0,
            sample_rate,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Reset filter state (useful for voice stealing)
    pub fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    /// Process with optional nonlinear saturation.
    /// `cutoff_hz`: Filter cutoff frequency
    /// `q`: Resonance (0.5 = no resonance, higher = more resonant)
    pub fn process(&mut self, input: f32, cutoff_hz: f32, q: f32) -> SvfOutput {
        // Clamp cutoff to safe range (avoid tan() blowup near Nyquist)
        let max_freq = self.sample_rate * 0.49;
        let cutoff = cutoff_hz.clamp(5.0, max_freq);
        
        let wd = 2.0 * std::f32::consts::PI * cutoff;
        let t = 1.0 / self.sample_rate;
        let g = (wd * t / 2.0).tan();
        let r = 1.0 / q.clamp(0.1, 100.0);  // Damping coefficient
        
        let a1 = 1.0 / (1.0 + g * (g + r));
        let a2 = g * a1;
        let a3 = g * a2;
        
        let hp = a1 * (input - r * self.ic1eq - self.ic1eq * g - self.ic2eq);
        let bp = a2 * input + a1 * self.ic1eq - a2 * self.ic2eq;
        let lp = a3 * input + a2 * self.ic1eq + a1 * self.ic2eq;
        
        self.ic1eq = 2.0 * bp - self.ic1eq;
        self.ic2eq = 2.0 * lp - self.ic2eq;
        
        let notch = hp + lp;      // Notch = HP + LP
        let ap = hp + lp - r * bp; // All-pass (approximation)
        let peak = lp - hp;       // Peak/bell shape
        
        SvfOutput {
            lp,
            hp,
            bp,
            notch,
            ap,
            peak,
        }
    }

    /// Nonlinear (saturating) SVF — Zavalishin method.
    /// Applies tanh saturation inside the feedback loop for analog warmth.
    pub fn process_nonlinear(&mut self, input: f32, cutoff_hz: f32, q: f32, drive: f32) -> SvfOutput {
        let max_freq = self.sample_rate * 0.49;
        let cutoff = cutoff_hz.clamp(5.0, max_freq);
        
        let wd = 2.0 * std::f32::consts::PI * cutoff;
        let t = 1.0 / self.sample_rate;
        let g = (wd * t / 2.0).tan();
        let r = 1.0 / q.clamp(0.1, 100.0);
        
        let a1 = 1.0 / (1.0 + g * (g + r));
        let a2 = g * a1;
        let a3 = g * a2;
        
        // Saturate the integrator states
        let ic1_sat = (self.ic1eq * (1.0 + drive)).tanh();
        let ic2_sat = (self.ic2eq * (1.0 + drive)).tanh();
        
        let hp = a1 * (input - r * ic1_sat - ic1_sat * g - ic2_sat);
        let bp = a2 * input + a1 * ic1_sat - a2 * ic2_sat;
        let lp = a3 * input + a2 * ic1_sat + a1 * ic2_sat;
        
        self.ic1eq = 2.0 * bp - self.ic1eq;
        self.ic2eq = 2.0 * lp - self.ic2eq;
        
        let notch = hp + lp;
        let ap = hp + lp - r * bp;
        let peak = lp - hp;
        
        SvfOutput {
            lp,
            hp,
            bp,
            notch,
            ap,
            peak,
        }
    }
}
