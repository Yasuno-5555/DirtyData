//! Mixer Module — 4-channel Mixer
//!
//! # Parameters
//! - VOL 1..4: 各チャンネルの音量
//! - MASTER: 最終出力音量
//!
//! # Inputs
//! - IN 1..4: オーディオ入力
//!
//! # Outputs
//! - OUT: ミックス出力

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SeedScope, SignalType,
};

pub struct MixerModule {}

impl MixerModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for MixerModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            let mut mixed = 0.0;
            for i in 0..4 {
                // inputs[i * 16 + v] refers to port i, voice v
                mixed += inputs[i * 16 + v] * params[i];
            }
            outputs[v] = mixed * params[4]; // MASTER
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_mixer",
        name: "MIXER",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "MIX", "UTL"],
        params: &[
            ParamDescriptor {
                name: "VOL 1",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 20.0 },
                min: 0.0,
                max: 1.0,
                default: 0.8,
                position: [0.3, 0.2],
                unit: "",
            },
            ParamDescriptor {
                name: "VOL 2",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 20.0 },
                min: 0.0,
                max: 1.0,
                default: 0.8,
                position: [0.7, 0.2],
                unit: "",
            },
            ParamDescriptor {
                name: "VOL 3",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 20.0 },
                min: 0.0,
                max: 1.0,
                default: 0.8,
                position: [0.3, 0.5],
                unit: "",
            },
            ParamDescriptor {
                name: "VOL 4",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 20.0 },
                min: 0.0,
                max: 1.0,
                default: 0.8,
                position: [0.7, 0.5],
                unit: "",
            },
            ParamDescriptor {
                name: "MASTER",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 20.0 },
                min: 0.0,
                max: 1.0,
                default: 0.7,
                position: [0.5, 0.8],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN 1",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.1, 0.1],
            },
            PortDescriptor {
                name: "IN 2",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.9, 0.1],
            },
            PortDescriptor {
                name: "IN 3",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.1, 0.4],
            },
            PortDescriptor {
                name: "IN 4",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.9, 0.4],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.5, 0.95],
            },
        ],
        factory: |sr| Box::new(MixerModule::new(sr)),
    }
}
