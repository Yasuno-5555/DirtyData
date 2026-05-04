//! Envelope Module — ADSR
//!
//! # Parameters
//! - ATTACK, DECAY, SUSTAIN, RELEASE
//!
//! # Inputs
//! - GATE: ゲート入力
//! - TRIG: リトリガー入力
//!
//! # Outputs
//! - OUT: エンベロープ出力

use crate::signal::{
    GateTracker, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection,
    RackDspNode, RackProcessContext, SignalType, TriggerDetector,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct EnvelopeModule {
    trackers: [GateTracker; 16],
    detectors: [TriggerDetector; 16],
    states: [EnvelopeState; 16],
    values: [f32; 16],
    sample_rate: f32,
}

impl EnvelopeModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            trackers: [GateTracker::new(); 16],
            detectors: [TriggerDetector::new(); 16],
            states: [(); 16].map(|_| EnvelopeState::Idle),
            values: [0.0; 16],
            sample_rate,
        }
    }
}

impl RackDspNode for EnvelopeModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        ctx: &RackProcessContext,
    ) {
        let a_knob = params[0];
        let d_knob = params[1];
        let s_knob = params[2];
        let r_knob = params[3];

        let dt = 1.0 / self.sample_rate;

        for i in 0..16 {
            let (gate_active, gate_rising, _) = self.trackers[i].process(inputs[0 * 16 + i]);
            let triggered = self.detectors[i].process(inputs[1 * 16 + i]) || gate_rising;

            // アナログ的不完全さ
            let p_offset = ctx.imperfection.personality[i] * 0.05;
            let a = (a_knob + p_offset).max(0.001);
            let d = (d_knob + p_offset).max(0.001);
            let s = (s_knob + p_offset * 0.1).clamp(0.0, 1.0);
            let r = (r_knob + p_offset).max(0.001);

            if triggered {
                self.states[i] = EnvelopeState::Attack;
            }

            match self.states[i] {
                EnvelopeState::Idle => {
                    self.values[i] = 0.0;
                }
                EnvelopeState::Attack => {
                    // Attack is often perceived better as a slightly faster-than-linear or exponential curve
                    // Here we use a standard RC-style curve towards 1.2 (to ensure we hit 1.0 fast and snap)
                    let target = 1.2;
                    let alpha = libm::expf(-dt / a);
                    self.values[i] = target + (self.values[i] - target) * alpha;
                    
                    if self.values[i] >= 1.0 {
                        self.values[i] = 1.0;
                        self.states[i] = EnvelopeState::Decay;
                    }
                }
                EnvelopeState::Decay => {
                    let target = s;
                    let alpha = libm::expf(-dt / d);
                    self.values[i] = target + (self.values[i] - target) * alpha;
                    
                    if (self.values[i] - s).abs() < 0.001 {
                        self.values[i] = s;
                        self.states[i] = EnvelopeState::Sustain;
                    }
                }
                EnvelopeState::Sustain => {
                    self.values[i] = s;
                    if !gate_active {
                        self.states[i] = EnvelopeState::Release;
                    }
                }
                EnvelopeState::Release => {
                    let target = 0.0;
                    let alpha = libm::expf(-dt / r);
                    self.values[i] = target + (self.values[i] - target) * alpha;

                    if self.values[i] <= 0.001 {
                        self.values[i] = 0.0;
                        self.states[i] = EnvelopeState::Idle;
                    }
                }
            }

            outputs[0 * 16 + i] = self.values[i] * 5.0;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_envelope",
        name: "ADSR",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "ENV", "ADSR"],
        params: &[
            ParamDescriptor {
                name: "ATTACK",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.001,
                max: 2.0,
                default: 0.1,
                position: [0.5, 0.2],
                unit: "s",
            },
            ParamDescriptor {
                name: "DECAY",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.001,
                max: 2.0,
                default: 0.2,
                position: [0.5, 0.4],
                unit: "s",
            },
            ParamDescriptor {
                name: "SUSTAIN",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.5,
                position: [0.5, 0.6],
                unit: "",
            },
            ParamDescriptor {
                name: "RELEASE",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.001,
                max: 5.0,
                default: 0.5,
                position: [0.5, 0.8],
                unit: "s",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "GATE",
                direction: PortDirection::Input,
                signal_type: SignalType::Gate,
                max_channels: 1,
                position: [0.2, 0.1],
            },
            PortDescriptor {
                name: "TRIG",
                direction: PortDirection::Input,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.8, 0.1],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.5, 0.95],
            },
        ],
        factory: |sr| Box::new(EnvelopeModule::new(sr)),
    }
}
