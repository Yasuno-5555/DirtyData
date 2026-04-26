use dirtydata_core::types::ConfigSnapshot;
use dirtydata_host::{PluginHost, HostError};
use rand::prelude::*;
use rand_pcg::Pcg32;
use std::sync::Arc;
use std::collections::VecDeque;

use super::base::*;

// ──────────────────────────────────────────────
// §1 — Sources
// ──────────────────────────────────────────────

pub struct OscillatorNode {
    phase: f32,
    freq_smooth: Option<SmoothedValue>,
}

impl OscillatorNode {
    pub fn new() -> Self {
        Self { phase: 0.0, freq_smooth: None }
    }
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
            "triangle" => {
                let v = self.phase * 4.0;
                if v < 1.0 { v - 0.0 }
                else if v < 3.0 { 2.0 - v }
                else { v - 4.0 }
            }
            _ => (self.phase * 2.0 * std::f32::consts::PI).sin(),
        };

        outputs[0][0] = val;
        outputs[0][1] = val;

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
        if let Some(val) = state.to_json::<serde_json::Value>() {
            if let Some(phase) = val.get("phase").and_then(|v| v.as_f64()) {
                self.phase = phase as f32;
            }
        }
    }
}

pub struct NoiseNode {
    rng: Pcg32,
}

impl NoiseNode {
    pub fn new(seed: u64) -> Self {
        Self { rng: Pcg32::seed_from_u64(seed) }
    }
}

impl DspNode for NoiseNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let val: f32 = self.rng.random_range(-1.0..1.0);
        outputs[0][0] = val;
        outputs[0][1] = val;
    }
}

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
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        if self.cursor + 1 < self.data.len() {
            outputs[0][0] = self.data[self.cursor];
            outputs[0][1] = self.data[self.cursor + 1];
            self.cursor += 2;
        } else {
            outputs[0][0] = 0.0;
            outputs[0][1] = 0.0;
        }
    }
}

// ──────────────────────────────────────────────
// §2 — Processors
// ──────────────────────────────────────────────

pub struct GainNode {
    gain_smooth: Option<SmoothedValue>,
}

impl GainNode {
    pub fn new() -> Self {
        Self { gain_smooth: None }
    }
}

impl DspNode for GainNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let gain_db_target = config.get("gain_db").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        
        let smooth = self.gain_smooth.get_or_insert_with(|| SmoothedValue::new(gain_db_target, ctx.sample_rate, 10.0));
        let gain_db = smooth.next();
        let linear = 10.0_f32.powf(gain_db / 20.0);
        
        if inputs.len() >= 2 {
            outputs[0][0] = inputs[0] * linear;
            outputs[0][1] = inputs[1] * linear;
        }
    }

    fn update_parameter(&mut self, param: &str, value: f32) {
        if param == "gain_db" {
            if let Some(s) = &mut self.gain_smooth {
                s.set_target(value);
            }
        }
    }
}

impl BiquadFilterNode {
    pub fn new() -> Self {
        Self { z1: [0.0, 0.0], z2: [0.0, 0.0], freq_smooth: None }
    }
}

pub struct BiquadFilterNode {
    z1: [f32; 2],
    z2: [f32; 2],
    freq_smooth: Option<SmoothedValue>,
}

impl DspNode for BiquadFilterNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let freq_target = config.get("frequency").and_then(|v| v.as_float()).unwrap_or(1000.0) as f32;
        let q = config.get("q").and_then(|v| v.as_float()).unwrap_or(0.707) as f32;
        let filter_type = config.get("type").and_then(|v| v.as_string());

        let smooth = self.freq_smooth.get_or_insert_with(|| SmoothedValue::new(freq_target, ctx.sample_rate, 10.0));
        let freq = smooth.next();

        // Simple RBJ Biquad coefficients
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
        if param == "frequency" {
            if let Some(s) = &mut self.freq_smooth {
                s.set_target(value);
            }
        }
    }
}

pub struct CompressorNode {
    envelope: f32,
}

impl CompressorNode {
    pub fn new() -> Self {
        Self { envelope: 0.0 }
    }
}

impl DspNode for CompressorNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let threshold_db = config.get("threshold_db").and_then(|v| v.as_float()).unwrap_or(-20.0) as f32;
        let ratio = config.get("ratio").and_then(|v| v.as_float()).unwrap_or(4.0) as f32;
        let attack_ms = config.get("attack_ms").and_then(|v| v.as_float()).unwrap_or(10.0) as f32;
        let release_ms = config.get("release_ms").and_then(|v| v.as_float()).unwrap_or(100.0) as f32;

        let threshold = 10.0_f32.powf(threshold_db / 20.0);
        let attack_alpha = 1.0 - (-1.0 / (attack_ms * ctx.sample_rate / 1000.0)).exp();
        let release_alpha = 1.0 - (-1.0 / (release_ms * ctx.sample_rate / 1000.0)).exp();

        let (l, r) = if inputs.len() >= 2 {
            (inputs[0], inputs[1])
        } else if inputs.len() == 1 {
            (inputs[0], inputs[0])
        } else {
            (0.0, 0.0)
        };

        let peak = l.abs().max(r.abs());
        let alpha = if peak > self.envelope { attack_alpha } else { release_alpha };
        self.envelope += alpha * (peak - self.envelope);

        let gain = if self.envelope > threshold {
            let over_db = 20.0 * (self.envelope / threshold).log10();
            let reduction_db = over_db * (1.0 - 1.0 / ratio);
            10.0_f32.powf(-reduction_db / 20.0)
        } else {
            1.0
        };

        outputs[0][0] = l * gain;
        outputs[0][1] = r * gain;
    }
}

pub struct ForeignNode {
    host: Option<PluginHost>,
    plugin_name: String,
    buffer_size: usize,
    in_buffer: Vec<f32>,
    out_buffer: Vec<f32>,
    buffer_idx: usize,
    has_crashed: bool,
}

impl ForeignNode {
    pub fn new(plugin_name: String, buffer_size: usize) -> Self {
        Self {
            host: None,
            plugin_name,
            buffer_size,
            in_buffer: vec![0.0; buffer_size],
            out_buffer: vec![0.0; buffer_size],
            buffer_idx: 0,
            has_crashed: false,
        }
    }

    fn ensure_host(&mut self) -> bool {
        if self.has_crashed { return false; }
        if self.host.is_some() { return true; }
        
        match PluginHost::new(&self.plugin_name, self.buffer_size) {
            Ok(h) => {
                self.host = Some(h);
                true
            }
            Err(_) => {
                self.has_crashed = true;
                false
            }
        }
    }
}

impl DspNode for ForeignNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        if !self.ensure_host() {
            // Fallback: Silence or pass through
            outputs[0] = [0.0, 0.0];
            return;
        }

        let input_val = if !inputs.is_empty() { inputs[0] } else { 0.0 };
        self.in_buffer[self.buffer_idx] = input_val;
        
        // We output the delayed sample from the previous block's processing
        // This introduces 1-block latency, which is expected for out-of-process
        outputs[0][0] = self.out_buffer[self.buffer_idx];
        outputs[0][1] = self.out_buffer[self.buffer_idx];

        self.buffer_idx += 1;
        if self.buffer_idx >= self.buffer_size {
            self.buffer_idx = 0;
            // Process the block
            if let Some(host) = &mut self.host {
                if host.process(&self.in_buffer, &mut self.out_buffer).is_err() {
                    self.has_crashed = true;
                    self.host = None;
                    if let Some(flag) = _ctx.crash_flag {
                        flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
        }
    }

    fn update_parameter(&mut self, param: &str, value: f32) {
        if let Some(host) = &mut self.host {
            // Dummy: try to parse param as u32 id
            if let Ok(id) = param.parse::<u32>() {
                let _ = host.set_parameter(id, value);
            }
        }
    }
}

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
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let delay_samples = config.get("delay_samples").and_then(|v| v.as_float()).unwrap_or(4410.0) as usize;
        let feedback = config.get("feedback").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        
        let read_pos = (self.write_pos + self.buffer.len() - delay_samples) % self.buffer.len();
        let delayed = self.buffer[read_pos];
        
        outputs[0][0] = delayed[0];
        outputs[0][1] = delayed[1];

        let in_l = if inputs.len() >= 1 { inputs[0] } else { 0.0 };
        let in_r = if inputs.len() >= 2 { inputs[1] } else { 0.0 };

        self.buffer[self.write_pos] = [
            in_l + delayed[0] * feedback,
            in_r + delayed[1] * feedback,
        ];
        
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
    }
}

// ──────────────────────────────────────────────
// §3 — Math
// ──────────────────────────────────────────────

pub struct AddNode;
impl AddNode { pub fn new() -> Self { Self } }

impl DspNode for AddNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        // Sum all stereo input pairs
        let mut l = 0.0;
        let mut r = 0.0;
        for chunk in inputs.chunks_exact(2) {
            l += chunk[0];
            r += chunk[1];
        }
        outputs[0][0] = l;
        outputs[0][1] = r;
    }
}

pub struct MultiplyNode;
impl MultiplyNode { pub fn new() -> Self { Self } }

impl DspNode for MultiplyNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        if inputs.len() >= 4 {
            outputs[0][0] = inputs[0] * inputs[2];
            outputs[0][1] = inputs[1] * inputs[3];
        } else {
            outputs[0][0] = 0.0;
            outputs[0][1] = 0.0;
        }
    }
}

pub struct ClipNode;
impl ClipNode { pub fn new() -> Self { Self } }

impl DspNode for ClipNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let min = config.get("min").and_then(|v| v.as_float()).unwrap_or(-1.0) as f32;
        let max = config.get("max").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        
        if inputs.len() >= 2 {
            outputs[0][0] = inputs[0].clamp(min, max);
            outputs[0][1] = inputs[1].clamp(min, max);
        }
    }
}

// ──────────────────────────────────────────────
// §4 — Alchemy (Modulation & Time)
// ──────────────────────────────────────────────

pub struct TriggerNode;
impl TriggerNode { pub fn new() -> Self { Self } }

impl DspNode for TriggerNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let trigger_sample = config.get("sample").and_then(|v| v.as_float()).unwrap_or(0.0) as u64;
        let val = if ctx.global_sample_index == trigger_sample { 1.0 } else { 0.0 };
        outputs[0][0] = val;
        outputs[0][1] = val;
    }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum EnvState { Idle, Attack, Decay, Sustain, Release, FastRelease }

pub struct EnvelopeNode {
    state: EnvState,
    level: f32,
}

impl EnvelopeNode {
    pub fn new() -> Self {
        Self { state: EnvState::Idle, level: 0.0 }
    }

    pub fn is_idle(&self) -> bool {
        self.state == EnvState::Idle
    }
}

impl DspNode for EnvelopeNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let a = config.get("attack").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let d = config.get("decay").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let s = config.get("sustain").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let r = config.get("release").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;

        let gate = inputs.get(0).cloned().unwrap_or(0.0) > 0.0;

        match self.state {
            EnvState::Idle => {
                if gate { self.state = EnvState::Attack; }
            }
            EnvState::Attack => {
                if !gate { self.state = EnvState::Release; }
                else {
                    self.level += 1.0 / (a * ctx.sample_rate);
                    if self.level >= 1.0 {
                        self.level = 1.0;
                        self.state = EnvState::Decay;
                    }
                }
            }
            EnvState::Decay => {
                if !gate { self.state = EnvState::Release; }
                else {
                    self.level -= (1.0 - s) / (d * ctx.sample_rate);
                    if self.level <= s {
                        self.level = s;
                        self.state = EnvState::Sustain;
                    }
                }
            }
            EnvState::Sustain => {
                if !gate { self.state = EnvState::Release; }
            }
            EnvState::Release => {
                if gate { self.state = EnvState::Attack; }
                else {
                    // C9 fix: Release rate based on current level, not sustain level.
                    // This ensures release works correctly even when sustain = 0.
                    let release_rate = 1.0 / (r.max(0.001) * ctx.sample_rate);
                    self.level -= release_rate;
                    if self.level <= 0.0 {
                        self.level = 0.0;
                        self.state = EnvState::Idle;
                    }
                }
            }
            EnvState::FastRelease => {
                // Fade out in 5ms to avoid pops
                let fade_out_rate = 1.0 / (0.005 * ctx.sample_rate);
                self.level -= fade_out_rate;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.state = EnvState::Idle;
                }
            }
        }

        outputs[0][0] = self.level;
        outputs[0][1] = self.level;
    }

    fn update_parameter(&mut self, param: &str, _value: f32) {
        if param == "steal" {
            self.state = EnvState::FastRelease;
        }
    }

    fn extract_state(&self) -> NodeState {
        NodeState::from_json(serde_json::json!({
            "state": self.state,
            "level": self.level
        }))
    }

    fn inject_state(&mut self, state: &NodeState) {
        if let Some(data) = state.to_json::<serde_json::Value>() {
            if let Some(s) = data.get("state").and_then(|v| serde_json::from_value::<EnvState>(v.clone()).ok()) {
                self.state = s;
            }
            if let Some(l) = data.get("level").and_then(|v| v.as_f64()) {
                self.level = l as f32;
            }
        }
    }
}

pub struct SequencerNode {
    last_step_idx: i32,
}

impl SequencerNode {
    pub fn new() -> Self {
        Self { last_step_idx: -1 }
    }
}

impl DspNode for SequencerNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let bpm = config.get("bpm").and_then(|v| v.as_float()).unwrap_or(120.0) as f32;
        let steps_data = config.get("steps").and_then(|v| v.as_list());
        
        let samples_per_step = (60.0 / (bpm * 4.0)) * ctx.sample_rate;
        let current_step_idx = ((ctx.global_sample_index as f32 / samples_per_step) as i32) % 16;
        
        outputs[0] = [0.0, 0.0];

        if current_step_idx != self.last_step_idx {
            // Step boundary!
            if let Some(steps) = steps_data {
                let step = &steps[current_step_idx as usize];
                if let Some(note_val) = step.as_float() {
                    // Simple protocol: L=1.0 (NoteOn), R=(Note<<8 | Velocity)
                    // For now, velocity is fixed at 100
                    let note = note_val as u32;
                    let vel = 100u32;
                    outputs[0][0] = 1.0; // NoteOn
                    outputs[0][1] = ((note << 8) | vel) as f32;
                } else {
                    // NoteOff if the previous step had a note?
                    // For now, let's just send NoteOff for ALL notes if step is empty
                    // Or more precisely, we need to track what note we started.
                    outputs[0][0] = 2.0; // NoteOff (All or specific)
                }
            }
            self.last_step_idx = current_step_idx;
        }
    }
}

pub struct AutomationNode;
impl AutomationNode { pub fn new() -> Self { Self } }

impl DspNode for AutomationNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let keyframes = config.get("keyframes").and_then(|v| v.as_list());
        let current_time = ctx.global_sample_index as f64 / ctx.sample_rate as f64;

        let mut val = 0.0;

        if let Some(keys) = keyframes {
            let mut prev_t = 0.0;
            let mut prev_v = 0.0;
            let mut found = false;

            for key in keys {
                if let Some(pair) = key.as_list() {
                    if pair.len() >= 2 {
                        let t = pair[0].as_float().unwrap_or(0.0);
                        let v = pair[1].as_float().unwrap_or(0.0) as f32;

                        if current_time < t {
                            let dt = t - prev_t;
                            if dt > 0.0 {
                                let frac = ((current_time - prev_t) / dt) as f32;
                                val = prev_v + (v - prev_v) * frac;
                            } else {
                                val = v;
                            }
                            found = true;
                            break;
                        }
                        prev_t = t;
                        prev_v = v;
                    }
                }
            }
            if !found {
                val = prev_v;
            }
        }

        outputs[0][0] = val;
        outputs[0][1] = val;
    }
}

pub struct MidiEvent {
    pub sample_index: u64,
    pub message: [u8; 3],
}

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
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, ctx: &ProcessContext) {
        // 1. Drain queue into pending
        while let Ok(event) = self.event_rx.try_recv() {
            self.pending_events.push(event);
        }

        // 2. Process events for current sample
        self.pending_events.retain(|event| {
            if event.sample_index <= ctx.global_sample_index {
                let status = event.message[0] & 0xF0;
                match status {
                    0x90 => { // Note On
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
                    0x80 => { // Note Off
                        self.gate = 0.0;
                    }
                    _ => {}
                }
                false // Handled
            } else {
                true // Future
            }
        });

        // Port 0: Gate
        outputs[0][0] = self.gate;
        outputs[0][1] = self.gate;
        // Port 1: Pitch
        if outputs.len() > 1 {
            outputs[1][0] = self.pitch_hz;
            outputs[1][1] = self.pitch_hz;
        }
        // Port 2: Velocity
        if outputs.len() > 2 {
            outputs[2][0] = self.velocity;
            outputs[2][1] = self.velocity;
        }
    }
}



// ──────────────────────────────────────────────
// §4 — Advanced & Chaos
// ──────────────────────────────────────────────

pub struct WavefolderNode {
    stages: usize,
}

impl WavefolderNode {
    pub fn new() -> Self { Self { stages: 4 } }
}

impl DspNode for WavefolderNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let gain = config.get("gain").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let stages = config.get("stages").and_then(|v| match v {
            dirtydata_core::types::ConfigValue::Int(i) => Some(*i as usize),
            _ => None,
        }).unwrap_or(4);
        
        for i in 0..outputs.len() {
            let mut l = inputs.get(i * 2).cloned().unwrap_or(0.0) * gain;
            let mut r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0) * gain;
            
            for _ in 0..stages {
                l = (l * std::f32::consts::PI * 0.5).sin();
                r = (r * std::f32::consts::PI * 0.5).sin();
            }
            outputs[i] = [l, r];
        }
    }
}

pub struct LorenzNode {
    state: [f32; 3],
    sigma: f32,
    rho: f32,
    beta: f32,
}

impl LorenzNode {
    pub fn new() -> Self {
        Self { state: [0.1, 0.0, 0.0], sigma: 10.0, rho: 28.0, beta: 8.0/3.0 }
    }
}

impl DspNode for LorenzNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let speed = config.get("speed").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let dt = speed / ctx.sample_rate;

        let sigma = self.sigma;
        let rho = self.rho;
        let beta = self.beta;

        // P1: Use zero-allocation fixed-size RK4
        rk4_step_fixed(&mut self.state, dt, 0.0, |state, _t| {
            [
                sigma * (state[1] - state[0]),
                state[0] * (rho - state[2]) - state[1],
                state[0] * state[1] - beta * state[2],
            ]
        });

        // Soft clamp to prevent blow-up
        for s in &mut self.state {
            *s = s.clamp(-100.0, 100.0);
            if !s.is_finite() { *s = 0.1; }
        }

        // Output X, Y, Z as 3 mono signals (mapped to stereo ports)
        outputs[0] = [self.state[0] * 0.05, self.state[1] * 0.05];
        if outputs.len() > 1 {
            outputs[1] = [self.state[2] * 0.05, 0.0];
        }
    }
}

pub struct MackeyGlassNode {
    history: VecDeque<f32>,
    tau_samples: usize,
    beta: f32,
    gamma: f32,
    n: f32,
    current_x: f32,
}

impl MackeyGlassNode {
    pub fn new(tau_ms: f32, sample_rate: f32) -> Self {
        let tau_samples = (tau_ms * 0.001 * sample_rate) as usize;
        let mut history = VecDeque::with_capacity(tau_samples + 1);
        for _ in 0..=tau_samples { history.push_back(0.5); }
        Self { history, tau_samples, beta: 2.0, gamma: 1.0, n: 10.0, current_x: 0.5 }
    }
}

impl DspNode for MackeyGlassNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let speed = config.get("speed").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let dt = speed / ctx.sample_rate;
        
        let x_tau = *self.history.front().unwrap();
        
        // Simple integration for Mackey-Glass (RK4-ish applied locally)
        let f = |x: f32, xt: f32| self.beta * xt / (1.0 + xt.powf(self.n)) - self.gamma * x;
        
        let k1 = f(self.current_x, x_tau);
        let k2 = f(self.current_x + k1 * dt * 0.5, x_tau);
        let k3 = f(self.current_x + k2 * dt * 0.5, x_tau);
        let k4 = f(self.current_x + k3 * dt, x_tau);
        
        self.current_x += (dt / 6.0) * (k1 + 2.0 * k2 + 2.0 * k3 + k4);
        self.history.push_back(self.current_x);
        self.history.pop_front();
        
        outputs[0] = [self.current_x, self.current_x];
    }
}

pub struct GrayScottNode {
    u: [Vec<f32>; 2],  // P2: Double buffer (no per-sample clone)
    v: [Vec<f32>; 2],
    current: usize,
    size: usize,
    f: f32,
    k: f32,
    du: f32,
    dv: f32,
}

impl GrayScottNode {
    pub fn new(size: usize) -> Self {
        let mut u0 = vec![1.0; size];
        let mut v0 = vec![0.0; size];
        for i in (size/2 - 5)..(size/2 + 5) { v0[i] = 0.5; }
        Self {
            u: [u0.clone(), vec![0.0; size]],
            v: [v0.clone(), vec![0.0; size]],
            current: 0,
            size, f: 0.0545, k: 0.062, du: 0.1, dv: 0.05,
        }
    }
}

impl DspNode for GrayScottNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let cur = self.current;
        let nxt = 1 - cur;
        
        for i in 0..self.size {
            let prev = if i == 0 { self.size - 1 } else { i - 1 };
            let next = if i == self.size - 1 { 0 } else { i + 1 };
            
            let u_val = self.u[cur][i];
            let v_val = self.v[cur][i];
            let lap_u = self.u[cur][prev] + self.u[cur][next] - 2.0 * u_val;
            let lap_v = self.v[cur][prev] + self.v[cur][next] - 2.0 * v_val;
            let uv2 = u_val * v_val * v_val;
            
            self.u[nxt][i] = (u_val + self.du * lap_u - uv2 + self.f * (1.0 - u_val)).clamp(0.0, 1.5);
            self.v[nxt][i] = (v_val + self.dv * lap_v + uv2 - (self.f + self.k) * v_val).clamp(0.0, 1.5);
        }
        
        self.current = nxt;
        outputs[0] = [self.u[nxt][self.size/2] * 2.0 - 1.0, self.v[nxt][self.size/2] * 2.0 - 1.0];
    }
}

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
            let trig = inputs.get(i * 2 + 2).cloned().unwrap_or(0.0); // Assume 3rd input is trigger
            
            if trig > 0.5 && self.last_trig <= 0.5 {
                self.last_val = [sig_l, sig_r];
            }
            self.last_trig = trig;
            outputs[i] = self.last_val;
        }
    }
}

pub struct ClockNode {
    phase: f32,
}

impl ClockNode {
    pub fn new() -> Self { Self { phase: 0.0 } }
}

impl DspNode for ClockNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let bpm = config.get("bpm").and_then(|v| v.as_float()).unwrap_or(120.0) as f32;
        let division = config.get("division").and_then(|v| v.as_float()).unwrap_or(4.0) as f32; // Default 1/4
        
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
                if self.rng.random::<f32>() < prob {
                    out = 1.0;
                }
            }
            outputs[i] = [out, out];
        }
    }
}

pub struct ReverbNode {
    delays: Vec<VecDeque<f32>>,
    feedback_matrix: [[f32; 4]; 4],
}

impl ReverbNode {
    pub fn new(sample_rate: f32) -> Self {
        let delay_times = [0.037, 0.043, 0.051, 0.061]; // Primes in seconds
        let delays = delay_times.iter().map(|&t| {
            let size = (t * sample_rate) as usize;
            let mut dq = VecDeque::with_capacity(size);
            for _ in 0..size { dq.push_back(0.0); }
            dq
        }).collect();

        // 4x4 Hadamard matrix for diffusion
        let h = 0.5;
        let feedback_matrix = [
            [h, h, h, h],
            [h, -h, h, -h],
            [h, h, -h, -h],
            [h, -h, -h, h],
        ];

        Self { delays, feedback_matrix }
    }
}

impl DspNode for ReverbNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let decay = config.get("decay").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let mix = config.get("mix").and_then(|v| v.as_float()).unwrap_or(0.3) as f32;

        for i in 0..outputs.len() {
            let input_l = inputs.get(i * 2).cloned().unwrap_or(0.0);
            let input_r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0);
            let mono_in = (input_l + input_r) * 0.5;

            // 1. Read delay outputs
            let mut y = [0.0; 4];
            for j in 0..4 {
                y[j] = *self.delays[j].front().unwrap();
            }

            // 2. Compute feedback
            let mut fb = [0.0; 4];
            for row in 0..4 {
                for col in 0..4 {
                    fb[row] += self.feedback_matrix[row][col] * y[col];
                }
            }

            // 3. Inject input and write back to delays
            for j in 0..4 {
                self.delays[j].push_back(mono_in + fb[j] * decay);
                self.delays[j].pop_front();
            }

            // 4. Output mix (L=Y0+Y1, R=Y2+Y3 for pseudo-stereo)
            let wet_l = y[0] + y[1];
            let wet_r = y[2] + y[3];
            
            outputs[i] = [
                input_l * (1.0 - mix) + wet_l * mix,
                input_r * (1.0 - mix) + wet_r * mix
            ];
        }
    }
}

pub struct Grain {
    pos: f32,
    duration_samples: f32,
    current_sample: f32,
    active: bool,
}

pub struct GranularNode {
    buffer: Vec<[f32; 2]>,
    write_pos: usize,
    grains: Vec<Grain>,
    next_grain_samples: f32,
}

impl GranularNode {
    pub fn new(sample_rate: f32) -> Self {
        let buf_size = (sample_rate * 2.0) as usize; // 2 seconds buffer
        let mut grains = Vec::new();
        for _ in 0..16 {
            grains.push(Grain { pos: 0.0, duration_samples: 0.0, current_sample: 0.0, active: false });
        }
        Self {
            buffer: vec![[0.0, 0.0]; buf_size],
            write_pos: 0,
            grains,
            next_grain_samples: 0.0,
        }
    }
}

impl DspNode for GranularNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let pos_norm = config.get("position").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let size_ms = config.get("size").and_then(|v| v.as_float()).unwrap_or(50.0) as f32;
        let density = config.get("density").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        
        let size_samples = (size_ms * 0.001 * ctx.sample_rate) as f32;
        
        // 1. Record input
        for i in 0..outputs.len() {
            let in_l = inputs.get(i * 2).cloned().unwrap_or(0.0);
            let in_r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0);
            self.buffer[self.write_pos] = [in_l, in_r];
            self.write_pos = (self.write_pos + 1) % self.buffer.len();

            // 2. Schedule new grain
            self.next_grain_samples -= 1.0;
            if self.next_grain_samples <= 0.0 {
                if let Some(grain) = self.grains.iter_mut().find(|g| !g.active) {
                    grain.active = true;
                    grain.current_sample = 0.0;
                    grain.duration_samples = size_samples;
                    // Jittered position
                    let jitter = (rand::random::<f32>() - 0.5) * 0.05;
                    grain.pos = (pos_norm + jitter).clamp(0.0, 1.0);
                }
                self.next_grain_samples = (1.0 - density) * size_samples * 0.5 + 100.0;
            }

            // 3. Process grains
            let mut mixed = [0.0, 0.0];
            for grain in self.grains.iter_mut().filter(|g| g.active) {
                let norm_idx = grain.current_sample / grain.duration_samples;
                
                // Simple triangle window
                let window = 1.0 - (2.0 * norm_idx - 1.0).abs();
                
                let read_base = (grain.pos * (self.buffer.len() as f32 - 1.0)) as usize;
                let read_idx = (read_base + grain.current_sample as usize) % self.buffer.len();
                let val = self.buffer[read_idx];
                
                mixed[0] += val[0] * window;
                mixed[1] += val[1] * window;
                
                grain.current_sample += 1.0;
                if grain.current_sample >= grain.duration_samples {
                    grain.active = false;
                }
            }

            outputs[i] = mixed;
        }
    }
}

pub struct WasmNode {
    instance: Option<wasmtime::Instance>,
    store: Option<wasmtime::Store<()>>,
    process_fn: Option<wasmtime::TypedFunc<(f32, f32), i64>>,
    failed: bool,
}

impl WasmNode {
    pub fn new() -> Self {
        Self { instance: None, store: None, process_fn: None, failed: false }
    }

    fn init(&mut self, path: &str) -> anyhow::Result<()> {
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::from_file(&engine, path)?;
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[])?;
        
        let process_fn = instance.get_typed_func::<(f32, f32), i64>(&mut store, "process")?;
        
        self.instance = Some(instance);
        self.store = Some(store);
        self.process_fn = Some(process_fn);
        Ok(())
    }
}

impl DspNode for WasmNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        if self.instance.is_none() && !self.failed {
            if let Some(path) = config.get("path").and_then(|v| v.as_string()) {
                if let Err(e) = self.init(path) {
                    eprintln!("Failed to init WasmNode: {}", e);
                    self.failed = true;
                }
            }
        }

        if let (Some(store), Some(f)) = (self.store.as_mut(), self.process_fn.as_mut()) {
            for i in 0..outputs.len() {
                let in_l = inputs.get(i * 2).cloned().unwrap_or(0.0);
                let in_r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0);
                
                match f.call(&mut *store, (in_l, in_r)) {
                    Ok(res) => {
                        // Unpack two f32 from i64
                        let out_l = f32::from_bits((res >> 32) as u32);
                        let out_r = f32::from_bits(res as u32);
                        outputs[i] = [out_l, out_r];
                    }
                    Err(_) => {
                        outputs[i] = [in_l, in_r];
                    }
                }
            }
        } else {
            // Bypass
            for i in 0..outputs.len() {
                outputs[i] = [
                    inputs.get(i * 2).cloned().unwrap_or(0.0),
                    inputs.get(i * 2 + 1).cloned().unwrap_or(0.0)
                ];
            }
        }
    }
}

// ──────────────────────────────────────────────
// §7.3 Missing Gaps Implementation
// ──────────────────────────────────────────────

/// Logic operations on signals (Gate/CV logic).
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

use rustfft::{FftPlanner, num_complex::Complex};

/// Spectral Freeze Node.
pub struct SpectralFreezeNode {
    size: usize,
    buffer: Vec<f32>,
    fft_result: Vec<Complex<f32>>,
    frozen: bool,
    write_pos: usize,
    read_pos: usize,
}

impl SpectralFreezeNode {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            buffer: vec![0.0; size],
            fft_result: vec![Complex::default(); size],
            frozen: false,
            write_pos: 0,
            read_pos: 0,
        }
    }
}

impl DspNode for SpectralFreezeNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let freeze = config.get("freeze").and_then(|v| v.as_bool()).unwrap_or(false);
        let input = inputs.get(0).cloned().unwrap_or(0.0);

        if freeze && !self.frozen {
            // Perform FFT once and freeze
            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(self.size);
            let mut complex_buf: Vec<Complex<f32>> = self.buffer.iter().map(|&x| Complex::new(x, 0.0)).collect();
            fft.process(&mut complex_buf);
            self.fft_result = complex_buf;
            self.frozen = true;
            
            // Generate time-domain frozen signal via IFFT
            let ifft = planner.plan_fft_inverse(self.size);
            let mut inv_buf = self.fft_result.clone();
            ifft.process(&mut inv_buf);
            for (i, c) in inv_buf.iter().enumerate() {
                self.buffer[i] = c.re / self.size as f32;
            }
        } else if !freeze {
            self.frozen = false;
        }

        // Fill input buffer if not frozen
        if !self.frozen {
            self.buffer[self.write_pos] = input;
            self.write_pos = (self.write_pos + 1) % self.size;
        }

        // Output logic: if frozen, loop the frozen buffer
        let out_val = if self.frozen {
             let v = self.buffer[self.read_pos];
             self.read_pos = (self.read_pos + 1) % self.size;
             v
        } else {
            input
        };

        for out in outputs.iter_mut() {
            *out = [out_val, out_val];
        }
    }
}

/// FFT Convolution Node.
pub struct FFTConvolveNode {
    size: usize,
    input_buffer: Vec<f32>,
    impulse_buffer: Vec<f32>,
    result_buffer: Vec<f32>,
    pos: usize,
}

impl FFTConvolveNode {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            input_buffer: vec![0.0; size],
            impulse_buffer: vec![0.0; size],
            result_buffer: vec![0.0; size],
            pos: 0,
        }
    }
}

impl DspNode for FFTConvolveNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).cloned().unwrap_or(0.0);
        let impulse = inputs.get(1).cloned().unwrap_or(0.0);

        self.input_buffer[self.pos] = input;
        self.impulse_buffer[self.pos] = impulse;
        self.pos += 1;

        if self.pos >= self.size {
            // Block process
            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(self.size);
            
            let mut in_complex: Vec<Complex<f32>> = self.input_buffer.iter().map(|&x| Complex::new(x, 0.0)).collect();
            let mut imp_complex: Vec<Complex<f32>> = self.impulse_buffer.iter().map(|&x| Complex::new(x, 0.0)).collect();
            
            fft.process(&mut in_complex);
            fft.process(&mut imp_complex);
            
            // Multiply in frequency domain
            for i in 0..self.size {
                in_complex[i] *= imp_complex[i];
            }
            
            let ifft = planner.plan_fft_inverse(self.size);
            ifft.process(&mut in_complex);
            
            for (i, c) in in_complex.iter().enumerate() {
                self.result_buffer[i] = c.re / self.size as f32;
            }
            self.pos = 0;
        }

        let out_val = self.result_buffer[self.pos];

        for out in outputs.iter_mut() {
            *out = [out_val, out_val];
        }
    }
}

/// OSC Output Node.
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

        // Rate-limited sending: only send if value changed significantly
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

// ──────────────────────────────────────────────
// Phase 5.5 — Feedback Hell
// ──────────────────────────────────────────────

/// A node that provides a 1-sample delay, enabling explicit feedback loops.
/// Use this to break causal cycles in the graph.
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
        // Output the value from the PREVIOUS sample
        outputs[0] = self.latch;
        
        // Capture the CURRENT sample for the next cycle
        if inputs.len() >= 2 {
            self.latch = [inputs[0], inputs[1]];
        }
    }
}

// ──────────────────────────────────────────────
// §7 — Containers (Encapsulation)
// ──────────────────────────────────────────────

pub struct InputProxyNode { value: f32 }
impl InputProxyNode { pub fn new() -> Self { Self { value: 0.0 } } }
impl DspNode for InputProxyNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        outputs[0] = [self.value, self.value];
    }
    fn update_parameter(&mut self, _param: &str, value: f32) { self.value = value; }
}

pub struct OutputProxyNode;
impl OutputProxyNode { pub fn new() -> Self { Self } }
impl DspNode for OutputProxyNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let val = inputs.get(0).cloned().unwrap_or(0.0);
        outputs[0] = [val, val];
    }
}

pub struct SubGraphNode {
    runner: Option<crate::DspRunner>,
    last_graph_hash: String,
}

impl SubGraphNode {
    pub fn new() -> Self {
        Self { runner: None, last_graph_hash: String::new() }
    }
}

impl DspNode for SubGraphNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let graph_json = config.get("graph_json").and_then(|v| v.as_string()).map(|s| s.as_str()).unwrap_or("");
        let hash = blake3::hash(graph_json.as_bytes()).to_string();

        if hash != self.last_graph_hash && !graph_json.is_empty() {
            if let Ok(graph) = serde_json::from_str::<dirtydata_core::ir::Graph>(&graph_json) {
                self.runner = Some(crate::DspRunner::new(graph, None, ctx.sample_rate));
                self.last_graph_hash = hash;
            }
        }

        if let Some(runner) = &mut self.runner {
            let mut proxy_ids = Vec::new();
            for (id, n) in &runner.get_graph().nodes {
                if n.kind == dirtydata_core::types::NodeKind::InputProxy {
                    proxy_ids.push(*id);
                }
            }
            for (id, node) in runner.nodes_mut() {
                if proxy_ids.contains(id) {
                    node.update_parameter("value", inputs.get(0).cloned().unwrap_or(0.0));
                }
            }
            
            let sub_out = runner.process_sample(ctx);
            outputs[0] = sub_out;
        } else {
            for o in outputs { *o = [0.0, 0.0]; }
        }
    }
}

// ──────────────────────────────────────────────
// Tier S Analog DSP Nodes (Topology Preserving)
// ──────────────────────────────────────────────

pub struct ZdfLadderNode {
    inner: dirtydata_dsp_zdf::ZdfLadder,
}
impl ZdfLadderNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_zdf::ZdfLadder::new(sample_rate) }
    }
}
impl DspNode for ZdfLadderNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let cutoff = config.get("cutoff").and_then(|v| v.as_float()).unwrap_or(1000.0) as f32;
        let res = config.get("resonance").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        let drive = config.get("drive").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        
        let out = self.inner.process(input, cutoff, res, drive);
        for o in outputs { *o = [out, out]; }
    }
}

pub struct SvfNode {
    inner: dirtydata_dsp_svf::Svf,
}
impl SvfNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_svf::Svf::new(sample_rate) }
    }
}
impl DspNode for SvfNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let cutoff = config.get("cutoff").and_then(|v| v.as_float()).unwrap_or(1000.0) as f32;
        let q = config.get("q").and_then(|v| v.as_float()).unwrap_or(0.707) as f32;
        let mode = config.get("mode").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        let drive = config.get("drive").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        
        let svf_out = if drive > 0.01 {
            self.inner.process_nonlinear(input, cutoff, q, drive)
        } else {
            self.inner.process(input, cutoff, q)
        };
        let out = match mode as i32 {
            0 => svf_out.lp,
            1 => svf_out.hp,
            2 => svf_out.bp,
            3 => svf_out.notch,
            4 => svf_out.ap,
            _ => svf_out.peak,
        };
        for o in outputs { *o = [out, out]; }
    }
}

pub struct DiodeClipperNode {
    inner: dirtydata_dsp_clipper::DiodeClipper,
}
impl DiodeClipperNode {
    pub fn new() -> Self {
        Self { inner: dirtydata_dsp_clipper::DiodeClipper::new() }
    }
}
impl DspNode for DiodeClipperNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let drive = config.get("drive").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let asymmetry = config.get("asymmetry").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        
        let out = self.inner.process(input, drive, asymmetry);
        for o in outputs { *o = [out, out]; }
    }
}

pub struct BbdDelayNode {
    inner: dirtydata_dsp_bbd::BbdDelay,
}
impl BbdDelayNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_bbd::BbdDelay::new(sample_rate, 2.0) }
    }
}
impl DspNode for BbdDelayNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let time_ms = config.get("time_ms").and_then(|v| v.as_float()).unwrap_or(300.0) as f32;
        let feedback = config.get("feedback").and_then(|v| v.as_float()).unwrap_or(0.3) as f32;
        let dirt = config.get("dirt").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        
        let out = self.inner.process(input, time_ms, feedback, dirt);
        for o in outputs { *o = [out, out]; }
    }
}

// ──────────────────────────────────────────────
// Tier A Physical Modeling DSP Nodes
// ──────────────────────────────────────────────

pub struct WdfSimpleRcNode {
    inner: dirtydata_dsp_wdf::WdfSimpleRc,
}
impl WdfSimpleRcNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_wdf::WdfSimpleRc::new(1000.0, 1e-6, sample_rate) }
    }
}
impl DspNode for WdfSimpleRcNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let out = self.inner.process(input);
        for o in outputs { *o = [out, out]; }
    }
}

pub struct WdfDiodeClipperNode {
    inner: dirtydata_dsp_wdf::WdfDiodeClipper,
}
impl WdfDiodeClipperNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_wdf::WdfDiodeClipper::new(4700.0, 10e-9, sample_rate) }
    }
}
impl DspNode for WdfDiodeClipperNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let out = self.inner.process(input);
        for o in outputs { *o = [out, out]; }
    }
}

pub struct KarplusStrongNode {
    inner: dirtydata_dsp_ks::KarplusStrong,
}
impl KarplusStrongNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_ks::KarplusStrong::new(sample_rate) }
    }
}
impl DspNode for KarplusStrongNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let freq = config.get("freq").and_then(|v| v.as_float()).unwrap_or(440.0) as f32;
        let damping = config.get("damping").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let dispersion = config.get("dispersion").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let pick_pos = config.get("pick_pos").and_then(|v| v.as_float()).unwrap_or(0.2) as f32;
        
        let out = self.inner.process(input, freq, damping, dispersion, pick_pos);
        for o in outputs { *o = [out, out]; }
    }
}

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
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        
        let material = config.get("material").and_then(|v| v.as_float()).unwrap_or(0.0) as u32;
        let freq = config.get("base_freq").and_then(|v| v.as_float()).unwrap_or(440.0) as f32;
        let bright = config.get("brightness").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        
        if material != self.last_material || (freq - self.last_freq).abs() > 0.1 || (bright - self.last_bright).abs() > 0.01 {
            self.inner.set_material(material, freq, bright);
            self.last_material = material;
            self.last_freq = freq;
            self.last_bright = bright;
        }
        
        let out = self.inner.process(input);
        for o in outputs { *o = [out, out]; }
    }
}

pub struct SpringReverbNode {
    inner: dirtydata_dsp_spring::SpringReverb,
}
impl SpringReverbNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_spring::SpringReverb::new(sample_rate) }
    }
}
impl DspNode for SpringReverbNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let decay = config.get("decay").and_then(|v| v.as_float()).unwrap_or(0.8) as f32;
        let dispersion = config.get("dispersion").and_then(|v| v.as_float()).unwrap_or(0.6) as f32;
        
        let out = self.inner.process(input, decay, dispersion);
        for o in outputs { *o = [out, out]; }
    }
}

// ──────────────────────────────────────────────
// Tier B "For Madmen" DSP Nodes (Chaos, Ecosystems, Degradation)
// ──────────────────────────────────────────────

pub struct ChuaCircuitNode {
    inner: dirtydata_dsp_chaos::ChuaCircuit,
}
impl ChuaCircuitNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_chaos::ChuaCircuit::new(sample_rate) }
    }
}
impl DspNode for ChuaCircuitNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let alpha = config.get("alpha").and_then(|v| v.as_float()).unwrap_or(15.6) as f32;
        let beta = config.get("beta").and_then(|v| v.as_float()).unwrap_or(28.0) as f32;
        let rate = config.get("rate").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        
        let out = self.inner.process(alpha, beta, rate);
        for o in outputs { *o = [out, out]; }
    }
}

pub struct ReactionDiffusionNode {
    inner: dirtydata_dsp_reaction::ReactionDiffusion,
}
impl ReactionDiffusionNode {
    pub fn new() -> Self {
        Self { inner: dirtydata_dsp_reaction::ReactionDiffusion::new(256) }
    }
}
impl DspNode for ReactionDiffusionNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let da = config.get("da").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let db = config.get("db").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let f = config.get("f").and_then(|v| v.as_float()).unwrap_or(0.055) as f32;
        let k = config.get("k").and_then(|v| v.as_float()).unwrap_or(0.062) as f32;
        
        let out = self.inner.process(input, da, db, f, k);
        for o in outputs { *o = [out, out]; }
    }
}

pub struct TapeMachineNode {
    inner: dirtydata_dsp_tape::TapeMachine,
}
impl TapeMachineNode {
    pub fn new(sample_rate: f32) -> Self {
        Self { inner: dirtydata_dsp_tape::TapeMachine::new(sample_rate) }
    }
}
impl DspNode for TapeMachineNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let drive = config.get("drive").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let wow = config.get("wow").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let flutter = config.get("flutter").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let bias = config.get("bias").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        
        let out = self.inner.process(input, drive, wow, flutter, bias);
        for o in outputs { *o = [out, out]; }
    }
}

// ──────────────────────────────────────────────
// Priority SSS: Matrix / Routing Hell
// ──────────────────────────────────────────────

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
        // Simple 2x2 matrix for now
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

// ──────────────────────────────────────────────
// Priority SS: CV Civilization
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// Priority S: Destruction
// ──────────────────────────────────────────────

pub struct BitCrushNode {
    inner: dirtydata_dsp_destruction::BitCrush,
}
impl BitCrushNode {
    pub fn new() -> Self { Self { inner: dirtydata_dsp_destruction::BitCrush::new() } }
}
impl DspNode for BitCrushNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let bits = config.get("bits").and_then(|v| v.as_float()).unwrap_or(8.0) as f32;
        let srr = config.get("srr").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let out = self.inner.process(input, bits, srr);
        for o in outputs { *o = [out, out]; }
    }
}

// ──────────────────────────────────────────────
// Priority S: Control as Instrument (Maths)
// ──────────────────────────────────────────────

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

// ──────────────────────────────────────────────
// Priority GOD: Circuit Sandbox (MNA Solver)
// ──────────────────────────────────────────────

pub struct CircuitSandboxNode {
    solver: dirtydata_dsp_circuit::MnaSolver,
    // Live probe history (ring buffer for popup oscilloscope)
    probe_voltages: Vec<f32>,
}

impl CircuitSandboxNode {
    pub fn new(sample_rate: f32) -> Self {
        let mut solver = dirtydata_dsp_circuit::MnaSolver::new(1.0 / sample_rate as f64);
        
        // --- PRESET: Transistor-based Diode Ladder (Moog-ish) ---
        // Node 0: Ground
        // Node 1: Signal In (Voltage Source)
        // Node 2: Cutoff Control (Voltage Source)
        // Nodes 3-6: Ladder Stages
        solver.set_num_nodes(7);
        
        // Signal Input
        solver.add_element(dirtydata_dsp_circuit::CircuitElement::VoltageSource {
            pos: dirtydata_dsp_circuit::NodeId(1), neg: dirtydata_dsp_circuit::NodeId(0), voltage: 0.0,
        });
        
        // Cutoff Control (thermal voltage biasing)
        solver.add_element(dirtydata_dsp_circuit::CircuitElement::VoltageSource {
            pos: dirtydata_dsp_circuit::NodeId(2), neg: dirtydata_dsp_circuit::NodeId(0), voltage: 0.7,
        });

        // 4-stage Diode Ladder (discrete components)
        for i in 0..4 {
            let n_in = if i == 0 { 1 } else { 3 + i - 1 };
            let n_out = 3 + i;
            
            // Diode Pair (nonlinear saturation)
            solver.add_element(dirtydata_dsp_circuit::CircuitElement::Diode {
                a: dirtydata_dsp_circuit::NodeId(n_in), 
                k: dirtydata_dsp_circuit::NodeId(n_out), 
                material: dirtydata_dsp_circuit::Material::Silicon,
                is: 1e-12,
            });
            // Stage Capacitor
            solver.add_element(dirtydata_dsp_circuit::CircuitElement::Capacitor {
                a: dirtydata_dsp_circuit::NodeId(n_out), 
                b: dirtydata_dsp_circuit::NodeId(0), 
                value: 1e-8, 
                state_v: 0.0,
                tolerance: 0.1,
                material: dirtydata_dsp_circuit::Material::Ceramic,
            });
        }
        
        Self { solver, probe_voltages: vec![0.0; 256] }
    }
}

impl DspNode for CircuitSandboxNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let input = inputs.get(0).copied().unwrap_or(0.0) as f64;
        let cutoff = config.get("cutoff").and_then(|v| v.as_float()).unwrap_or(0.7) as f64;
        
        // --- Priority MONSTER: Environmental Sabotage ---
        if let Some(temp) = config.get("temp_c").and_then(|v| v.as_float()) {
            self.solver.context.temperature_c = temp as f64;
        }
        if let Some(drift) = config.get("drift").and_then(|v| v.as_float()) {
            self.solver.context.global_drift = drift as f64;
        }
        if let Some(vcc) = config.get("vcc").and_then(|v| v.as_float()) {
            self.solver.context.vcc = vcc as f64;
        }

        // Set parameters via handles...
        if let Some(dirtydata_dsp_circuit::CircuitElement::VoltageSource { voltage, .. }) = self.solver.add_element_dummy_handle(0) {
            *voltage = input;
        }
        if let Some(dirtydata_dsp_circuit::CircuitElement::VoltageSource { voltage, .. }) = self.solver.add_element_dummy_handle(1) {
            *voltage = cutoff;
        }

        let state = self.solver.solve();
        let out = state.voltages.get(6).copied().unwrap_or(0.0) as f32; // Last stage
        
        // --- §SSS: Visual Nodal Probing ---
        // Expose state to GUI via ctx.shared_state
        if ctx.sample_rate > 0.0 {
            // Push to ring buffer for "Nodal Probing"
            self.probe_voltages.rotate_left(1);
            if let Some(last) = self.probe_voltages.last_mut() { *last = out; }
            
            // If iterations is high, the circuit is "screaming" (vibrate UI)
            if state.iterations > 40 {
                // Trigger visual vibration event (conceptual)
            }
        }
        
        for o in outputs { *o = [out, out]; }
    }
}

// ──────────────────────────────────────────────
// Priority SSS: Circuit Module (Custom reusable MNA nodes)
// ──────────────────────────────────────────────

pub struct CircuitModuleNode {
    solver: dirtydata_dsp_circuit::MnaSolver,
    /// Maps audio input index to internal voltage source index
    input_v_sources: Vec<usize>,
    /// Maps internal node IDs to audio output indices
    output_nodes: Vec<usize>,
}

impl CircuitModuleNode {
    pub fn new(sample_rate: f32, definition_json: &str) -> Option<Self> {
        let def: dirtydata_core::types::CircuitDefinition = serde_json::from_str(definition_json).ok()?;
        let elements: Vec<dirtydata_dsp_circuit::CircuitElement> = serde_json::from_str(&def.elements_json).ok()?;
        
        let mut solver = dirtydata_dsp_circuit::MnaSolver::new(1.0 / sample_rate as f64);
        
        // Find max node ID to set_num_nodes
        let mut max_node = 0;
        for el in &elements {
            match el {
                dirtydata_dsp_circuit::CircuitElement::Resistor { a, b, .. } => { max_node = max_node.max(a.0).max(b.0); }
                dirtydata_dsp_circuit::CircuitElement::Capacitor { a, b, .. } => { max_node = max_node.max(a.0).max(b.0); }
                dirtydata_dsp_circuit::CircuitElement::Diode { a, k, .. } => { max_node = max_node.max(a.0).max(k.0); }
                dirtydata_dsp_circuit::CircuitElement::VoltageSource { pos, neg, .. } => { max_node = max_node.max(pos.0).max(neg.0); }
            }
        }
        solver.set_num_nodes(max_node + 1);

        let mut input_v_sources = Vec::new();
        for (_, &node_id) in &def.input_mappings {
            let idx = solver.num_elements(); // Track position of voltage source
            solver.add_element(dirtydata_dsp_circuit::CircuitElement::VoltageSource {
                pos: dirtydata_dsp_circuit::NodeId(node_id),
                neg: dirtydata_dsp_circuit::NodeId(0), // Ground reference
                voltage: 0.0,
            });
            input_v_sources.push(idx);
        }

        for el in elements { solver.add_element(el); }
        
        let mut output_nodes = Vec::new();
        for (_, &node_id) in &def.output_mappings {
            output_nodes.push(node_id);
        }

        Some(Self { solver, input_v_sources, output_nodes })
    }
}

impl DspNode for CircuitModuleNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, ctx: &ProcessContext) {
        // 1. Inject samples as voltages
        for (i, &v_idx) in self.input_v_sources.iter().enumerate() {
            if let Some(val) = inputs.get(i) {
                if let Some(dirtydata_dsp_circuit::CircuitElement::VoltageSource { voltage, .. }) = self.solver.add_element_dummy_handle(v_idx) {
                    *voltage = *val as f64;
                }
            }
        }

        // 2. Step the physics
        let state = self.solver.solve();
        
        if let (Some(info), Some(id)) = (ctx.convergence_info.as_ref(), ctx.node_id) {
            info.insert(id, state.iterations);
        }

        if !state.converged {
            if let (Some(diag), Some(id)) = (ctx.node_diagnostics.as_ref(), ctx.node_id) {
                if let Some(culprit) = state.failure_culprit {
                    diag.insert(id, crate::DiagnosticRecord {
                        message: culprit,
                        severity: crate::DiagnosticSeverity::Error,
                        timestamp: ctx.global_sample_index,
                    });
                }
            }
        }

        // 3. Extract voltages as samples
        for (i, &node_id) in self.output_nodes.iter().enumerate() {
            if let Some(out_pair) = outputs.get_mut(i) {
                let v = state.voltages.get(node_id).copied().unwrap_or(0.0) as f32;
                *out_pair = [v, v];
            }
        }
    }
}

// ──────────────────────────────────────────────
// Priority GOD: Vocal Tract Physical Modeling
// ──────────────────────────────────────────────

pub struct VocalTractNode {
    inner: dirtydata_dsp_vocal::VocalTract,
}
impl VocalTractNode {
    pub fn new(sample_rate: f32) -> Self {
        let _ = sample_rate;
        Self { inner: dirtydata_dsp_vocal::VocalTract::new(44) } // 44 sections
    }
}
impl DspNode for VocalTractNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let freq = config.get("pitch").and_then(|v| v.as_float()).unwrap_or(110.0) as f32;
        let tongue_x = config.get("tongue_x").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let tongue_y = config.get("tongue_y").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let tension = config.get("tension").and_then(|v| v.as_float()).unwrap_or(0.8) as f32;
        let velum = config.get("velum").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        
        // Check for vowel preset
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
        for o in outputs { *o = [out, out]; }
    }
}



