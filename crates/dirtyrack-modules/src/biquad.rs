//! Biquad Filter (VCF-2) — Precision State Variable Filter
//!
//! # 憲法遵守
//! - `libm` による数学的同一性。
//! - LP/HP/BP/Notch のマルチモード。
//! - Moog よりもレゾナンスの効きがシャープで数学的に正確。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SeedScope, SignalType,
};

pub struct BiquadModule {
    sample_rate: f32,
    s1: [f32; 16],
    s2: [f32; 16],
}

impl BiquadModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            s1: [0.0; 16],
            s2: [0.0; 16],
        }
    }
}

impl RackDspNode for BiquadModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let cutoff_knob = params[0].max(0.01).min(10.0);
        let res_knob = params[1].max(0.0).min(0.99);
        let mode = params[2] as usize;

        // Common coefficients
        let freq = 20.0 * libm::powf(1000.0, cutoff_knob / 10.0);
        let g = libm::tanf(std::f32::consts::PI * freq / self.sample_rate);
        let k = 2.0 * (1.0 - res_knob);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        for v in 0..16 {
            let input = inputs[0 * 16 + v];
            
            let v3 = input - self.s2[v];
            let v1 = a1 * self.s1[v] + a2 * v3;
            let v2 = self.s2[v] + a2 * self.s1[v] + a3 * v3;
            
            self.s1[v] = 2.0 * v1 - self.s1[v];
            self.s2[v] = 2.0 * v2 - self.s2[v];

            let out = match mode {
                0 => v2,               // LP
                1 => input - k*v1 - v2, // HP
                2 => v1,               // BP
                _ => input - k*v1,      // Notch
            };

            outputs[0 * 16 + v] = out;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_vcf_biquad",
        name: "VCF-2",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "FLT", "VCF"],
        params: &[
            ParamDescriptor {
                name: "CUTOFF",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 10.0,
                default: 5.0,
                position: [0.5, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "RES",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.5,
                position: [0.5, 0.4],
                unit: "Q",
            },
            ParamDescriptor {
                name: "MODE",
                kind: ParamKind::Switch { positions: 4 },
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 3.0,
                default: 0.0,
                position: [0.5, 0.7],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.2, 0.9],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.8, 0.9],
            },
        ],
        factory: |sr| Box::new(BiquadModule::new(sr)),
    }
}
