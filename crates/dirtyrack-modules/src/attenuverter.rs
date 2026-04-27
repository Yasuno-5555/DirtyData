//! Attenuverter Module — The Soul of CV
//!
//! # 憲法遵守
//! - `out = (in * gain) + offset`
//! - gain は -1.0 .. +1.0 (反転対応)
//! - 信号を「反転」させてモジュレーションを逆相にするための必須ツール。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SeedScope, SignalType,
};

pub struct AttenuverterModule {}

impl AttenuverterModule {
    pub fn new(_sr: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for AttenuverterModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            // Dual channels
            for i in 0..2 {
                let input = inputs[i * 16 + v];
                let gain = params[i]; // -1.0 .. 1.0
                let offset = params[i + 2]; // -5.0 .. 5.0
                outputs[i * 16 + v] = input * gain + offset;
            }
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_attenuverter",
        name: "Attenuverter",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "UTL", "MIX"],
        params: &[
            ParamDescriptor {
                name: "GAIN 1",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: -1.0,
                max: 1.0,
                default: 1.0,
                position: [0.5, 0.2],
                unit: "x",
            },
            ParamDescriptor {
                name: "GAIN 2",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: -1.0,
                max: 1.0,
                default: 1.0,
                position: [0.5, 0.5],
                unit: "x",
            },
            ParamDescriptor {
                name: "OFFSET 1",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: -5.0,
                max: 5.0,
                default: 0.0,
                position: [0.8, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "OFFSET 2",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: -5.0,
                max: 5.0,
                default: 0.0,
                position: [0.8, 0.5],
                unit: "V",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN 1",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.2, 0.2],
            },
            PortDescriptor {
                name: "IN 2",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.2, 0.5],
            },
            PortDescriptor {
                name: "OUT 1",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.5, 0.8],
            },
            PortDescriptor {
                name: "OUT 2",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.8, 0.8],
            },
        ],
        factory: |sr| Box::new(AttenuverterModule::new(sr)),
    }
}
