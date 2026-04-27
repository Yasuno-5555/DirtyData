//! Wavefolder Module — West Coast Harmonic Shaper
//!
//! # 憲法遵守
//! - 再帰的なサイン波フォールディングまたは反転フォールディング。
//! - 振幅が閾値を超えた場合に「折り返す」ことで複雑な倍音を生成。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SeedScope, SignalType,
};

pub struct WavefolderModule {}

impl WavefolderModule {
    pub fn new(_sr: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for WavefolderModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let input = inputs[0] + inputs[1]; // IN + CV
        let fold_gain = params[0]; // 1.0 .. 10.0
        let bias = params[1]; // -5.0 .. 5.0

        let mut x = (input + bias) * fold_gain;

        // Recursive folding (sin-based for smooth transitions)
        // 4 stages of folding
        for _ in 0..4 {
            x = 5.0 * libm::sinf(x * (std::f32::consts::PI / 5.0));
        }

        outputs[0] = x;
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_shaper_fold",
        name: "Wavefolder",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[
            ParamDescriptor {
                name: "FOLD",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 1.0,
                max: 10.0,
                default: 1.0,
                position: [0.5, 0.3],
                unit: "x",
            },
            ParamDescriptor {
                name: "BIAS",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: -5.0,
                max: 5.0,
                default: 0.0,
                position: [0.5, 0.6],
                unit: "V",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.2, 0.9],
            },
            PortDescriptor {
                name: "FOLD CV",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.5, 0.9],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.9],
            },
        ],
        factory: |sr| Box::new(WavefolderModule::new(sr)),
    }
}
