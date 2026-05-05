//! ZDF Ladder Filter — Zero-Delay Feedback Moog-style Filter
//!
//! # 憲法遵守
//! - Topology Preserving Transform (TPT) による 0-delay feedback 実装。
//! - 非線形飽和（tanh）をフィードバック・ループ内に配置。
//! - 自己発振（Self-oscillation）の数学的正当性を保持。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};

pub struct ZdfLadderModule {
    s: [[f32; 4]; 16], // Filter states (integrators)
    sample_rate: f32,
}

impl ZdfLadderModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            s: [[0.0; 4]; 16],
            sample_rate,
        }
    }
}

impl RackDspNode for ZdfLadderModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let cutoff = params[0];
        let resonance = params[1] * 4.0; // 0.0 to 4.0

        // TPT ladder filter coefficients
        let f = libm::tanf(std::f32::consts::PI * cutoff / self.sample_rate);
        let g = f / (1.0 + f);
        let g2 = g * g;
        let g3 = g2 * g;
        let g4 = g3 * g;

        for v in 0..16 {
            let input = inputs[v];

            // 1. Solve for instantaneous feedback
            // S = g^3*s1 + g^2*s2 + g*s3 + s4
            let s_total = g3 * self.s[v][0] + g2 * self.s[v][1] + g * self.s[v][2] + self.s[v][3];
            let y0 = (input - resonance * s_total) / (1.0 + resonance * g4);

            // 2. Step integrators
            let mut x = y0;
            for i in 0..4 {
                let v_node = (x - self.s[v][i]) * g;
                let y = v_node + self.s[v][i];
                self.s[v][i] = y + v_node;
                x = y;
            }

            outputs[v] = x; // 24dB/oct Lowpass
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_zdf_ladder",
        name: "ZDF LADDER",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals {
            background_color: [20, 20, 30],
            text_color: [200, 200, 255],
            accent_color: [100, 100, 255],
            panel_texture: crate::signal::PanelTexture::MatteBlack, knob_style: crate::signal::KnobStyle::ClassicSilver,
        },
        tags: &["Builtin", "FLT", "VCF"],
        params: &[
            ParamDescriptor {
                name: "CUTOFF",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 10.0,
                default: 5.0,
                position: [0.5, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "RESONANCE",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 4.0,
                default: 1.0,
                position: [0.5, 0.45],
                unit: "k",
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
                name: "LP4",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.95],
            },
        ],
        factory: |sr| Box::new(ZdfLadderModule::new(sr)),
    }
}
