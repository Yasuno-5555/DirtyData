//! Output Module — ラックの最終出口。
//! DirtyRack (Mono) から DirtyData (Stereo) へのブリッジを担う。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};

pub struct OutputModule {}

impl OutputModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for OutputModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let master = params.get(0).copied().unwrap_or(0.0);
        let num_voices = 16;

        for i in 0..num_voices {
            // Safety: Check if we have enough inputs before accessing
            let l_idx = i;
            let r_idx = 16 + i;

            let l = inputs.get(l_idx).copied().unwrap_or(0.0) * master;
            let r = inputs.get(r_idx).copied().unwrap_or(0.0) * master;

            // Output indices are also guarded
            if outputs.len() >= 32 {
                outputs[0 * 16 + i] = libm::tanhf(l * 0.8);
                outputs[1 * 16 + i] = libm::tanhf(r * 0.8);
            }
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_output",
        name: "AUDIO OUT",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[ParamDescriptor {
            name: "MASTER",
            kind: ParamKind::Knob,
            response: ParamResponse::Smoothed { ms: 20.0 },
            min: 0.0,
            max: 2.0, // Allow more gain
            default: 1.0,
            position: [0.5, 0.3],
            unit: "dB",
        }],
        ports: &[
            PortDescriptor {
                name: "LEFT",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.3, 0.6],
            },
            PortDescriptor {
                name: "RIGHT",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.7, 0.6],
            },
            PortDescriptor {
                name: "OUT L",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.3, 0.85],
            },
            PortDescriptor {
                name: "OUT R",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.7, 0.85],
            },
        ],
        factory: |sr| Box::new(OutputModule::new(sr)),
    }
}
