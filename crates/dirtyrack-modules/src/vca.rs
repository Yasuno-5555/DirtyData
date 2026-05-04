//! VCA Module — Voltage Controlled Amplifier

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};

pub struct VcaModule {
    gain_states: [f32; 16],
}

impl VcaModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self {
            gain_states: [0.0; 16],
        }
    }
}

impl RackDspNode for VcaModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        ctx: &RackProcessContext,
    ) {
        let level_knob = params[0];
        let cv_amt = params[1];

        for i in 0..16 {
            let input = inputs[0 * 16 + i];
            let cv = inputs[1 * 16 + i];

            // 個体差によるゲインのズレ
            let p_offset = ctx.imperfection.personality[i] * 0.02;

            // target gain calculation
            let target_gain = (level_knob + (cv / 5.0) * cv_amt + p_offset).clamp(0.0, 4.0);
            
            // Smoothing filter (1-pole LP) to prevent clicks
            // Time constant ~ 1ms
            let alpha = 0.99; 
            self.gain_states[i] = self.gain_states[i] * alpha + target_gain * (1.0 - alpha);

            // 非線形性 (Analog saturation)
            let gain = libm::tanhf(self.gain_states[i]);

            outputs[0 * 16 + i] = input * gain;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn vca_factory(sr: f32) -> Box<dyn RackDspNode> {
    Box::new(VcaModule::new(sr))
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_vca",
        name: "VCA",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default_const(),
        tags: &["Builtin", "AMP", "VCA"],
        params: &[
            ParamDescriptor {
                name: "LEVEL",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.5, 0.3],
                unit: "",
            },
            ParamDescriptor {
                name: "CV_AMT",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 1.0,
                position: [0.5, 0.6],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.5, 0.1],
            },
            PortDescriptor {
                name: "CV",
                direction: PortDirection::Input,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.5, 0.8],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.5, 0.95],
            },
        ],
        factory: vca_factory,
    }
}
