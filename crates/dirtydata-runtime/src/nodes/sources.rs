use super::base::*;
use dirtydata_core::types::ConfigSnapshot;
use rand::prelude::*;
use rand_pcg::Pcg32;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MidiEvent {
    pub sample_index: u64,
    pub message: [u8; 3],
}

#[derive(Clone)]
pub struct OscillatorNode {
    pub phase: f32,
    pub freq_smooth: Option<SmoothedValue>,
}

impl OscillatorNode {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            freq_smooth: None,
        }
    }
}

impl DspNode for OscillatorNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let freq_target = config
            .get("frequency")
            .and_then(|v| v.as_float())
            .unwrap_or(440.0) as f32;
        let wave_type = config.get("waveform").and_then(|v| v.as_string());
        let smooth = self
            .freq_smooth
            .get_or_insert_with(|| SmoothedValue::new(freq_target, ctx.sample_rate, 10.0));
        let freq = smooth.next();
        let phase_inc = freq / ctx.sample_rate;
        let val = match wave_type.map(|s| s.as_str()).unwrap_or("sine") {
            "sine" => (self.phase * 2.0 * std::f32::consts::PI).sin(),
            "saw" => (self.phase * 2.0) - 1.0,
            "square" => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            _ => (self.phase * 2.0 * std::f32::consts::PI).sin(),
        };
        outputs[0] = [val, val];
        self.phase = (self.phase + phase_inc) % 1.0;
    }
    fn update_parameter(&mut self, param: &str, value: f32) {
        if param == "frequency" {
            if let Some(s) = &mut self.freq_smooth {
                s.set_target(value);
            }
        }
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

#[derive(Clone)]
pub struct NoiseNode {
    rng: Pcg32,
}

impl NoiseNode {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Pcg32::seed_from_u64(seed),
        }
    }
}

impl DspNode for NoiseNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let val: f32 = self.rng.gen_range(-1.0..1.0);
        outputs[0] = [val, val];
    }
    fn extract_state(&self) -> NodeState {
        NodeState::Empty
    }
    fn inject_state(&mut self, _state: &NodeState) {}
}

#[derive(Clone)]
pub struct AssetReaderNode {
    data: Arc<Vec<f32>>,
    cursor: usize,
}

impl AssetReaderNode {
    pub fn new(data: Arc<Vec<f32>>) -> Self {
        Self { data, cursor: 0 }
    }
}

impl DspNode for AssetReaderNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        if self.cursor + 1 < self.data.len() {
            outputs[0] = [self.data[self.cursor], self.data[self.cursor + 1]];
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

#[derive(Clone)]
pub struct MidiInNode {
    event_rx: crossbeam_channel::Receiver<MidiEvent>,
    gate: f32,
    pitch_hz: f32,
    velocity: f32,
    pending_events: Vec<MidiEvent>,
}

impl MidiInNode {
    pub fn new(event_rx: crossbeam_channel::Receiver<MidiEvent>) -> Self {
        Self {
            event_rx,
            gate: 0.0,
            pitch_hz: 440.0,
            velocity: 0.0,
            pending_events: Vec::new(),
        }
    }
}

impl DspNode for MidiInNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.pending_events.push(event);
        }
        self.pending_events.retain(|event| {
            if event.sample_index <= ctx.global_sample_index {
                let status = event.message[0] & 0xF0;
                match status {
                    0x90 => {
                        let note = event.message[1];
                        let vel = event.message[2];
                        if vel > 0 {
                            self.gate = 1.0;
                            self.pitch_hz = 440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0);
                            self.velocity = vel as f32 / 127.0;
                        } else {
                            self.gate = 0.0;
                        }
                    }
                    0x80 => {
                        self.gate = 0.0;
                    }
                    _ => {}
                }
                false
            } else {
                true
            }
        });
        outputs[0] = [self.gate, self.gate];
        if outputs.len() > 1 {
            outputs[1] = [self.pitch_hz, self.pitch_hz];
        }
        if outputs.len() > 2 {
            outputs[2] = [self.velocity, self.velocity];
        }
    }
}

#[derive(Clone)]
pub struct TriggerNode {
    last_gate: bool,
}
impl TriggerNode {
    pub fn new() -> Self {
        Self { last_gate: false }
    }
}
impl DspNode for TriggerNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let gate = inputs.get(0).cloned().unwrap_or(0.0) > 0.0;
        let triggered = gate && !self.last_gate;
        self.last_gate = gate;
        let val = if triggered { 1.0 } else { 0.0 };
        outputs[0] = [val, val];
    }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EnvState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
    FastRelease,
}

#[derive(Clone)]
pub struct EnvelopeNode {
    pub state: EnvState,
    pub level: f32,
}
impl EnvelopeNode {
    pub fn new() -> Self {
        Self {
            state: EnvState::Idle,
            level: 0.0,
        }
    }
    pub fn is_idle(&self) -> bool {
        self.state == EnvState::Idle
    }
}
impl DspNode for EnvelopeNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let a = config
            .get("attack")
            .and_then(|v| v.as_float())
            .unwrap_or(0.1) as f32;
        let d = config
            .get("decay")
            .and_then(|v| v.as_float())
            .unwrap_or(0.1) as f32;
        let s = config
            .get("sustain")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let r = config
            .get("release")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let gate = inputs.get(0).cloned().unwrap_or(0.0) > 0.0;
        match self.state {
            EnvState::Idle => {
                if gate {
                    self.state = EnvState::Attack;
                }
            }
            EnvState::Attack => {
                if !gate {
                    self.state = EnvState::Release;
                } else {
                    self.level += 1.0 / (a.max(0.001) * ctx.sample_rate);
                    if self.level >= 1.0 {
                        self.level = 1.0;
                        self.state = EnvState::Decay;
                    }
                }
            }
            EnvState::Decay => {
                if !gate {
                    self.state = EnvState::Release;
                } else {
                    self.level -= (1.0 - s) / (d.max(0.001) * ctx.sample_rate);
                    if self.level <= s {
                        self.level = s;
                        self.state = EnvState::Sustain;
                    }
                }
            }
            EnvState::Sustain => {
                if !gate {
                    self.state = EnvState::Release;
                }
            }
            EnvState::Release => {
                if gate {
                    self.state = EnvState::Attack;
                } else {
                    self.level -= 1.0 / (r.max(0.001) * ctx.sample_rate);
                    if self.level <= 0.0 {
                        self.level = 0.0;
                        self.state = EnvState::Idle;
                    }
                }
            }
            EnvState::FastRelease => {
                self.level -= 1.0 / (0.005 * ctx.sample_rate);
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.state = EnvState::Idle;
                }
            }
        }
        outputs[0] = [self.level, self.level];
    }
    fn update_parameter(&mut self, param: &str, _value: f32) {
        if param == "steal" {
            self.state = EnvState::FastRelease;
        }
    }
    fn extract_state(&self) -> NodeState {
        NodeState::from_json(serde_json::json!({ "state": self.state, "level": self.level }))
    }
    fn inject_state(&mut self, state: &NodeState) {
        if let Some(data) = state.to_json::<serde_json::Value>() {
            if let Some(s) = data
                .get("state")
                .and_then(|v| serde_json::from_value::<EnvState>(v.clone()).ok())
            {
                self.state = s;
            }
            if let Some(l) = data.get("level").and_then(|v| v.as_f64()) {
                self.level = l as f32;
            }
        }
    }
}

#[derive(Clone)]
pub struct AutomationNode {
    pub smooth: SmoothedValue,
}
impl AutomationNode {
    pub fn new() -> Self {
        Self {
            smooth: SmoothedValue::new(0.0, 44100.0, 10.0),
        }
    }
}
impl DspNode for AutomationNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let val = config
            .get("value")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32;
        self.smooth.set_target(val);
        let out = self.smooth.next();
        outputs[0] = [out, out];
    }
}

#[derive(Clone)]
pub struct SequencerNode {
    pub last_step_idx: i32,
}
impl SequencerNode {
    pub fn new() -> Self {
        Self { last_step_idx: -1 }
    }
}
impl DspNode for SequencerNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let bpm = config
            .get("bpm")
            .and_then(|v| v.as_float())
            .unwrap_or(120.0) as f32;
        let steps_data = config.get("steps").and_then(|v| v.as_list());
        let samples_per_step = (60.0 / (bpm * 4.0)) * ctx.sample_rate;
        let current_step_idx = ((ctx.global_sample_index as f32 / samples_per_step) as i32) % 16;
        outputs[0] = [0.0, 0.0];
        if current_step_idx != self.last_step_idx {
            if let Some(steps) = steps_data {
                if let Some(step) = steps.get(current_step_idx as usize) {
                    if let Some(note_val) = step.as_float() {
                        outputs[0] = [1.0, (((note_val as u32) << 8) | 100) as f32];
                    } else {
                        outputs[0] = [2.0, 0.0];
                    }
                }
            }
            self.last_step_idx = current_step_idx;
        }
    }
}
