//! Clock Tree Module — Rhythmic Ecology
//!
//! # 憲法遵守
//! - 単一のクロック入力から、分周、確率的ゲート、フェーズオフセットを生成。
//! - 決定論的なランダム（ChaChaベース）による「有機的なリズム」の創出。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SeedScope, SignalType, TriggerDetector,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct ClockTreeModule {
    trigger: TriggerDetector,
    counts: [u32; 4],
    rng: ChaCha8Rng,
}

impl ClockTreeModule {
    pub fn new(_sr: f32) -> Self {
        Self {
            trigger: TriggerDetector::new(),
            counts: [0; 4],
            rng: ChaCha8Rng::seed_from_u64(0x42),
        }
    }
}

impl RackDspNode for ClockTreeModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let clock = inputs[0];
        let is_tick = self.trigger.process(clock);

        for i in 0..4 {
            let div = (params[i * 2] as u32).max(1);
            let prob = params[i * 2 + 1]; // 0.0 .. 1.0

            if is_tick {
                self.counts[i] += 1;
                if self.counts[i] >= div {
                    self.counts[i] = 0;
                    // Probability check
                    if self.rng.gen_bool(prob as f64) {
                        outputs[i] = 5.0;
                    } else {
                        outputs[i] = 0.0;
                    }
                } else {
                    outputs[i] = 0.0;
                }
            } else {
                // Keep gate high for a short burst (simplified)
                if outputs[i] > 0.0 {
                    outputs[i] *= 0.9;
                    if outputs[i] < 0.1 {
                        outputs[i] = 0.0;
                    }
                }
            }
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_util_clocktree",
        name: "Clock Tree",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[
            ParamDescriptor {
                name: "DIV 1",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 1.0,
                max: 16.0,
                default: 1.0,
                position: [0.3, 0.2],
                unit: "",
            },
            ParamDescriptor {
                name: "PROB 1",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 1.0,
                position: [0.7, 0.2],
                unit: "",
            },
            ParamDescriptor {
                name: "DIV 2",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 1.0,
                max: 16.0,
                default: 2.0,
                position: [0.3, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "PROB 2",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 1.0,
                position: [0.7, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "DIV 3",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 1.0,
                max: 16.0,
                default: 4.0,
                position: [0.3, 0.6],
                unit: "",
            },
            ParamDescriptor {
                name: "PROB 3",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 1.0,
                position: [0.7, 0.6],
                unit: "",
            },
            ParamDescriptor {
                name: "DIV 4",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 1.0,
                max: 16.0,
                default: 8.0,
                position: [0.3, 0.8],
                unit: "",
            },
            ParamDescriptor {
                name: "PROB 4",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 1.0,
                position: [0.7, 0.8],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "CLK IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Clock,
                max_channels: 1,
                position: [0.1, 0.1],
            },
            PortDescriptor {
                name: "OUT 1",
                direction: PortDirection::Output,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.9, 0.2],
            },
            PortDescriptor {
                name: "OUT 2",
                direction: PortDirection::Output,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.9, 0.4],
            },
            PortDescriptor {
                name: "OUT 3",
                direction: PortDirection::Output,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.9, 0.6],
            },
            PortDescriptor {
                name: "OUT 4",
                direction: PortDirection::Output,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.9, 0.8],
            },
        ],
        factory: |sr| Box::new(ClockTreeModule::new(sr)),
    }
}
