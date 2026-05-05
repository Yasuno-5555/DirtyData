//! Noise Module — White/Pink Noise
//!
//! # Outputs
//! - WHITE: ホワイトノイズ
//! - PINK: ピンクノイズ (予定)

use crate::signal::{PortDescriptor, PortDirection, RackDspNode, RackProcessContext, SignalType};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct NoiseModule {
    rngs: [ChaCha8Rng; 16],
    pink_states: [[f32; 7]; 16],
}

impl NoiseModule {
    pub fn new(_sr: f32) -> Self {
        Self {
            rngs: std::array::from_fn(|i| ChaCha8Rng::seed_from_u64(0x1337 + i as u64)),
            pink_states: [[0.0; 7]; 16],
        }
    }
}

impl RackDspNode for NoiseModule {
    fn reset(&mut self) {
        for i in 0..16 {
            self.rngs[i] = ChaCha8Rng::seed_from_u64(0x1337 + i as u64);
            self.pink_states[i] = [0.0; 7];
        }
    }

    fn randomize(&mut self, seed: u64) {
        for i in 0..16 {
            self.rngs[i] = ChaCha8Rng::seed_from_u64(seed + i as u64);
        }
    }
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            let white: f32 = self.rngs[v].gen_range(-1.0..1.0);

            // Voss-McCartney approximation for Pink Noise
            self.pink_states[v][0] = 0.99886 * self.pink_states[v][0] + white * 0.0555179;
            self.pink_states[v][1] = 0.99332 * self.pink_states[v][1] + white * 0.0750312;
            self.pink_states[v][2] = 0.96900 * self.pink_states[v][2] + white * 0.1538520;
            self.pink_states[v][3] = 0.86650 * self.pink_states[v][3] + white * 0.3104856;
            self.pink_states[v][4] = 0.55000 * self.pink_states[v][4] + white * 0.5329522;
            self.pink_states[v][5] = -0.7616 * self.pink_states[v][5] - white * 0.0168980;

            let pink = self.pink_states[v][0]
                + self.pink_states[v][1]
                + self.pink_states[v][2]
                + self.pink_states[v][3]
                + self.pink_states[v][4]
                + self.pink_states[v][5]
                + self.pink_states[v][6]
                + white * 0.5362;
            self.pink_states[v][6] = white * 0.115926;

            outputs[0 * 16 + v] = white * 5.0; // WHITE (5V peak)
            outputs[1 * 16 + v] = pink * 0.75; // PINK
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_noise",
        name: "NOISE",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "OSC", "UTL"],
        params: &[],
        ports: &[
            PortDescriptor {
                name: "WHITE",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.5, 0.7],
            },
            PortDescriptor {
                name: "PINK",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.5, 0.9],
            },
        ],
        factory: |sr| Box::new(NoiseModule::new(sr)),
    }
}
