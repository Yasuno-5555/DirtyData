//! Sample & Hold Module — Entropy Capture
//!
//! # 憲法遵守
//! - トリガー入力を正確に検出し、その瞬間の入力を保持。
//! - 決定論的リプレイにおいて、同一のタイミングで同一の値を保持することを保証。

use crate::signal::{
    BuiltinModuleDescriptor, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType, TriggerDetector,
};

pub struct SampleHoldModule {
    held_value: f32,
    trigger: TriggerDetector,
}

impl SampleHoldModule {
    pub fn new(_sr: f32) -> Self {
        Self {
            held_value: 0.0,
            trigger: TriggerDetector::new(),
        }
    }
}

impl RackDspNode for SampleHoldModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let input = inputs[0 * 16]; // Port 0 (IN)
        let trig_in = inputs[1 * 16]; // Port 1 (TRIG)

        if self.trigger.process(trig_in) {
            self.held_value = input;
        }

        for v in 0..16 {
            outputs[0 * 16 + v] = self.held_value;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_sh",
        name: "S&H",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "UTL", "MOD"],
        params: &[],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.5, 0.2],
            },
            PortDescriptor {
                name: "TRIG",
                direction: PortDirection::Input,
                signal_type: SignalType::Trigger,
                max_channels: 1,
                position: [0.5, 0.5],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.5, 0.8],
            },
        ],
        factory: |sr| Box::new(SampleHoldModule::new(sr)),
    }
}
