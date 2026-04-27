//! Mod Matrix Module — Complex CV Routing Layer
//!
//! # 憲法遵守
//! - 4つの入力（Sources）を4つの出力（Targets）へ。
//! - 16個のスライダーで各パスの Depth (量) と Polarity (極性) を制御。
//! - パッチが巨大化しても、配線を整理したまま複雑な変調ネットワークを構築可能。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SignalType,
};

pub struct ModMatrixModule {}

impl ModMatrixModule {
    pub fn new(_sr: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for ModMatrixModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        // Clear outputs
        for o in outputs.iter_mut() {
            *o = 0.0;
        }

        for v in 0..16 {
            for src in 0..4 {
                let src_val = inputs[src * 16 + v];
                for dst in 0..4 {
                    let depth = params[src * 4 + dst]; // -1.0 .. 1.0
                    outputs[dst * 16 + v] += src_val * depth;
                }
            }
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    let mut params = Vec::new();
    for src in 1..=4 {
        for dst in 1..=4 {
            params.push(ParamDescriptor {
                name: Box::leak(format!("S{}->T{}", src, dst).into_boxed_str()),
                kind: ParamKind::Slider,
                response: ParamResponse::Immediate,
                min: -1.0,
                max: 1.0,
                default: 0.0,
                position: [0.15 + (dst as f32) * 0.15, 0.1 + (src as f32) * 0.15],
                unit: "",
            });
        }
    }

    BuiltinModuleDescriptor {
        id: "dirty_util_modmatrix",
        name: "Mod Matrix",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 12,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: Box::leak(params.into_boxed_slice()),
        ports: &[
            PortDescriptor {
                name: "SRC 1",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.05, 0.25],
            },
            PortDescriptor {
                name: "SRC 2",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.05, 0.40],
            },
            PortDescriptor {
                name: "SRC 3",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.05, 0.55],
            },
            PortDescriptor {
                name: "SRC 4",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.05, 0.70],
            },
            PortDescriptor {
                name: "TGT 1",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.25, 0.9],
            },
            PortDescriptor {
                name: "TGT 2",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.45, 0.9],
            },
            PortDescriptor {
                name: "TGT 3",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.65, 0.9],
            },
            PortDescriptor {
                name: "TGT 4",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.85, 0.9],
            },
        ],
        factory: |sr| Box::new(ModMatrixModule::new(sr)),
    }
}
