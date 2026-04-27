use dirtyrack_sdk::*;

pub struct SimpleGain {
    sample_rate: f32,
}

impl SimpleGain {
    pub fn new(sample_rate: f32) -> Self {
        Self { sample_rate }
    }
}

impl RackDspNode for SimpleGain {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let gain = params[0];
        for i in 0..16 {
            outputs[0 * 16 + i] = inputs[0 * 16 + i] * gain;
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

pub fn descriptor() -> &'static ModuleDescriptor {
    &ModuleDescriptor {
        id: "com.dirtyrack.example.gain",
        name: "SDK Example Gain",
        version: "1.1.0",
        manufacturer: "DirtyRack SDK",
        hp_width: 4,
        visuals: ModuleVisuals {
            background_color: [60, 60, 60],
            text_color: [255, 255, 255],
            accent_color: [255, 200, 0],
            panel_texture: PanelTexture::MatteBlack,
        },
        tags: &["Example", "Utility"],
        params: &[ParamDescriptor {
            name: "GAIN",
            kind: ParamKind::Knob,
            response: ParamResponse::Immediate,
            min: 0.0,
            max: 2.0,
            default: 1.0,
            position: [0.5, 0.5],
            unit: "x",
        }],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.1],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.9],
            },
        ],
        factory: |sr| Box::new(SimpleGain::new(sr)),
    }
}

export_dirty_module!(descriptor);
