//! Quantizer Module — Scale Quantizer
//!
//! # Parameters
//! - SCALE: 音階選択
//!
//! # Inputs
//! - IN: CV入力 (1V/Oct)
//!
//! # Outputs
//! - OUT: 量子化済みCV

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};

pub struct QuantizerModule {}

impl QuantizerModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for QuantizerModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            let input = inputs[0 * 16 + v];
            // 12-TET quantization: round to nearest 1/12V
            outputs[0 * 16 + v] = (input * 12.0).round() / 12.0;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_quantizer",
        name: "QUANT",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "UTL", "PITCH"],
        params: &[ParamDescriptor {
            name: "SCALE",
            kind: ParamKind::Knob,
            response: ParamResponse::Immediate,
            min: 0.0,
            max: 1.0,
            default: 0.0,
            position: [0.5, 0.3],
            unit: "",
        }],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::VoltPerOct,
                max_channels: 1,
                position: [0.5, 0.7],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::VoltPerOct,
                max_channels: 1,
                position: [0.5, 0.9],
            },
        ],
        factory: |sr| Box::new(QuantizerModule::new(sr)),
    }
}
