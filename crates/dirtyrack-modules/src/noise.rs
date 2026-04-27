//! Noise Module — White/Pink Noise
//!
//! # Outputs
//! - WHITE: ホワイトノイズ
//! - PINK: ピンクノイズ (予定)

use crate::signal::{
    ParamDescriptor, PortDescriptor, PortDirection, RackDspNode, RackProcessContext, SeedScope,
    SignalType,
};

pub struct NoiseModule {
    state: u32,
    initial_seed: u32,
}

impl NoiseModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self { state: 0xACE1, initial_seed: 0xACE1 } 
    }
}

impl RackDspNode for NoiseModule {
    fn reset(&mut self) {
        self.state = self.initial_seed;
    }

    fn randomize(&mut self, seed: u64) {
        self.initial_seed = (seed & 0xFFFFFFFF) as u32;
        self.state = self.initial_seed;
    }
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            // Simple Xorshift per voice (using the same state for simplicity, but we could seed per voice)
            let mut x = self.state.wrapping_add(v as u32);
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.state = x;

            // WHITE NOISE: Map u32 to -5.0..5.0
            let white = (x as f32 / u32::MAX as f32) * 10.0 - 5.0;
            outputs[0 * 16 + v] = white;

            // PINK NOISE: Simple filter approximation
            // (In a real implementation, we'd use a Voss-McCartney or similar)
            // For now, a simple integrator to give it some 'weight'
            let pink = (white * 0.1).clamp(-5.0, 5.0); // Placeholder but better than 0
            outputs[1 * 16 + v] = pink;
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
