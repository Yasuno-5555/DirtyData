use super::base::*;
use dirtydata_core::types::ConfigSnapshot;

#[derive(Clone)]
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

#[derive(Clone)]
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
        let filter_type = config.get("type").and_then(|v| v.as_string());
        let smooth = self.freq_smooth.get_or_insert_with(|| SmoothedValue::new(freq_target, ctx.sample_rate, 10.0));
        let freq = smooth.next();

        let w0 = 2.0 * std::f32::consts::PI * freq / ctx.sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let (b0, b1, b2, a0, a1, a2) = match filter_type.map(|s| s.as_str()).unwrap_or("lpf") {
            "hpf" => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            "bandpass" => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            "notch" => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            "peak" => {
                let gain_db = config.get("gain_db").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
                let a_val = 10.0_f32.powf(gain_db / 40.0);
                let b0 = 1.0 + alpha * a_val;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a_val;
                let a0 = 1.0 + alpha / a_val;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a_val;
                (b0, b1, b2, a0, a1, a2)
            }
            _ => { // LPF
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        let inv_a0 = 1.0 / a0;
        let ff0 = b0 * inv_a0;
        let ff1 = b1 * inv_a0;
        let ff2 = b2 * inv_a0;
        let fb1 = a1 * inv_a0;
        let fb2 = a2 * inv_a0;

        for i in 0..2 {
            let x = if inputs.len() > i { inputs[i] } else { 0.0 };
            let y = ff0 * x + self.z1[i];
            self.z1[i] = ff1 * x - fb1 * y + self.z2[i];
            self.z2[i] = ff2 * x - fb2 * y;
            outputs[0][i] = y;
        }
    }
    fn update_parameter(&mut self, param: &str, value: f32) {
        if param == "frequency" { if let Some(s) = &mut self.freq_smooth { s.set_target(value); } }
    }
}

#[derive(Clone)]
pub struct AddNode;
impl AddNode { pub fn new() -> Self { Self } }
impl DspNode for AddNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let mut l = 0.0;
        let mut r = 0.0;
        for chunk in inputs.chunks_exact(2) {
            l += chunk[0];
            r += chunk[1];
        }
        outputs[0] = [l, r];
    }
}

#[derive(Clone)]
pub struct MultiplyNode;
impl MultiplyNode { pub fn new() -> Self { Self } }
impl DspNode for MultiplyNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        if inputs.len() >= 4 {
            outputs[0] = [inputs[0] * inputs[2], inputs[1] * inputs[3]];
        } else {
            outputs[0] = [0.0, 0.0];
        }
    }
}

#[derive(Clone)]
pub struct ClipNode;
impl ClipNode { pub fn new() -> Self { Self } }
impl DspNode for ClipNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let min = config.get("min").and_then(|v| v.as_float()).unwrap_or(-1.0) as f32;
        let max = config.get("max").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        if inputs.len() >= 2 {
            outputs[0] = [inputs[0].clamp(min, max), inputs[1].clamp(min, max)];
        }
    }
}

#[derive(Clone)]
pub struct CompressorNode {
    envelope: f32,
}

impl CompressorNode {
    pub fn new() -> Self { Self { envelope: 0.0 } }
}

impl DspNode for CompressorNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
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
