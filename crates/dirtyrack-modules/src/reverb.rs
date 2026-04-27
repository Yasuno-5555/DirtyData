//! Reverb Module — Deterministic FDN Space
//!
//! # 憲法遵守
//! - 4x4 Feedback Delay Network (FDN) による空間シミュレート。
//! - ハウスホルダー行列によるユニタリなエネルギー保存フィードバック。
//! - 決定論的な固定ディレイ長を使用。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SeedScope, SignalType,
};

pub struct ReverbModule {
    buffers: [Vec<f32>; 4],
    write_pos: [usize; 4],
}

impl ReverbModule {
    pub fn new(sr: f32) -> Self {
        // Prime numbers or distinct lengths for better density
        let lengths = [
            (sr * 0.029).as_usize(),
            (sr * 0.037).as_usize(),
            (sr * 0.043).as_usize(),
            (sr * 0.047).as_usize(),
        ];
        Self {
            buffers: [
                vec![0.0; lengths[0]],
                vec![0.0; lengths[1]],
                vec![0.0; lengths[2]],
                vec![0.0; lengths[3]],
            ],
            write_pos: [0; 4],
        }
    }
}

trait AsUsize {
    fn as_usize(self) -> usize;
}
impl AsUsize for f32 {
    fn as_usize(self) -> usize {
        self as usize
    }
}

impl RackDspNode for ReverbModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let input = inputs[0 * 16]; // Port 0 (IN)
        let size = params[0]; // Decay time
        let dry_wet = params[1];

        // 1. Read from delays
        let mut d = [0.0; 4];
        for i in 0..4 {
            d[i] = self.buffers[i][self.write_pos[i]];
        }

        // 2. Householder Matrix Mixing (Unitary)
        let sum = d[0] + d[1] + d[2] + d[3];
        let mut mixed = [0.0; 4];
        for i in 0..4 {
            mixed[i] = d[i] - 0.5 * sum;
        }

        // 3. Feedback & Write
        for i in 0..4 {
            let val = input * 0.25 + mixed[i] * size;
            self.buffers[i][self.write_pos[i]] = val;
            self.write_pos[i] = (self.write_pos[i] + 1) % self.buffers[i].len();
        }

        // Output (Mono sum of FDN)
        let reverb_out = (d[0] + d[1] + d[2] + d[3]) * 0.5;
        let final_out = input * (1.0 - dry_wet) + reverb_out * dry_wet;

        for v in 0..16 {
            outputs[0 * 16 + v] = final_out;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_eff_reverb",
        name: "Reverb",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "FX", "REVERB"],
        params: &[
            ParamDescriptor {
                name: "SIZE",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 0.98,
                default: 0.5,
                position: [0.5, 0.3],
                unit: "",
            },
            ParamDescriptor {
                name: "MIX",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.3,
                position: [0.5, 0.6],
                unit: "",
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
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.9],
            },
        ],
        factory: |sr| Box::new(ReverbModule::new(sr)),
    }
}
