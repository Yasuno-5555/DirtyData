use dirtydata_core::types::ConfigSnapshot;
use super::base::*;
use rand::prelude::*;
use rand_pcg::Pcg32;

#[derive(Clone)]
pub struct LogicNode;
impl LogicNode { pub fn new() -> Self { Self } }
impl DspNode for LogicNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let mode = config.get("mode").and_then(|v| v.as_string()).map(|s| s.as_str()).unwrap_or("AND");
        let threshold = config.get("threshold").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;

        let a = inputs.get(0).cloned().unwrap_or(0.0) > threshold;
        let b = inputs.get(1).cloned().unwrap_or(0.0) > threshold;

        let res = match mode {
            "AND" => a && b,
            "OR" => a || b,
            "XOR" => a ^ b,
            "NOT" => !a,
            _ => a && b,
        };

        let val = if res { 1.0 } else { 0.0 };
        for out in outputs.iter_mut() {
            *out = [val, val];
        }
    }
}

#[derive(Clone)]
pub struct SlewLimiterNode {
    current: f32,
}

impl SlewLimiterNode {
    pub fn new() -> Self { Self { current: 0.0 } }
}

impl DspNode for SlewLimiterNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let rise = config.get("rise").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let fall = config.get("fall").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        
        for i in 0..outputs.len() {
            let target = inputs.get(i * 2).cloned().unwrap_or(0.0);
            let diff = target - self.current;
            let limit = if diff > 0.0 { rise } else { fall };
            let step = diff.clamp(-limit / ctx.sample_rate, limit / ctx.sample_rate);
            self.current += step;
            outputs[i] = [self.current, self.current];
        }
    }
}

#[derive(Clone)]
pub struct SampleHoldNode {
    last_val: [f32; 2],
    last_trig: f32,
}

impl SampleHoldNode {
    pub fn new() -> Self { Self { last_val: [0.0, 0.0], last_trig: 0.0 } }
}

impl DspNode for SampleHoldNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        for i in 0..outputs.len() {
            let sig_l = inputs.get(i * 2).cloned().unwrap_or(0.0);
            let sig_r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0);
            let trig = inputs.get(i * 2 + 2).cloned().unwrap_or(0.0);
            
            if trig > 0.5 && self.last_trig <= 0.5 {
                self.last_val = [sig_l, sig_r];
            }
            self.last_trig = trig;
            outputs[i] = self.last_val;
        }
    }
}

#[derive(Clone)]
pub struct ClockNode {
    phase: f32,
}

impl ClockNode {
    pub fn new() -> Self { Self { phase: 0.0 } }
}

impl DspNode for ClockNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let bpm = config.get("bpm").and_then(|v| v.as_float()).unwrap_or(120.0) as f32;
        let division = config.get("division").and_then(|v| v.as_float()).unwrap_or(4.0) as f32;
        
        let freq = (bpm / 60.0) * (division / 4.0);
        let phase_step = freq / ctx.sample_rate;
        
        for i in 0..outputs.len() {
            let old_phase = self.phase;
            self.phase = (self.phase + phase_step).fract();
            
            let trigger = if self.phase < old_phase { 1.0 } else { 0.0 };
            outputs[i] = [trigger, trigger];
        }
    }
}

#[derive(Clone)]
pub struct ProbabilityGateNode {
    rng: Pcg32,
}

impl ProbabilityGateNode {
    pub fn new() -> Self { Self { rng: Pcg32::seed_from_u64(42) } }
}

impl DspNode for ProbabilityGateNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let prob = config.get("probability").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        
        for i in 0..outputs.len() {
            let trig = inputs.get(i * 2).cloned().unwrap_or(0.0);
            let mut out = 0.0;
            if trig > 0.5 {
                if self.rng.gen::<f32>() < prob {
                    out = 1.0;
                }
            }
            outputs[i] = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct FeedbackNode {
    latch: [f32; 2],
}

impl FeedbackNode {
    pub fn new() -> Self {
        Self { latch: [0.0, 0.0] }
    }
}

impl DspNode for FeedbackNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        outputs[0] = self.latch;
        if inputs.len() >= 2 {
            self.latch = [inputs[0], inputs[1]];
        }
    }
}

#[derive(Clone)]
pub struct OscOutNode {
    last_sent_val: f32,
    threshold: f32,
}

impl OscOutNode {
    pub fn new() -> Self {
        Self {
            last_sent_val: 0.0,
            threshold: 0.001,
        }
    }
}

impl DspNode for OscOutNode {
    fn process(&mut self, inputs: &[f32], _outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let addr = config.get("address").and_then(|v| v.as_string()).map(|s| s.as_str()).unwrap_or("/dirtydata/out");
        let val = inputs.get(0).cloned().unwrap_or(0.0);

        if (val - self.last_sent_val).abs() > self.threshold {
            if let Some(tx) = ctx.osc_tx {
                let _ = tx.try_send(OscMessage {
                    addr: addr.to_string(),
                    args: vec![rosc::OscType::Float(val)],
                });
                self.last_sent_val = val;
            }
        }
    }
}

#[derive(Clone)]
pub struct MatrixMixerNode {
    inner: dirtydata_dsp_matrix::MatrixMixer,
}
impl MatrixMixerNode {
    pub fn new(num_in: usize, num_out: usize) -> Self {
        Self { inner: dirtydata_dsp_matrix::MatrixMixer::new(num_in, num_out) }
    }
}
impl DspNode for MatrixMixerNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let g00 = config.get("g00").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let g01 = config.get("g01").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        let g10 = config.get("g10").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        let g11 = config.get("g11").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        
        self.inner.set_gain(0, 0, g00);
        self.inner.set_gain(1, 0, g01);
        self.inner.set_gain(0, 1, g10);
        self.inner.set_gain(1, 1, g11);
        
        let in_flat: Vec<f32> = inputs.iter().copied().collect();
        let mut out_flat = vec![0.0; outputs.len() * 2];
        self.inner.process(&in_flat, &mut out_flat);
        
        for (i, o) in outputs.iter_mut().enumerate() {
            o[0] = out_flat[i * 2];
            o[1] = out_flat[i * 2 + 1];
        }
    }
}

#[derive(Clone)]
pub struct SlewNode {
    inner: dirtydata_dsp_cv::Slew,
}
impl SlewNode {
    pub fn new() -> Self { Self { inner: dirtydata_dsp_cv::Slew::new() } }
}
impl DspNode for SlewNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let rise = config.get("rise").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let fall = config.get("fall").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let out = self.inner.process(input, rise, fall, ctx.sample_rate);
        for o in outputs { *o = [out, out]; }
    }
}

#[derive(Clone)]
pub struct EuclideanSequencerNode {
    inner: dirtydata_dsp_cv::EuclideanSequencer,
}
impl EuclideanSequencerNode {
    pub fn new() -> Self { Self { inner: dirtydata_dsp_cv::EuclideanSequencer::new() } }
}
impl DspNode for EuclideanSequencerNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let clock = inputs.get(0).copied().unwrap_or(0.0);
        self.inner.steps = config.get("steps").and_then(|v| v.as_float()).unwrap_or(16.0) as u32;
        self.inner.hits = config.get("hits").and_then(|v| v.as_float()).unwrap_or(4.0) as u32;
        let out = self.inner.process(clock);
        for o in outputs { *o = [out, out]; }
    }
}

#[derive(Clone)]
pub struct FunctionGeneratorNode {
    inner: dirtydata_dsp_control::FunctionGenerator,
}
impl FunctionGeneratorNode {
    pub fn new() -> Self { Self { inner: dirtydata_dsp_control::FunctionGenerator::new() } }
}
impl DspNode for FunctionGeneratorNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let trigger = inputs.get(0).copied().unwrap_or(0.0);
        let rise = config.get("rise").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let fall = config.get("fall").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let cycle = config.get("cycle").and_then(|v| v.as_bool()).unwrap_or(false);
        let out = self.inner.process(trigger, rise, fall, cycle, ctx.sample_rate);
        for o in outputs { *o = [out, out]; }
    }
}
