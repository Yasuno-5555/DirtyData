//! LFO Module — Low Frequency Oscillator
//!
//! # Parameters
//! - RATE: 速度
//! - SHAPE: 波形選択
//! - AMT: 出力量

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};

pub struct LfoModule {
    phase: f32,
    sample_rate: f32,
}

impl LfoModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate,
        }
    }
}

impl RackDspNode for LfoModule {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let rate = params[0];
        let _shape = params[1];
        let amt = params[2];

        let freq = 0.01 * 2.0_f32.powf(rate * 10.0);
        self.phase = (self.phase + freq / self.sample_rate).fract();

        let sin_val = (self.phase * 2.0 * std::f32::consts::PI).sin() * amt * 5.0;
        let sq_val = (if self.phase < 0.5 { 1.0 } else { -1.0 }) * amt * 5.0;

        for v in 0..16 {
            outputs[0 * 16 + v] = sin_val; // TRI/SINE Port 0
            outputs[1 * 16 + v] = sq_val; // SQUARE Port 1
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_lfo",
        name: "LFO",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "LFO", "MOD"],
        params: &[
            ParamDescriptor {
                name: "RATE",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.0,
                max: 1.0,
                default: 0.5,
                position: [0.5, 0.2],
                unit: "Hz",
            },
            ParamDescriptor {
                name: "SHAPE",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.5, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "AMT",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 20.0 },
                min: 0.0,
                max: 1.0,
                default: 1.0,
                position: [0.5, 0.6],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "TRI",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.3, 0.9],
            },
            PortDescriptor {
                name: "SQUARE",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.7, 0.9],
            },
        ],
        factory: |sr| Box::new(LfoModule::new(sr)),
    }
}
