//! Clock Module — Master Clock
//!
//! # Parameters
//! - BPM: テンポ
//!
//! # Outputs
//! - CLK: クロック出力
//! - RESET: リセット出力 (手動)

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SeedScope, SignalType,
};

pub struct ClockModule {
    phase: f64,
}

impl ClockModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self { phase: 0.0 }
    }
}

impl RackDspNode for ClockModule {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        ctx: &RackProcessContext,
    ) {
        let bpm = params[0];
        let freq = bpm / 60.0;

        let prev_phase = self.phase;
        self.phase = (self.phase + (freq / ctx.sample_rate) as f64).fract();

        // 5ms pulse
        let clk_val = if self.phase < (freq as f64 * 0.005) {
            5.0
        } else {
            0.0
        };
        let reset_val = if self.phase < prev_phase { 5.0 } else { 0.0 };

        for v in 0..16 {
            outputs[0 * 16 + v] = clk_val;
            outputs[1 * 16 + v] = reset_val;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_clock",
        name: "CLOCK",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "CLK", "UTL"],
        params: &[ParamDescriptor {
            name: "BPM",
            kind: ParamKind::Knob,
            response: ParamResponse::Smoothed { ms: 100.0 },
            min: 20.0,
            max: 300.0,
            default: 120.0,
            position: [0.5, 0.3],
            unit: "BPM",
        }],
        ports: &[
            PortDescriptor {
                name: "CLK",
                direction: PortDirection::Output,
                signal_type: SignalType::Clock,
                max_channels: 1,
                position: [0.5, 0.7],
            },
            PortDescriptor {
                name: "RESET",
                direction: PortDirection::Output,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.5, 0.9],
            },
        ],
        factory: |sr| Box::new(ClockModule::new(sr)),
    }
}
