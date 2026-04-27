//! Logic Module — Boolean Gate Operations
//!
//! # 憲法遵守
//! - 閾値 2.0V (crate::signal::GATE_THRESHOLD) を基準に真偽判定。
//! - AND, OR, XOR, NOT の各論理演算を提供。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SeedScope, SignalType,
};

pub struct LogicModule {}

impl LogicModule {
    pub fn new(_sr: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for LogicModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            let a = inputs[0 * 16 + v] > crate::signal::GATE_THRESHOLD;
            let b = inputs[1 * 16 + v] > crate::signal::GATE_THRESHOLD;

            outputs[0 * 16 + v] = if a && b { 5.0 } else { 0.0 }; // AND
            outputs[1 * 16 + v] = if a || b { 5.0 } else { 0.0 }; // OR
            outputs[2 * 16 + v] = if a ^ b { 5.0 } else { 0.0 }; // XOR
            outputs[3 * 16 + v] = if !a { 5.0 } else { 0.0 }; // NOT A
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_logic",
        name: "Logic",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "UTL", "LOGIC"],
        params: &[],
        ports: &[
            PortDescriptor {
                name: "A",
                direction: PortDirection::Input,
                signal_type: SignalType::Gate,
                max_channels: 1,
                position: [0.2, 0.2],
            },
            PortDescriptor {
                name: "B",
                direction: PortDirection::Input,
                signal_type: SignalType::Gate,
                max_channels: 1,
                position: [0.2, 0.4],
            },
            PortDescriptor {
                name: "AND",
                direction: PortDirection::Output,
                signal_type: SignalType::Gate,
                max_channels: 1,
                position: [0.8, 0.2],
            },
            PortDescriptor {
                name: "OR",
                direction: PortDirection::Output,
                signal_type: SignalType::Gate,
                max_channels: 1,
                position: [0.8, 0.4],
            },
            PortDescriptor {
                name: "XOR",
                direction: PortDirection::Output,
                signal_type: SignalType::Gate,
                max_channels: 1,
                position: [0.8, 0.6],
            },
            PortDescriptor {
                name: "NOT_A",
                direction: PortDirection::Output,
                signal_type: SignalType::Gate,
                max_channels: 1,
                position: [0.8, 0.8],
            },
        ],
        factory: |sr| Box::new(LogicModule::new(sr)),
    }
}
