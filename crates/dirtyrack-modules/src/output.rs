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
        let master = params[0];

        for i in 0..16 {
            let l = inputs[i] * master;
            let r = inputs[16 + i] * master;

            // Soft-clipping limiter (tanh) to prevent digital harshness
            // Port 2 (OUT_L) is the first output port (index 0..16)
            outputs[0 * 16 + i] = libm::tanhf(l * 0.2) * 5.0;
            // Port 3 (OUT_R) is the second output port (index 16..32)
            outputs[1 * 16 + i] = libm::tanhf(r * 0.2) * 5.0;
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
            max: 1.0,
            default: 0.7,
            position: [0.5, 0.4],
            unit: "dB",
        }],
        ports: &[
            PortDescriptor {
                name: "LEFT",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.3, 0.8],
            },
            PortDescriptor {
                name: "RIGHT",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.7, 0.8],
            },
            PortDescriptor {
                name: "OUT_L",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.0, 0.0],
            },
            PortDescriptor {
                name: "OUT_R",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.0, 0.0],
            },
        ],
        factory: |sr| Box::new(OutputModule::new(sr)),
    }
}
