//! Compressor Module — Dynamics Control & Sidechain
//!
//! # 憲法遵守
//! - ピーク検出によるゲインリダクション。
//! - サイドチェイン入力をサポートし、ダッキングが可能。
//! - エンベロープ・フォロワーの決定論的一貫性。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SignalType,
};

pub struct CompressorModule {
    envelopes: [f32; 16],
    sample_rate: f32,
}

impl CompressorModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            envelopes: [0.0; 16],
            sample_rate,
        }
    }
}

impl RackDspNode for CompressorModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let thresh = params[0]; // 0V .. 5V
        let ratio = params[1]; // 1.0 .. 20.0
        let attack = params[2]; // ms
        let release = params[3]; // ms

        // Coefficients
        let att_coeff = libm::expf(-1.0 / (attack * 0.001 * self.sample_rate));
        let rel_coeff = libm::expf(-1.0 / (release * 0.001 * self.sample_rate));

        for v in 0..16 {
            let input = inputs[0 * 16 + v];
            let sidechain = if inputs[1 * 16 + v].abs() > 0.001 {
                inputs[1 * 16 + v]
            } else {
                input
            };

            // Envelope follower (peak)
            let abs_sc = libm::fabsf(sidechain);
            if abs_sc > self.envelopes[v] {
                self.envelopes[v] = att_coeff * self.envelopes[v] + (1.0 - att_coeff) * abs_sc;
            } else {
                self.envelopes[v] = rel_coeff * self.envelopes[v] + (1.0 - rel_coeff) * abs_sc;
            }

            // Gain reduction
            let mut gain = 1.0;
            if self.envelopes[v] > thresh && thresh > 0.0 {
                let over = self.envelopes[v] / thresh;
                let target_gain = libm::powf(over, (1.0 / ratio) - 1.0);
                gain = target_gain;
            }

            outputs[0 * 16 + v] = input * gain;
            outputs[1 * 16 + v] = (1.0 - gain) * 5.0; // GR Meter output
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_dyn_comp",
        name: "Compressor",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[
            ParamDescriptor {
                name: "THRESHOLD",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 2.0,
                position: [0.5, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "RATIO",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 1.0,
                max: 20.0,
                default: 4.0,
                position: [0.5, 0.4],
                unit: ":1",
            },
            ParamDescriptor {
                name: "ATTACK",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 1.0,
                max: 100.0,
                default: 10.0,
                position: [0.5, 0.6],
                unit: "ms",
            },
            ParamDescriptor {
                name: "RELEASE",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 10.0,
                max: 1000.0,
                default: 100.0,
                position: [0.5, 0.8],
                unit: "ms",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.2, 0.15],
            },
            PortDescriptor {
                name: "SIDECHAIN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.2, 0.35],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.15],
            },
            PortDescriptor {
                name: "GR",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.8, 0.35],
            },
        ],
        factory: |sr| Box::new(CompressorModule::new(sr)),
    }
}
