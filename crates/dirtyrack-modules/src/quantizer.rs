//! Quantizer Module — Scale Quantizer
//!
//! # Parameters
//! - SCALE: 音階選択
//!
//! # Inputs
//! - IN: CV入力 (1V/Oct)
//!
//! # Outputs
//! - OUT: 量子化済みCV

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};

pub struct QuantizerModule {}

impl QuantizerModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self {}
    }
}

impl RackDspNode for QuantizerModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let transpose = params[0];
        let scale_mode = params[1] as usize; // 0: Chrom, 1: Major, 2: Minor, 3: Pent, 4: Blues

        let scales: [&[f32]; 5] = [
            &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0], // Chromatic
            &[0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0],                           // Major
            &[0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0],                           // Minor
            &[0.0, 2.0, 4.0, 7.0, 9.0],                                      // Pentatonic
            &[0.0, 3.0, 5.0, 6.0, 7.0, 10.0],                                // Blues
        ];

        let active_scale = scales.get(scale_mode).unwrap_or(&scales[0]);

        for v in 0..16 {
            let input = inputs[0 * 16 + v];

            // 1V/oct: 0.0V is C4, 1.0V is C5, etc.
            // 12 semitones per volt
            let semitones = (input * 12.0) + transpose;
            let octave = libm::floorf(semitones / 12.0);
            let note = semitones - octave * 12.0;

            // Find closest note in scale
            let mut closest = active_scale[0];
            let mut min_diff = 100.0;
            for &s_note in *active_scale {
                let diff = libm::fabsf(note - s_note);
                if diff < min_diff {
                    min_diff = diff;
                    closest = s_note;
                }
            }

            outputs[0 * 16 + v] = (octave * 12.0 + closest) / 12.0;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_quantizer",
        name: "QUANT",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 4,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "UTL", "PITCH"],
        params: &[
            ParamDescriptor {
                name: "TRANSPOSE",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: -12.0,
                max: 12.0,
                default: 0.0,
                position: [0.5, 0.3],
                unit: "ST",
            },
            ParamDescriptor {
                name: "SCALE",
                kind: ParamKind::Switch { positions: 5 },
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 4.0,
                default: 0.0,
                position: [0.5, 0.6],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::VoltPerOct,
                max_channels: 1,
                position: [0.5, 0.7],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::VoltPerOct,
                max_channels: 1,
                position: [0.5, 0.9],
            },
        ],
        factory: |sr| Box::new(QuantizerModule::new(sr)),
    }
}
