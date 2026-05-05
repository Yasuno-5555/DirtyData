//! Cellular Automata Module — 1D Wolfram Automaton at Audio Rate
//!
//! # 決定論的カオス
//! - Rule(0-255) に従って 1D セル・オートマトンをオーディオレートで実行。
//! - セルの状態を電圧として出力。SeedとRuleが同じなら完全に同じ波形を生成。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType, SmoothedParam, TriggerDetector,
};
use serde::{Deserialize, Serialize};

const FIELD_SIZE: usize = 32;

#[derive(Serialize, Deserialize)]

pub struct AutomataModule {
    cells: [[bool; FIELD_SIZE]; 16],
    next_cells: [[bool; FIELD_SIZE]; 16],
    rule_smooth: SmoothedParam,
    rate_smooth: SmoothedParam,
    clock_detectors: [TriggerDetector; 16],
    phase: [f32; 16],
    sample_rate: f32,
}

impl AutomataModule {
    pub fn new(sample_rate: f32) -> Self {
        let mut cells = [[false; FIELD_SIZE]; 16];
        for v in 0..16 {
            cells[v][FIELD_SIZE / 2] = true; // Center seed
        }

        Self {
            cells,
            next_cells: [[false; FIELD_SIZE]; 16],
            rule_smooth: SmoothedParam::new(30.0, sample_rate, 50.0),
            rate_smooth: SmoothedParam::new(0.5, sample_rate, 10.0), // 0.0 to 1.0 (normalized)
            clock_detectors: [TriggerDetector::new(); 16],
            phase: [0.0; 16],
            sample_rate,
        }
    }

    fn step_automaton(&mut self, v: usize, rule: u8) {
        for i in 0..FIELD_SIZE {
            let left = if i == 0 {
                self.cells[v][FIELD_SIZE - 1]
            } else {
                self.cells[v][i - 1]
            };
            let center = self.cells[v][i];
            let right = if i == FIELD_SIZE - 1 {
                self.cells[v][0]
            } else {
                self.cells[v][i + 1]
            };

            let pattern = ((left as u8) << 2) | ((center as u8) << 1) | (right as u8);
            self.next_cells[v][i] = (rule & (1 << pattern)) != 0;
        }
        self.cells[v].copy_from_slice(&self.next_cells[v]);
    }

    fn calculate_voltage(&self, v: usize) -> f32 {
        let mut sum = 0;
        for &cell in &self.cells[v] {
            if cell {
                sum += 1;
            }
        }
        (sum as f32 / FIELD_SIZE as f32) * 10.0 - 5.0
    }
}

impl RackDspNode for AutomataModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let rule_knob = params[0];
        let rate_knob = params[1]; // 0.0 = External Clock only, 1.0 = Audio rate

        self.rule_smooth.set(rule_knob);
        self.rate_smooth.set(rate_knob);

        let jitter = _ctx.imperfection.drift[0];
        let rule_val = self.rule_smooth.next(jitter).clamp(0.0, 255.0) as u8;
        let rate_val = self.rate_smooth.next(jitter).clamp(0.0, 1.0);

        for i in 0..16 {
            let clock_in = inputs[0 * 16 + i];
            let rule_cv = inputs[1 * 16 + i];

            let current_rule = (rule_val as f32 + rule_cv * 25.5).clamp(0.0, 255.0) as u8;

            let mut trigger_step = false;

            // 1. External Clock
            if self.clock_detectors[i].process(clock_in) {
                trigger_step = true;
            }

            // 2. Internal Oscillator Rate
            if rate_val > 0.01 {
                let freq = 10.0f32.powf(rate_val * 4.0);
                let dt = freq / self.sample_rate;
                self.phase[i] += dt;
                if self.phase[i] >= 1.0 {
                    self.phase[i] -= 1.0;
                    trigger_step = true;
                }
            }

            if trigger_step {
                self.step_automaton(i, current_rule);
            }

            outputs[0 * 16 + i] = self.calculate_voltage(i);
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_automata",
        name: "AUTOMATA",
        version: "1.0.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals {
            background_color: [20, 20, 20],
            text_color: [0, 255, 0], // Matrix green
            accent_color: [50, 255, 50],
            panel_texture: crate::signal::PanelTexture::MatteBlack,
        },
        tags: &["Builtin", "GEN", "OSC", "CHAOS"],
        params: &[
            ParamDescriptor {
                name: "RULE",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.0,
                max: 255.0,
                default: 30.0, // Rule 30 is famously chaotic
                position: [0.5, 0.3],
                unit: "",
            },
            ParamDescriptor {
                name: "RATE",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.5,
                position: [0.5, 0.6],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "CLOCK",
                direction: PortDirection::Input,
                signal_type: SignalType::Trigger,
                max_channels: 16,
                position: [0.2, 0.85],
            },
            PortDescriptor {
                name: "RULE_CV",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 16,
                position: [0.5, 0.85],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio, // Can be used as audio or CV
                max_channels: 16,
                position: [0.8, 0.85],
            },
        ],
        factory: |sr| Box::new(AutomataModule::new(sr)),
    }
}
