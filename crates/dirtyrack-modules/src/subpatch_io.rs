//! Subpatch IO Modules — Internal ports for Composite Modules.

use crate::signal::{PortDescriptor, PortDirection, RackDspNode, RackProcessContext, SignalType};

/// Represents an INPUT port of the parent CompositeModule from *inside* the subpatch.
pub struct SubpatchInput {
    pub port_index: usize,
    pub signals: [f32; 16],
}

impl SubpatchInput {
    pub fn new(port_index: usize) -> Self {
        Self {
            port_index,
            signals: [0.0; 16],
        }
    }
}

impl RackDspNode for SubpatchInput {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            outputs[v] = self.signals[v];
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn input_descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "subpatch_in",
        name: "SUB IN",
        version: "1.0.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Internal"],
        params: &[],
        ports: &[PortDescriptor {
            name: "OUT",
            direction: PortDirection::Output,
            signal_type: SignalType::Audio,
            max_channels: 1,
            position: [0.5, 0.8],
        }],
        factory: |_sr| Box::new(SubpatchInput::new(0)), // Default index, will be patched if needed
    }
}

/// Represents an OUTPUT port of the parent CompositeModule from *inside* the subpatch.
pub struct SubpatchOutput {
    pub port_index: usize,
    pub signals: [f32; 16],
}

impl SubpatchOutput {
    pub fn new(port_index: usize) -> Self {
        Self {
            port_index,
            signals: [0.0; 16],
        }
    }
}

impl RackDspNode for SubpatchOutput {
    fn process(
        &mut self,
        inputs: &[f32],
        _outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            self.signals[v] = inputs[v];
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn output_descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "subpatch_out",
        name: "SUB OUT",
        version: "1.0.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Internal"],
        params: &[],
        ports: &[PortDescriptor {
            name: "IN",
            direction: PortDirection::Input,
            signal_type: SignalType::Audio,
            max_channels: 1,
            position: [0.5, 0.8],
        }],
        factory: |_sr| Box::new(SubpatchOutput::new(0)),
    }
}

/// Represents a PARAMETER (Macro Knob) of the parent CompositeModule from *inside* the subpatch.
pub struct SubpatchParameter {
    pub param_index: usize,
    pub value: f32,
}

impl SubpatchParameter {
    pub fn new(param_index: usize) -> Self {
        Self {
            param_index,
            value: 0.0,
        }
    }
}

impl RackDspNode for SubpatchParameter {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        // Output the parameter value as a CV signal
        for v in 0..16 {
            outputs[v] = self.value;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn parameter_descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "subpatch_param",
        name: "SUB PARAM",
        version: "1.0.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Internal"],
        params: &[],
        ports: &[PortDescriptor {
            name: "CV",
            direction: PortDirection::Output,
            signal_type: SignalType::UniCV,
            max_channels: 1,
            position: [0.5, 0.8],
        }],
        factory: |_sr| Box::new(SubpatchParameter::new(0)),
    }
}
