//! Eroding Tape — Entropy-driven buffer degradation.
//!
//! バッファに記憶された音が、時間経過とともに物理法則（熱力学の第二法則的）に
//! 従って徐々にノイズへと還元されていく。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection,
    RackDspNode, RackProcessContext, SignalType, SmoothedParam,
};

const MAX_TAPE_SAMPLES: usize = 44100 * 4; // 4 seconds

pub struct ErosionModule {
    buffer: [Vec<f32>; 16],
    write_idx: [usize; 16],
    time_smooth: SmoothedParam,
    entropy_smooth: SmoothedParam,
    feedback_smooth: SmoothedParam,
    noise_state: [u32; 16],
    sample_rate: f32,
}

impl ErosionModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buffer: std::array::from_fn(|_| vec![0.0; MAX_TAPE_SAMPLES]),
            write_idx: [0; 16],
            time_smooth: SmoothedParam::new(1.0, sample_rate, 50.0),
            entropy_smooth: SmoothedParam::new(0.1, sample_rate, 50.0),
            feedback_smooth: SmoothedParam::new(0.9, sample_rate, 50.0),
            noise_state: std::array::from_fn(|i| 123456789 + i as u32),
            sample_rate,
        }
    }

    fn fast_rand(state: &mut u32) -> f32 {
        *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        (*state as f32) / (std::u32::MAX as f32) * 2.0 - 1.0
    }
}

impl RackDspNode for ErosionModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let time_knob = params[0];
        let entropy_knob = params[1];
        let fb_knob = params[2];

        self.time_smooth.set(time_knob);
        self.entropy_smooth.set(entropy_knob);
        self.feedback_smooth.set(fb_knob);

        let time_val = self.time_smooth.next(0.0).clamp(0.001, 4.0);
        let entropy = self.entropy_smooth.next(0.0).clamp(0.0, 1.0);
        let fb = self.feedback_smooth.next(0.0).clamp(0.0, 1.5);

        for i in 0..16 {
            let input = inputs[0 * 16 + i];
            
            let delay_samples = (time_val * self.sample_rate) as usize;
            let current_w_idx = self.write_idx[i];
            
            let read_idx = (current_w_idx + MAX_TAPE_SAMPLES - delay_samples) % MAX_TAPE_SAMPLES;
            let mut delayed_sample = self.buffer[i][read_idx];

            // --- Erosion Process ---
            if entropy > 0.001 {
                // 1. Amplitude decay (energy loss)
                delayed_sample *= 1.0 - (entropy * 0.001);

                // 2. Add thermal noise (entropy increase)
                let noise = Self::fast_rand(&mut self.noise_state[i]);
                delayed_sample += noise * entropy * 0.05;

                // 3. Simple Lowpass filtering to simulate magnetic particle loss
                // Use the adjacent sample in the buffer to average
                let prev_read_idx = if read_idx == 0 { MAX_TAPE_SAMPLES - 1 } else { read_idx - 1 };
                let prev_sample = self.buffer[i][prev_read_idx];
                
                let filter_coeff = (entropy * 0.1).clamp(0.0, 0.99);
                delayed_sample = delayed_sample * (1.0 - filter_coeff) + prev_sample * filter_coeff;
            }

            // Write back with feedback
            let write_val = input + (delayed_sample * fb).clamp(-5.0, 5.0);
            self.buffer[i][current_w_idx] = write_val;

            self.write_idx[i] = (current_w_idx + 1) % MAX_TAPE_SAMPLES;

            // Output mix
            outputs[0 * 16 + i] = delayed_sample;
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_erosion",
        name: "EROSION",
        version: "1.0.0",
        manufacturer: "DirtyRack",
        hp_width: 10,
        visuals: crate::signal::ModuleVisuals {
            background_color: [50, 40, 30],
            text_color: [255, 200, 150],
            accent_color: [255, 100, 50],
            panel_texture: crate::signal::PanelTexture::VintageCream,
        },
        tags: &["Builtin", "DLY", "LOFI", "FX"],
        params: &[
            ParamDescriptor {
                name: "TIME",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.001,
                max: 4.0,
                default: 1.0,
                position: [0.5, 0.2],
                unit: "s",
            },
            ParamDescriptor {
                name: "ENTROPY",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.0,
                max: 1.0,
                default: 0.1,
                position: [0.3, 0.5],
                unit: "",
            },
            ParamDescriptor {
                name: "FEEDBACK",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.0,
                max: 1.5, // Allow runaway feedback
                default: 0.9,
                position: [0.7, 0.5],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.2, 0.85],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.8, 0.85],
            },
        ],
        factory: |sr| Box::new(ErosionModule::new(sr)),
    }
}
