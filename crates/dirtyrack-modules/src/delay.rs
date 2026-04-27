//! Delay Module — Deterministic Echo Engine
//!
//! # 憲法遵守
//! - `ConsistencyBudget::BitIdentical` を追求。
//! - 内部バッファは固定長で、補間には決定論的な線形補間を使用。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SeedScope, SignalType,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct DelayStateData {
    buffer: Vec<f32>,
    write_pos: usize,
}

pub struct DelayModule {
    buffer: Vec<f32>,
    write_pos: usize,
    sample_rate: f32,
}

impl DelayModule {
    pub fn new(sample_rate: f32) -> Self {
        // Max 2 seconds delay
        let size = (sample_rate * 2.0) as usize;
        Self {
            buffer: vec![0.0; size],
            write_pos: 0,
            sample_rate,
        }
    }
}

impl RackDspNode for DelayModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let input = inputs[0 * 16]; // Port 0 (IN)
        let time_cv = inputs[1 * 16]; // Port 1 (TIME_CV)
        let time_knob = params[0]; // 0.0 .. 1.0 (0 .. 2s)
        let feedback = params[1].clamp(0.0, 0.99);
        let dry_wet = params[2].clamp(0.0, 1.0);

        let total_time = (time_knob + time_cv * 0.1).clamp(0.0, 1.0);
        let delay_samples = (total_time * (self.buffer.len() as f32 - 1.0)).max(1.0);

        // Read position with linear interpolation for bit-identical determinism
        let read_pos = (self.write_pos as f32 - delay_samples + self.buffer.len() as f32)
            % self.buffer.len() as f32;
        let idx0 = read_pos as usize;
        let idx1 = (idx0 + 1) % self.buffer.len();
        let frac = read_pos - idx0 as f32;

        let delayed = self.buffer[idx0] * (1.0 - frac) + self.buffer[idx1] * frac;

        // Write back with feedback
        self.buffer[self.write_pos] = input + delayed * feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();

        for v in 0..16 {
            outputs[0 * 16 + v] = input * (1.0 - dry_wet) + delayed * dry_wet;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_delay",
        name: "Delay",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "FX", "DELAY"],
        params: &[
            ParamDescriptor {
                name: "TIME",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.0,
                max: 1.0,
                default: 0.3,
                position: [0.5, 0.2],
                unit: "s",
            },
            ParamDescriptor {
                name: "FEEDBACK",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.5,
                position: [0.5, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "MIX",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.5,
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
                position: [0.2, 0.85],
            },
            PortDescriptor {
                name: "TIME_CV",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.5, 0.85],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.85],
            },
        ],
        factory: |sr| Box::new(DelayModule::new(sr)),
    }
}
