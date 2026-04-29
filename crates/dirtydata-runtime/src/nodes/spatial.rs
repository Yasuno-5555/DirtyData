use super::base::*;
use dirtydata_core::types::ConfigSnapshot;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct DelayNode {
    buffer: Vec<[f32; 2]>,
    write_pos: usize,
}

impl DelayNode {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![[0.0, 0.0]; max_delay_samples],
            write_pos: 0,
        }
    }
}

impl DspNode for DelayNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let delay_samples = config
            .get("delay_samples")
            .and_then(|v| v.as_float())
            .unwrap_or(4410.0) as usize;
        let feedback = config
            .get("feedback")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let read_pos = (self.write_pos + self.buffer.len() - delay_samples) % self.buffer.len();
        let delayed = self.buffer[read_pos];
        outputs[0] = delayed;
        let in_l = if inputs.len() >= 1 { inputs[0] } else { 0.0 };
        let in_r = if inputs.len() >= 2 { inputs[1] } else { 0.0 };
        self.buffer[self.write_pos] = [in_l + delayed[0] * feedback, in_r + delayed[1] * feedback];
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
    }
}

#[derive(Clone)]
pub struct ReverbNode {
    delays: Vec<VecDeque<f32>>,
    feedback_matrix: [[f32; 4]; 4],
}

impl ReverbNode {
    pub fn new(sample_rate: f32) -> Self {
        let delay_times = [0.037, 0.043, 0.051, 0.061]; // Primes in seconds
        let delays = delay_times
            .iter()
            .map(|&t| {
                let size = (t * sample_rate) as usize;
                let mut dq = VecDeque::with_capacity(size);
                for _ in 0..size {
                    dq.push_back(0.0);
                }
                dq
            })
            .collect();

        let h = 0.5;
        let feedback_matrix = [[h, h, h, h], [h, -h, h, -h], [h, h, -h, -h], [h, -h, -h, h]];

        Self {
            delays,
            feedback_matrix,
        }
    }
}

impl DspNode for ReverbNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let decay = config
            .get("decay")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let mix = config.get("mix").and_then(|v| v.as_float()).unwrap_or(0.3) as f32;

        for i in 0..outputs.len() {
            let input_l = inputs.get(i * 2).cloned().unwrap_or(0.0);
            let input_r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0);
            let mono_in = (input_l + input_r) * 0.5;

            let mut y = [0.0; 4];
            for j in 0..4 {
                y[j] = *self.delays[j].front().unwrap();
            }

            let mut fb = [0.0; 4];
            for row in 0..4 {
                for col in 0..4 {
                    fb[row] += self.feedback_matrix[row][col] * y[col];
                }
            }

            for j in 0..4 {
                self.delays[j].push_back(mono_in + fb[j] * decay);
                self.delays[j].pop_front();
            }

            let wet_l = y[0] + y[1];
            let wet_r = y[2] + y[3];

            outputs[i] = [
                input_l * (1.0 - mix) + wet_l * mix,
                input_r * (1.0 - mix) + wet_r * mix,
            ];
        }
    }
}

#[derive(Clone)]
pub struct SpringReverbNode {
    inner: dirtydata_dsp_spring::SpringReverb,
}
impl SpringReverbNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_spring::SpringReverb::new(sample_rate),
        }
    }
}
impl DspNode for SpringReverbNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let decay = config
            .get("decay")
            .and_then(|v| v.as_float())
            .unwrap_or(0.8) as f32;
        let dispersion = config
            .get("dispersion")
            .and_then(|v| v.as_float())
            .unwrap_or(0.6) as f32;

        let out = self.inner.process(input, decay, dispersion);
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct BbdDelayNode {
    inner: dirtydata_dsp_bbd::BbdDelay,
}
impl BbdDelayNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_bbd::BbdDelay::new(sample_rate, 2.0),
        }
    }
}
impl DspNode for BbdDelayNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let time_ms = config
            .get("time_ms")
            .and_then(|v| v.as_float())
            .unwrap_or(300.0) as f32;
        let feedback = config
            .get("feedback")
            .and_then(|v| v.as_float())
            .unwrap_or(0.3) as f32;
        let dirt = config.get("dirt").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;

        let out = self.inner.process(input, time_ms, feedback, dirt);
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct TapeMachineNode {
    inner: dirtydata_dsp_tape::TapeMachine,
}
impl TapeMachineNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_tape::TapeMachine::new(sample_rate),
        }
    }
}
impl DspNode for TapeMachineNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let drive = config
            .get("drive")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;
        let wow = config.get("wow").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let flutter = config
            .get("flutter")
            .and_then(|v| v.as_float())
            .unwrap_or(0.1) as f32;
        let bias = config.get("bias").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;

        let out = self.inner.process(input, drive, wow, flutter, bias);
        for o in outputs {
            *o = [out, out];
        }
    }
}
