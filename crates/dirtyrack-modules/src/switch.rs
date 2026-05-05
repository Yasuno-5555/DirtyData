//! Sequential Switch — Musical Structure Builder
//!
//! # 憲法遵守
//! - トリガー入力（Clock）を受信するたびに、入力を出力 A, B, C, D の順にルーティング。
//! - パッチの展開（Aメロ→Bメロ）を決定論的に自動化。

use crate::signal::{
    BuiltinModuleDescriptor, PortDescriptor, PortDirection, RackDspNode, RackProcessContext,
    SignalType, TriggerDetector,
};

pub struct SeqSwitchModule {
    current_steps: [usize; 16],
    triggers: [TriggerDetector; 16],
    resets: [TriggerDetector; 16],
}

impl SeqSwitchModule {
    pub fn new(_sr: f32) -> Self {
        Self {
            current_steps: [0; 16],
            triggers: [TriggerDetector::new(); 16],
            resets: [TriggerDetector::new(); 16],
        }
    }
}

impl RackDspNode for SeqSwitchModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            let clock = inputs[1 * 16 + v]; // Port 1 (CLK)
            let reset = inputs[2 * 16 + v]; // Port 2 (RESET)

            if self.resets[v].process(reset) {
                self.current_steps[v] = 0;
            } else if self.triggers[v].process(clock) {
                self.current_steps[v] = (self.current_steps[v] + 1) % 4;
            }

            let input = inputs[0 * 16 + v]; // Port 0 (IN)
            for i in 0..4 {
                outputs[i * 16 + v] = if i == self.current_steps[v] {
                    input
                } else {
                    0.0
                };
            }
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_switch_seq",
        name: "Seq Switch",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "SEQ", "UTL"],
        params: &[],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.2, 0.2],
            },
            PortDescriptor {
                name: "CLK",
                direction: PortDirection::Input,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.2, 0.5],
            },
            PortDescriptor {
                name: "RESET",
                direction: PortDirection::Input,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.2, 0.8],
            },
            PortDescriptor {
                name: "OUT A",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.2],
            },
            PortDescriptor {
                name: "OUT B",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.4],
            },
            PortDescriptor {
                name: "OUT C",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.6],
            },
            PortDescriptor {
                name: "OUT D",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.8],
            },
        ],
        factory: |sr| Box::new(SeqSwitchModule::new(sr)),
    }
}
