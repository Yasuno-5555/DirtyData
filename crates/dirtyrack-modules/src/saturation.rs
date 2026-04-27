//! Saturation Module — 非線形回路モデリング (Soft Saturation)
//! 
//! # 憲法遵守
//! - Diode Clipper: tanh ベースのソフトクリッピング。
//! - Transistor: 非対称な飽和特性。
//! - Tape: 非対称なコンプレッション特性。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};

pub enum SatMode {
    Diode,
    Transistor,
    Tape,
}

pub struct SaturationModule {
    #[allow(dead_code)]
    mode: SatMode,
}

impl SaturationModule {
    pub fn new() -> Self {
        Self { mode: SatMode::Diode }
    }

    fn process_sample(&self, x: f32, drive: f32, mode: &SatMode) -> f32 {
        let x = x * drive;
        match mode {
            SatMode::Diode => libm::tanhf(x),
            SatMode::Transistor => {
                if x > 0.0 {
                    1.0 - libm::expf(-x)
                } else {
                    - (1.0 - libm::expf(x)) * 0.7 // Asymmetry
                }
            }
            SatMode::Tape => {
                // Simplified tape asymmetry: push positive harder
                let x = if x > 0.0 { x * 1.2 } else { x };
                libm::tanhf(x * 0.8)
            }
        }
    }
}

impl RackDspNode for SaturationModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let drive = libm::powf(10.0, params[0] / 5.0); // 0..10V -> 1..100x gain
        let mode_idx = params[1] as usize;
        let mode = match mode_idx {
            0 => SatMode::Diode,
            1 => SatMode::Transistor,
            _ => SatMode::Tape,
        };

        for v in 0..16 {
            let input = inputs[v];
            outputs[v] = self.process_sample(input, drive, &mode) * 5.0; // Back to 5V range
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_saturation",
        name: "DRIVE",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals {
            background_color: [60, 20, 20],
            text_color: [255, 150, 100],
            accent_color: [255, 50, 50],
            panel_texture: crate::signal::PanelTexture::BrushedAluminium,
        },
        tags: &["Builtin", "DST"],
        params: &[
            ParamDescriptor {
                name: "DRIVE",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 10.0,
                default: 0.0,
                position: [0.5, 0.3],
                unit: "dB",
            },
            ParamDescriptor {
                name: "MODE",
                kind: ParamKind::Switch { positions: 3 },
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 2.0,
                default: 0.0,
                position: [0.5, 0.6],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.8],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.95],
            },
        ],
        factory: |_| Box::new(SaturationModule::new()),
    }
}
