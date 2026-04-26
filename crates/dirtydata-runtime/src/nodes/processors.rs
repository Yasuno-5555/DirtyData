use super::base::*;
use dirtydata_core::types::ConfigSnapshot;

pub struct GainNode {
    pub gain_smooth: Option<SmoothedValue>,
}

impl GainNode {
    pub fn new() -> Self { Self { gain_smooth: None } }
}

impl DspNode for GainNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let gain_db_target = config.get("gain_db").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        let smooth = self.gain_smooth.get_or_insert_with(|| SmoothedValue::new(gain_db_target, ctx.sample_rate, 10.0));
        let linear = 10.0_f32.powf(smooth.next() / 20.0);
        if inputs.len() >= 2 {
            outputs[0] = [inputs[0] * linear, inputs[1] * linear];
        }
    }
}

pub struct BiquadFilterNode {
    z1: [f32; 2],
    z2: [f32; 2],
    freq_smooth: Option<SmoothedValue>,
}

impl BiquadFilterNode {
    pub fn new() -> Self { Self { z1: [0.0, 0.0], z2: [0.0, 0.0], freq_smooth: None } }
}

impl DspNode for BiquadFilterNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let freq_target = config.get("frequency").and_then(|v| v.as_float()).unwrap_or(1000.0) as f32;
        let q = config.get("q").and_then(|v| v.as_float()).unwrap_or(0.707) as f32;
        let smooth = self.freq_smooth.get_or_insert_with(|| SmoothedValue::new(freq_target, ctx.sample_rate, 10.0));
        let w0 = 2.0 * std::f32::consts::PI * smooth.next() / ctx.sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();
        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        let inv_a0 = 1.0 / a0;
        for i in 0..2 {
            let x = if inputs.len() > i { inputs[i] } else { 0.0 };
            let y = (b0 * x + self.z1[i]) * inv_a0;
            self.z1[i] = b1 * x - a1 * y + self.z2[i];
            self.z2[i] = b2 * x - a2 * y;
            outputs[0][i] = y;
        }
    }
}

pub struct CompressorNode {
    envelope: f32,
}

impl CompressorNode {
    pub fn new() -> Self { Self { envelope: 0.0 } }
}

impl DspNode for CompressorNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let threshold_db = config.get("threshold_db").and_then(|v| v.as_float()).unwrap_or(-20.0) as f32;
        let ratio = config.get("ratio").and_then(|v| v.as_float()).unwrap_or(4.0) as f32;
        let threshold = 10.0_f32.powf(threshold_db / 20.0);
        let peak = if inputs.len() >= 2 { inputs[0].abs().max(inputs[1].abs()) } else { 0.0 };
        self.envelope += 0.1 * (peak - self.envelope);
        let gain = if self.envelope > threshold {
            let over_db = 20.0 * (self.envelope / threshold).log10();
            10.0_f32.powf(-(over_db * (1.0 - 1.0 / ratio)) / 20.0)
        } else { 1.0 };
        if inputs.len() >= 2 {
            outputs[0] = [inputs[0] * gain, inputs[1] * gain];
        }
    }
}
