//! Crossfader Module — Smooth Morphing
//!
//! # 憲法遵守
//! - 2つの入力を CV で線形補間（モーフィング）。
//! - 音色やエフェクトの「混ざり具合」を動的に制御。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SeedScope, SignalType,
};

pub struct CrossfaderModule {}

impl CrossfaderModule {
    pub fn new(_sr: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for CrossfaderModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let mix_param = params[0];

        for v in 0..16 {
            let a = inputs[0 * 16 + v];
            let b = inputs[1 * 16 + v];
            let cv = inputs[2 * 16 + v];

            // Combine param and CV (0.0 .. 1.0)
            let mix = (mix_param + cv * 0.2).clamp(0.0, 1.0);

            outputs[0 * 16 + v] = a * (1.0 - mix) + b * mix;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_mixer_xfade",
        name: "Crossfader",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "MIX", "UTL"],
        params: &[ParamDescriptor {
            name: "MIX",
            kind: ParamKind::Knob,
            response: ParamResponse::Immediate,
            min: 0.0,
            max: 1.0,
            default: 0.5,
            position: [0.5, 0.4],
            unit: "",
        }],
        ports: &[
            PortDescriptor {
                name: "IN A",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.2, 0.2],
            },
            PortDescriptor {
                name: "IN B",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.2],
            },
            PortDescriptor {
                name: "MIX CV",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.5, 0.6],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.5, 0.9],
            },
        ],
        factory: |sr| Box::new(CrossfaderModule::new(sr)),
    }
}
