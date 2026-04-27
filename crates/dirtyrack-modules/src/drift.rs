//! Drift Module — Controlled Instability
//!
//! # 憲法遵守
//! - 決定論的なドリフト（低周波のランダム変動）を生成。
//! - 各ボイスやモジュールに「アナログ的な不確実性」を意図的に導入。
//! - 共有シードによる再現可能な揺らぎ。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SignalType,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct DriftModule {
    rng: ChaCha8Rng,
    current_val: f32,
    target_val: f32,
}

impl DriftModule {
    pub fn new(_sr: f32) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(0x99),
            current_val: 0.0,
            target_val: 0.0,
        }
    }
}

impl RackDspNode for DriftModule {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let rate = params[0]; // 0.0001 .. 0.01
        let depth = params[1]; // 0.0 .. 5.0

        // Linear interpolation towards a random target
        if (self.current_val - self.target_val).abs() < 0.01 {
            self.target_val = self.rng.gen_range(-1.0..1.0);
        }

        self.current_val += (self.target_val - self.current_val) * rate;

        outputs[0] = self.current_val * depth;
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_util_drift",
        name: "Drift",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[
            ParamDescriptor {
                name: "RATE",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0001,
                max: 0.05,
                default: 0.001,
                position: [0.5, 0.3],
                unit: "",
            },
            ParamDescriptor {
                name: "DEPTH",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 1.0,
                position: [0.5, 0.6],
                unit: "V",
            },
        ],
        ports: &[PortDescriptor {
            name: "OUT",
            direction: PortDirection::Output,
            signal_type: SignalType::BiCV,
            max_channels: 1,
            position: [0.5, 0.9],
        }],
        factory: |sr| Box::new(DriftModule::new(sr)),
    }
}
