use super::base::*;
use dirtydata_core::types::ConfigSnapshot;
use std::sync::Arc;
use rand::prelude::*;
use rand_pcg::Pcg32;

pub struct OscillatorNode {
    pub phase: f32,
    pub freq_smooth: Option<SmoothedValue>,
}

impl OscillatorNode {
    pub fn new() -> Self { Self { phase: 0.0, freq_smooth: None } }
}

impl DspNode for OscillatorNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let freq_target = config.get("frequency").and_then(|v| v.as_float()).unwrap_or(440.0) as f32;
        let wave_type = config.get("waveform").and_then(|v| v.as_string());
        let smooth = self.freq_smooth.get_or_insert_with(|| SmoothedValue::new(freq_target, ctx.sample_rate, 10.0));
        let freq = smooth.next();
        let phase_inc = freq / ctx.sample_rate;
        let val = match wave_type.map(|s| s.as_str()).unwrap_or("sine") {
            "sine" => (self.phase * 2.0 * std::f32::consts::PI).sin(),
            "saw" => (self.phase * 2.0) - 1.0,
            "square" => if self.phase < 0.5 { 1.0 } else { -1.0 },
            _ => (self.phase * 2.0 * std::f32::consts::PI).sin(),
        };
        outputs[0] = [val, val];
        self.phase = (self.phase + phase_inc) % 1.0;
    }
    fn update_parameter(&mut self, param: &str, value: f32) {
        if param == "frequency" { if let Some(s) = &mut self.freq_smooth { s.set_target(value); } }
    }
    fn extract_state(&self) -> NodeState { 
        NodeState::from_json(serde_json::json!({ "phase": self.phase })) 
    }
    fn inject_state(&mut self, state: &NodeState) {
        if let Some(data) = state.to_json::<serde_json::Value>() {
            if let Some(p) = data.get("phase").and_then(|v| v.as_f64()) {
                self.phase = p as f32;
            }
        }
    }
}

pub struct NoiseNode {
    rng: Pcg32,
}

impl NoiseNode {
    pub fn new(seed: u64) -> Self { Self { rng: Pcg32::seed_from_u64(seed) } }
}

impl DspNode for NoiseNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let val: f32 = self.rng.random_range(-1.0..1.0);
        outputs[0] = [val, val];
    }
    fn extract_state(&self) -> NodeState { NodeState::Empty }
    fn inject_state(&mut self, _state: &NodeState) {}
}

pub struct AssetReaderNode {
    data: Arc<Vec<f32>>,
    cursor: usize,
}

impl AssetReaderNode {
    pub fn new(data: Arc<Vec<f32>>) -> Self { Self { data, cursor: 0 } }
}

impl DspNode for AssetReaderNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        if self.cursor + 1 < self.data.len() {
            outputs[0] = [self.data[self.cursor], self.data[self.cursor+1]];
            self.cursor += 2;
        } else {
            outputs[0] = [0.0, 0.0];
        }
    }
    fn extract_state(&self) -> NodeState {
        NodeState::from_json(self.cursor)
    }
    fn inject_state(&mut self, state: &NodeState) {
        if let Some(cursor) = state.to_json::<usize>() {
            self.cursor = cursor;
        }
    }
}

pub struct MidiInNode {
    _rx: crossbeam_channel::Receiver<crate::nodes::MidiEvent>,
}

impl MidiInNode {
    pub fn new(rx: crossbeam_channel::Receiver<crate::nodes::MidiEvent>) -> Self { Self { _rx: rx } }
}

impl DspNode for MidiInNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        // Just a stub for now
        outputs[0] = [0.0, 0.0];
    }
}
