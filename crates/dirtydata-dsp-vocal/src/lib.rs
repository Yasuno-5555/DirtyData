#![allow(clippy::all)]

//! VocalTract Physical Modeling — Kelly-Lochbaum Waveguide
//!
//! **C8 Fix**: Full bidirectional wave propagation (forward + backward).
//! Without backward wave propagation, no standing waves form, meaning
//! no formants — the vowels あいうえお cannot be produced.
//!
//! **R7**: Nasal cavity coupling via 3-port junction at velum.
//!
//! ## Kelly-Lochbaum Scattering Junction
//!
//! At the boundary between sections with areas A_i and A_{i+1},
//! the reflection coefficient is:
//!
//!   r_i = (A_{i+1} - A_i) / (A_{i+1} + A_i)
//!
//! Forward and backward waves interact as:
//!
//!   f_out = f_in * (1 + r) + b_in * (-r)
//!   b_out = f_in * (r)     + b_in * (1 - r)
//!
//! This bidirectional propagation creates standing waves → formants.

/// Rosenberg glottal pulse model (improved over simple LF)
#[derive(Clone)]
pub struct GlottalSource {
    phase: f32,
    freq: f32,
    // Aspiration noise state
    noise_state: u32,
}

impl GlottalSource {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            freq: 100.0,
            noise_state: 12345,
        }
    }
}

impl Default for GlottalSource {
    fn default() -> Self {
        Self::new()
    }
}

impl GlottalSource {
    /// Fast pseudo-random noise (xorshift32, no dependency on rand)
    #[inline]
    fn noise(&mut self) -> f32 {
        self.noise_state ^= self.noise_state << 13;
        self.noise_state ^= self.noise_state >> 17;
        self.noise_state ^= self.noise_state << 5;
        (self.noise_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Generate one sample of glottal flow.
    /// `tension`: 0.0 = breathy, 1.0 = pressed/tense
    pub fn process(&mut self, sample_rate: f32, tension: f32) -> f32 {
        self.phase += self.freq / sample_rate;
        if self.phase > 1.0 {
            self.phase -= 1.0;
        }

        // Rosenberg C2 glottal pulse
        // Open phase: 0..Tp, Closing phase: Tp..Tp+Tn, Closed: rest
        let tp = 0.4; // Open phase ratio
        let tn = 0.16; // Closing phase ratio

        let pulse = if self.phase < tp {
            // Opening: 3(t/Tp)^2 - 2(t/Tp)^3
            let t_norm = self.phase / tp;
            3.0 * t_norm * t_norm - 2.0 * t_norm * t_norm * t_norm
        } else if self.phase < tp + tn {
            // Closing: cos^2 shape
            let t_norm = (self.phase - tp) / tn;
            let cos_val = (t_norm * std::f32::consts::PI * 0.5).cos();
            cos_val * cos_val
        } else {
            0.0 // Closed phase
        };

        // Aspiration noise (modulated by glottal opening)
        let noise = self.noise() * (1.0 - tension.clamp(0.0, 1.0)) * 0.15;
        let aspiration = noise * pulse.sqrt(); // Noise only during open phase

        pulse * tension.clamp(0.2, 1.0) + aspiration
    }

    pub fn set_freq(&mut self, f: f32) {
        self.freq = f.clamp(50.0, 500.0);
    }
}

/// A single waveguide section with forward and backward traveling waves.
#[derive(Clone)]
pub struct WaveguideSection {
    pub area: f32,     // Cross-sectional area (cm²)
    pub forward: f32,  // Right-traveling (forward) wave
    pub backward: f32, // Left-traveling (backward) wave
}

impl WaveguideSection {
    fn new(area: f32) -> Self {
        Self {
            area: area.max(0.01),
            forward: 0.0,
            backward: 0.0,
        }
    }
}

/// Full vocal tract model with bidirectional Kelly-Lochbaum scattering,
/// glottal reflection, lip radiation, and nasal coupling.
#[derive(Clone)]
pub struct VocalTract {
    /// Oral tract sections (pharynx → mouth)
    pub sections: Vec<WaveguideSection>,
    /// Nasal tract sections
    pub nasal_sections: Vec<WaveguideSection>,
    /// Glottal source
    pub glottis: GlottalSource,
    /// Velum coupling (0.0 = closed, 1.0 = fully open)
    pub velum_opening: f32,
    /// Velum position (which oral section connects to nasal tract)
    velum_idx: usize,
    /// Lip radiation filter state (1st-order highpass)
    lip_z1: f32,
    /// Nose radiation filter state
    nose_z1: f32,
}

impl VocalTract {
    pub fn new(num_sections: usize) -> Self {
        let num_sections = num_sections.max(8);
        let nasal_len = num_sections / 3;
        let velum_idx = num_sections / 4; // Velum connects near pharynx

        // Default oral tract areas (neutral vowel /ə/)
        let oral = (0..num_sections)
            .map(|i| {
                let t = i as f32 / num_sections as f32;
                // Slightly constricted in the middle
                let area = 1.0 + 0.5 * (t * std::f32::consts::PI * 2.0).sin();
                WaveguideSection::new(area.max(0.1))
            })
            .collect();

        // Nasal tract: narrows towards nostrils
        let nasal = (0..nasal_len)
            .map(|i| {
                let t = i as f32 / nasal_len as f32;
                WaveguideSection::new(2.0 - t * 1.5)
            })
            .collect();

        Self {
            sections: oral,
            nasal_sections: nasal,
            glottis: GlottalSource::new(),
            velum_opening: 0.0,
            velum_idx,
            lip_z1: 0.0,
            nose_z1: 0.0,
        }
    }

    /// Kelly-Lochbaum scattering junction (2-port).
    ///
    /// Given areas A1 and A2 at a junction:
    ///   r = (A2 - A1) / (A2 + A1)
    ///   f_out = f_in * (1 + r) + b_in * (-r)
    ///   b_out = f_in * r       + b_in * (1 - r)
    #[inline]
    fn scatter_2port(f_in: f32, b_in: f32, area_left: f32, area_right: f32) -> (f32, f32) {
        let r = (area_right - area_left) / (area_right + area_left).max(0.001);
        let f_out = f_in * (1.0 + r) + b_in * (-r);
        let b_out = f_in * r + b_in * (1.0 - r);
        (f_out, b_out)
    }

    /// 3-port scattering junction for velum (oral-nasal coupling).
    ///
    /// Port 1: oral tract (from pharynx)
    /// Port 2: oral tract (to mouth)
    /// Port 3: nasal tract
    ///
    /// Uses area-weighted power-preserving junction.
    #[inline]
    fn scatter_3port(
        f1_in: f32,
        b2_in: f32,
        b3_in: f32,
        a1: f32,
        a2: f32,
        a3: f32,
    ) -> (f32, f32, f32) {
        let sum_a = (a1 + a2 + a3).max(0.001);
        // Junction pressure (continuity condition)
        let p_j = 2.0 * (a1 * f1_in + a2 * b2_in + a3 * b3_in) / sum_a;
        let b1_out = p_j - f1_in; // Reflected back to pharynx
        let f2_out = p_j - b2_in; // Forward into mouth
        let f3_out = p_j - b3_in; // Forward into nose
        (b1_out, f2_out, f3_out)
    }

    /// Process one sample of the vocal tract.
    pub fn process(&mut self, sample_rate: f32, tension: f32) -> f32 {
        let n = self.sections.len();
        let _ = sample_rate; // Waveguide propagation is inherently sample-rate dependent

        // --- 1. Glottal source injection ---
        let source = self.glottis.process(sample_rate, tension);

        // Glottal reflection: partially reflecting boundary
        // r_glottis ≈ 0.7-0.95 (mostly closed during phonation)
        let r_glottis = 0.75 + tension.clamp(0.0, 1.0) * 0.2;
        let glottis_reflected = self.sections[0].backward * r_glottis;
        self.sections[0].forward = source + glottis_reflected;

        // --- 2. Forward scattering (glottis → lips) ---
        // Process junctions from left to right, computing new forward waves
        let mut new_forward = vec![0.0_f32; n];
        let mut new_backward = vec![0.0_f32; n];
        new_forward[0] = self.sections[0].forward;

        for i in 0..n - 1 {
            let is_velum = i == self.velum_idx && self.velum_opening > 0.01;

            if is_velum {
                // 3-port junction at velum
                let nasal_area = self
                    .nasal_sections
                    .get(0)
                    .map(|s| s.area * self.velum_opening)
                    .unwrap_or(0.0);
                let nasal_backward = self
                    .nasal_sections
                    .get(0)
                    .map(|s| s.backward)
                    .unwrap_or(0.0);

                let (b_pharynx, f_mouth, f_nose) = Self::scatter_3port(
                    self.sections[i].forward,
                    self.sections[i + 1].backward,
                    nasal_backward,
                    self.sections[i].area,
                    self.sections[i + 1].area,
                    nasal_area,
                );
                new_backward[i] = b_pharynx;
                new_forward[i + 1] = f_mouth;

                // Inject forward wave into nasal tract
                if let Some(ns) = self.nasal_sections.first_mut() {
                    ns.forward = f_nose;
                }
            } else {
                // Standard 2-port Kelly-Lochbaum junction
                let (f_out, b_out) = Self::scatter_2port(
                    self.sections[i].forward,
                    self.sections[i + 1].backward,
                    self.sections[i].area,
                    self.sections[i + 1].area,
                );
                new_forward[i + 1] = f_out;
                new_backward[i] = b_out;
            }
        }

        // --- 3. Lip radiation ---
        // Lips act as a partially reflecting boundary + radiation highpass
        let r_lip = -0.85; // Negative reflection at open end
        let lip_forward = new_forward[n - 1];
        new_backward[n - 1] = lip_forward * r_lip;

        // Radiation: first-order highpass (lips act as differentiator)
        let lip_raw = lip_forward + new_backward[n - 1];
        let lip_out = lip_raw - self.lip_z1;
        self.lip_z1 = lip_raw * 0.999; // Slight leak to prevent DC

        // --- 4. Update all section states ---
        for i in 0..n {
            self.sections[i].forward = new_forward[i];
            self.sections[i].backward = new_backward[i];
        }

        // --- 5. Nasal tract processing ---
        let mut nose_out = 0.0;
        let nn = self.nasal_sections.len();
        if nn > 0 && self.velum_opening > 0.01 {
            // Forward propagation through nasal tract
            for i in 0..nn - 1 {
                let (f_out, b_out) = Self::scatter_2port(
                    self.nasal_sections[i].forward,
                    self.nasal_sections[i + 1].backward,
                    self.nasal_sections[i].area,
                    self.nasal_sections[i + 1].area,
                );
                self.nasal_sections[i + 1].forward = f_out;
                self.nasal_sections[i].backward = b_out;
            }

            // Nostril radiation (more damped than lips)
            let r_nostril = -0.7;
            let last = nn - 1;
            self.nasal_sections[last].backward = self.nasal_sections[last].forward * r_nostril;
            let nose_raw = self.nasal_sections[last].forward + self.nasal_sections[last].backward;
            nose_out = nose_raw - self.nose_z1;
            self.nose_z1 = nose_raw * 0.999;
        }

        // --- 6. Mix oral + nasal output ---
        lip_out + nose_out * self.velum_opening
    }

    /// Set tongue shape to produce different vowels.
    /// `x`: tongue position (0.0 = back/pharynx, 1.0 = front/lips)
    /// `y`: tongue height / constriction diameter (0.0 = closed, 1.0 = open)
    pub fn set_tongue(&mut self, x: f32, y: f32) {
        let n = self.sections.len();
        let constriction_pos = (x.clamp(0.0, 1.0) * n as f32) as usize;
        let diameter = y.clamp(0.01, 1.0) * 3.5; // Scale to cm

        for i in 0..n {
            let dist = (i as f32 - constriction_pos as f32).abs();
            // Gaussian constriction profile
            let constriction = (-dist * dist / (n as f32 * 0.04)).exp();
            let area = 1.5 + (diameter - 1.5) * constriction;
            self.sections[i].area = area.max(0.05); // Never fully closed
        }
    }

    /// Set velum opening for nasal sounds.
    /// 0.0 = oral only (a, i, u, e, o)
    /// 1.0 = nasal (n, m, ŋ)
    pub fn set_velum(&mut self, opening: f32) {
        self.velum_opening = opening.clamp(0.0, 1.0);
    }

    /// Configure vowel presets based on Japanese vowels.
    pub fn set_vowel(&mut self, vowel: char) {
        match vowel {
            'a' | 'あ' => {
                self.set_tongue(0.5, 0.9);
                self.set_velum(0.0);
            }
            'i' | 'い' => {
                self.set_tongue(0.8, 0.3);
                self.set_velum(0.0);
            }
            'u' | 'う' => {
                self.set_tongue(0.3, 0.3);
                self.set_velum(0.0);
            }
            'e' | 'え' => {
                self.set_tongue(0.7, 0.5);
                self.set_velum(0.0);
            }
            'o' | 'お' => {
                self.set_tongue(0.4, 0.4);
                self.set_velum(0.0);
            }
            'n' | 'ん' => {
                self.set_tongue(0.5, 0.7);
                self.set_velum(0.8);
            }
            'm' => {
                self.set_tongue(0.9, 0.01);
                self.set_velum(1.0);
            }
            _ => {
                self.set_tongue(0.5, 0.7);
                self.set_velum(0.0);
            }
        }
    }
}
