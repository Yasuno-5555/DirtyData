use super::base::*;
use dirtydata_core::types::ConfigSnapshot;

#[derive(Clone)]
pub struct KarplusStrongNode {
    inner: dirtydata_dsp_ks::KarplusStrong,
}
impl KarplusStrongNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_ks::KarplusStrong::new(sample_rate),
        }
    }
}
impl DspNode for KarplusStrongNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let freq = config
            .get("freq")
            .and_then(|v| v.as_float())
            .unwrap_or(440.0) as f32;
        let damping = config
            .get("damping")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let dispersion = config
            .get("dispersion")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let pick_pos = config
            .get("pick_pos")
            .and_then(|v| v.as_float())
            .unwrap_or(0.2) as f32;

        let out = self
            .inner
            .process(input, freq, damping, dispersion, pick_pos);
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct ModalResonatorNode {
    inner: dirtydata_dsp_modal::ModalResonatorBank,
    last_material: u32,
    last_freq: f32,
    last_bright: f32,
}
impl ModalResonatorNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_modal::ModalResonatorBank::new(sample_rate),
            last_material: 999,
            last_freq: -1.0,
            last_bright: -1.0,
        }
    }
}
impl DspNode for ModalResonatorNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);

        let material = config
            .get("material")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as u32;
        let freq = config
            .get("base_freq")
            .and_then(|v| v.as_float())
            .unwrap_or(440.0) as f32;
        let bright = config
            .get("brightness")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;

        if material != self.last_material
            || (freq - self.last_freq).abs() > 0.1
            || (bright - self.last_bright).abs() > 0.01
        {
            self.inner.set_material(material, freq, bright);
            self.last_material = material;
            self.last_freq = freq;
            self.last_bright = bright;
        }

        let out = self.inner.process(input);
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct VocalTractNode {
    inner: dirtydata_dsp_vocal::VocalTract,
}
impl VocalTractNode {
    pub fn new(sample_rate: f32) -> Self {
        let _ = sample_rate;
        Self {
            inner: dirtydata_dsp_vocal::VocalTract::new(44),
        } // 44 sections
    }
}
impl DspNode for VocalTractNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let freq = config
            .get("pitch")
            .and_then(|v| v.as_float())
            .unwrap_or(110.0) as f32;
        let tongue_x = config
            .get("tongue_x")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let tongue_y = config
            .get("tongue_y")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let tension = config
            .get("tension")
            .and_then(|v| v.as_float())
            .unwrap_or(0.8) as f32;
        let velum = config
            .get("velum")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32;

        let vowel = config.get("vowel").and_then(|v| v.as_string());
        if let Some(v) = vowel {
            if let Some(ch) = v.chars().next() {
                self.inner.set_vowel(ch);
            }
        } else {
            self.inner.glottis.set_freq(freq);
            self.inner.set_tongue(tongue_x, tongue_y);
            self.inner.set_velum(velum);
        }

        let out = self.inner.process(ctx.sample_rate, tension);
        for o in outputs {
            *o = [out, out];
        }
    }
}
