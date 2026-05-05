//! Merkle Time-Weaver — Hash-driven deterministic glitch delay.
//!
//! バッファ内の読み出し位置を、シードと入力波形の簡易ハッシュで決定する。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType, SmoothedParam,
};

const MAX_DELAY_SAMPLES: usize = 44100 * 2; // 2 seconds at 44.1kHz

pub struct MerkleDelayModule {
    buffer: [Vec<f32>; 16], // 16 voices
    write_idx: [usize; 16],
    time_smooth: SmoothedParam,
    feedback_smooth: SmoothedParam,
    glitch_smooth: SmoothedParam,
    hash_state: [u32; 16],
    sample_rate: f32,
}

impl MerkleDelayModule {
    pub fn new(sample_rate: f32) -> Self {
        let buffer: [Vec<f32>; 16] = std::array::from_fn(|_| vec![0.0; MAX_DELAY_SAMPLES]);
        Self {
            buffer,
            write_idx: [0; 16],
            time_smooth: SmoothedParam::new(0.5, sample_rate, 50.0),
            feedback_smooth: SmoothedParam::new(0.5, sample_rate, 10.0),
            glitch_smooth: SmoothedParam::new(0.0, sample_rate, 10.0),
            hash_state: [0; 16],
            sample_rate,
        }
    }

    // A very fast, simple hash function for audio rate
    fn update_hash(state: &mut u32, input: f32) {
        let bits = input.to_bits();
        *state = state
            .wrapping_mul(1664525)
            .wrapping_add(1013904223)
            .wrapping_add(bits);
    }
}

impl RackDspNode for MerkleDelayModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        ctx: &RackProcessContext,
    ) {
        let time_knob = params[0];
        let fb_knob = params[1];
        let glitch_knob = params[2];

        self.time_smooth.set(time_knob);
        self.feedback_smooth.set(fb_knob);
        self.glitch_smooth.set(glitch_knob);

        let jitter = ctx.imperfection.drift[0];
        let base_time = self.time_smooth.next(jitter).clamp(0.001, 2.0);
        let fb = self.feedback_smooth.next(jitter).clamp(0.0, 1.2); // Allow self-oscillation
        let glitch_amt = self.glitch_smooth.next(jitter).clamp(0.0, 1.0);

        for i in 0..16 {
            let input = inputs[0 * 16 + i];

            // Update continuous hash state based on input audio
            Self::update_hash(&mut self.hash_state[i], input);

            let max_delay_samples_for_voice = base_time * self.sample_rate;
            let current_w_idx = self.write_idx[i];

            // Standard read pointer
            let mut read_ptr = (current_w_idx as f32 + MAX_DELAY_SAMPLES as f32
                - max_delay_samples_for_voice)
                % MAX_DELAY_SAMPLES as f32;

            // Merkle Glitch Modulation
            if glitch_amt > 0.001 {
                let seed_mix = (ctx.project_seed as u32).wrapping_add(i as u32);
                let jump_hash = self.hash_state[i] ^ seed_mix;

                let jump = (jump_hash % max_delay_samples_for_voice as u32) as f32 * glitch_amt;

                if (jump_hash & 0x1000) != 0 {
                    read_ptr = (read_ptr + jump) % MAX_DELAY_SAMPLES as f32;
                } else {
                    read_ptr =
                        (read_ptr + MAX_DELAY_SAMPLES as f32 - jump) % MAX_DELAY_SAMPLES as f32;
                }
            }

            // Linear Interpolation
            let i0 = read_ptr as usize;
            let i1 = (i0 + 1) % MAX_DELAY_SAMPLES;
            let frac = read_ptr - i0 as f32;
            let delayed_sample = self.buffer[i][i0] * (1.0 - frac) + self.buffer[i][i1] * frac;

            // Soft clip feedback
            let fb_signal = (delayed_sample * fb).tanh();

            self.buffer[i][current_w_idx] = input + fb_signal;

            self.write_idx[i] = (current_w_idx + 1) % MAX_DELAY_SAMPLES;

            // Mix Dry/Wet (50/50 for simplicity here, could be a parameter)
            outputs[0 * 16 + i] = (input + delayed_sample) * 0.707;
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_merkle_delay",
        name: "MRKL DLY",
        version: "1.0.0",
        manufacturer: "DirtyRack",
        hp_width: 10,
        visuals: crate::signal::ModuleVisuals {
            background_color: [40, 30, 40],
            text_color: [200, 150, 255],
            accent_color: [180, 50, 255],
            panel_texture: crate::signal::PanelTexture::MatteBlack, knob_style: crate::signal::KnobStyle::ClassicSilver,
        },
        tags: &["Builtin", "DLY", "GLITCH"],
        params: &[
            ParamDescriptor {
                name: "TIME",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.001,
                max: 2.0,
                default: 0.5,
                position: [0.5, 0.2],
                unit: "s",
            },
            ParamDescriptor {
                name: "FEEDBACK",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.2,
                default: 0.5,
                position: [0.3, 0.5],
                unit: "",
            },
            ParamDescriptor {
                name: "GLITCH",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.0,
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
        factory: |sr| Box::new(MerkleDelayModule::new(sr)),
    }
}
