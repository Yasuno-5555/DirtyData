//! Macro Controller Module — The Command Center
//!
//! # 憲法遵守
//! - 8つの独立したCV出力を一括管理。
//! - パッチ全体の「シーン」や「抽象的な音色変化」を一つのノブに集約。
//! - 決定論的再生において、演奏者の「マクロな意図」を確実に再現。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SignalType,
};

pub struct MacroModule {}

impl MacroModule {
    pub fn new(_sr: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for MacroModule {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            for i in 0..8 {
                outputs[i * 16 + v] = params[i];
            }
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_util_macro",
        name: "Macro 8",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[
            ParamDescriptor {
                name: "M1",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 0.0,
                position: [0.2, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "M2",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 0.0,
                position: [0.4, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "M3",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 0.0,
                position: [0.6, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "M4",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 0.0,
                position: [0.8, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "M5",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 0.0,
                position: [0.2, 0.5],
                unit: "V",
            },
            ParamDescriptor {
                name: "M6",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 0.0,
                position: [0.4, 0.5],
                unit: "V",
            },
            ParamDescriptor {
                name: "M7",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 0.0,
                position: [0.6, 0.5],
                unit: "V",
            },
            ParamDescriptor {
                name: "M8",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 0.0,
                position: [0.8, 0.5],
                unit: "V",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "OUT 1",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.2, 0.8],
            },
            PortDescriptor {
                name: "OUT 2",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.4, 0.8],
            },
            PortDescriptor {
                name: "OUT 3",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.6, 0.8],
            },
            PortDescriptor {
                name: "OUT 4",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.8, 0.8],
            },
            PortDescriptor {
                name: "OUT 5",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.2, 0.9],
            },
            PortDescriptor {
                name: "OUT 6",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.4, 0.9],
            },
            PortDescriptor {
                name: "OUT 7",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.6, 0.9],
            },
            PortDescriptor {
                name: "OUT 8",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.8, 0.9],
            },
        ],
        factory: |sr| Box::new(MacroModule::new(sr)),
    }
}
