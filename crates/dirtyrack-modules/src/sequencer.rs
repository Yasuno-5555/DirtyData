//! Sequencer Module — 8-step Gate Sequencer
//!
//! # Parameters
//! - STEPS 1..8: 各ステップのON/OFF
//!
//! # Inputs
//! - CLOCK: クロック入力
//! - RESET: リセット入力
//!
//! # Outputs
//! - GATE: ゲート出力

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SeedScope, SignalType, TriggerDetector,
};

pub struct SequencerModule {
    clock_detector: TriggerDetector,
    reset_detector: TriggerDetector,
    current_step: usize,
}

impl SequencerModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self {
            clock_detector: TriggerDetector::new(),
            reset_detector: TriggerDetector::new(),
            current_step: 0,
        }
    }
}

impl RackDspNode for SequencerModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let clock_in = inputs[0 * 16]; // Port 0 (CLOCK)
        let reset_in = inputs[1 * 16]; // Port 1 (RESET)

        if self.reset_detector.process(reset_in) {
            self.current_step = 0;
        }

        if self.clock_detector.process(clock_in) {
            self.current_step = (self.current_step + 1) % 8;
        }

        let step_val = params[self.current_step];
        let gate = if step_val > 0.5 { 5.0 } else { 0.0 };
        for v in 0..16 {
            outputs[v] = gate;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_sequencer",
        name: "SEQ-8",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 12,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "SEQ", "UTL"],
        params: &[
            ParamDescriptor {
                name: "S1",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.1, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "S2",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.2, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "S3",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.3, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "S4",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.4, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "S5",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.5, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "S6",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.6, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "S7",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.7, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "S8",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.8, 0.4],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "CLOCK",
                direction: PortDirection::Input,
                signal_type: SignalType::Clock,
                max_channels: 1,
                position: [0.1, 0.8],
            },
            PortDescriptor {
                name: "RESET",
                direction: PortDirection::Input,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.2, 0.8],
            },
            PortDescriptor {
                name: "GATE",
                direction: PortDirection::Output,
                signal_type: SignalType::Gate,
                max_channels: 1,
                position: [0.9, 0.8],
            },
        ],
        factory: |sr| Box::new(SequencerModule::new(sr)),
    }
}
