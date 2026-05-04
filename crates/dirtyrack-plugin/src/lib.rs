use dirtyrack_modules::midi::MidiCvModule;
use dirtyrack_modules::output::OutputModule;
use dirtyrack_modules::runner::{Connection, GraphSnapshot, RackRunner};
use dirtyrack_modules::signal::{RackDspNode, SeedScope};
use dirtyrack_modules::vco::VcoModule;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::sync::Arc;

struct DirtyRackPlugin {
    params: Arc<DirtyRackParams>,
    runner: RackRunner,
    snapshot: GraphSnapshot,
}

#[derive(Params)]
struct DirtyRackParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "mod"]
    pub modulation: FloatParam,
}

impl Default for DirtyRackPlugin {
    fn default() -> Self {
        let sample_rate = 44100.0;
        let mut runner = RackRunner::new(sample_rate, SeedScope::Global(0));

        // Default patch: MIDI-CV -> VCO -> OUTPUT
        let midi = Box::new(MidiCvModule::new(sample_rate));
        let vco = Box::new(VcoModule::new(sample_rate));
        let output = Box::new(OutputModule::new(sample_rate));

        let mut snapshot = GraphSnapshot {
            order: vec![0, 1, 2],
            connections: vec![
                Connection {
                    from_module: 0,
                    from_port: 0,
                    to_module: 1,
                    to_port: 0,
                }, // MIDI Pitch -> VCO V/OCT
                Connection {
                    from_module: 1,
                    from_port: 0,
                    to_module: 2,
                    to_port: 0,
                }, // VCO SINE -> OUTPUT L
                Connection {
                    from_module: 1,
                    from_port: 0,
                    to_module: 2,
                    to_port: 1,
                }, // VCO SINE -> OUTPUT R
            ],
            port_counts: vec![(0, 4), (4, 4), (2, 0)],
            node_ids: vec![1, 2, 3],
            node_type_ids: vec!["midi".to_string(), "vco".to_string(), "output".to_string()],
            modulations: vec![vec![], vec![], vec![]],
            forward_edges: vec![
                vec![Connection { from_module: 0, from_port: 0, to_module: 1, to_port: 0 }],
                vec![
                    Connection { from_module: 1, from_port: 0, to_module: 2, to_port: 0 },
                    Connection { from_module: 1, from_port: 0, to_module: 2, to_port: 1 }
                ],
                vec![]
            ],
            back_edges: vec![
                Connection { from_module: 0, from_port: 0, to_module: 1, to_port: 0 },
                Connection { from_module: 1, from_port: 0, to_module: 2, to_port: 0 },
                Connection { from_module: 1, from_port: 0, to_module: 2, to_port: 1 }
            ],
        };

        let nodes: Vec<Box<dyn RackDspNode>> = vec![midi, vco, output];
        runner.apply_snapshot(&mut snapshot, nodes);

        Self {
            params: Arc::new(DirtyRackParams {
                editor_state: EguiState::from_size(800, 600),
                gain: FloatParam::new("Gain", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
                modulation: FloatParam::new(
                    "Modulation",
                    0.0,
                    FloatRange::Linear {
                        min: -1.0,
                        max: 1.0,
                    },
                )
                .with_poly_modulation_id(0),
            }),
            runner,
            snapshot,
        }
    }
}

impl Plugin for DirtyRackPlugin {
    const NAME: &'static str = "DirtyRack";
    const VENDOR: &'static str = "DirtyRack Team";
    const URL: &'static str = "https://github.com/yasuno/DirtyRack";
    const EMAIL: &'static str = "yasuno@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let params_vec = vec![
            vec![],                   // MIDI-CV
            vec![5.0, 0.0, 0.0, 0.5], // VCO
            vec![0.7],                // OUTPUT
        ];

        for mut channel_samples in buffer.iter_samples() {
            // Handle MIDI
            while let Some(event) = context.next_event() {
                match event {
                    NoteEvent::NoteOn {
                        note,
                        voice_id,
                        velocity,
                        ..
                    } => {
                        self.runner.active_nodes[0].on_midi(
                            note,
                            voice_id.unwrap_or(note as i32),
                            (velocity * 127.0) as u8,
                            true,
                        );
                    }
                    NoteEvent::NoteOff { note, voice_id, .. } => {
                        self.runner.active_nodes[0].on_midi(
                            note,
                            voice_id.unwrap_or(note as i32),
                            0,
                            false,
                        );
                    }
                    NoteEvent::PolyModulation {
                        voice_id,
                        normalized_offset,
                        ..
                    } => {
                        if let Some(midi_module) = self.runner.active_nodes[0]
                            .as_any_mut()
                            .downcast_mut::<MidiCvModule>()
                        {
                            midi_module.poly_modulate(voice_id, normalized_offset);
                        }
                    }
                    _ => (),
                }
            }

            self.runner.process_sample(&self.snapshot, &params_vec);

            // OutputModule (idx 2)
            // Port 0 is LEFT, Port 1 is RIGHT. (16ch mono-cable architecture)
            let left = self.runner.output_buffers[2][0 * 16]; // Voice 0
            let right = self.runner.output_buffers[2][1 * 16];

            let gain = self.params.gain.value();
            *channel_samples.get_mut(0).unwrap() = left * gain;
            *channel_samples.get_mut(1).unwrap() = right * gain;
        }

        ProcessStatus::Normal
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            |ctx, _setter, _state| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("DirtyRack VST3");
                    ui.label("Instrument-like Modular Rack");
                    ui.separator();
                    ui.label("Currently running default patch:");
                    ui.label("MIDI-CV -> VCO -> OUTPUT");
                });
            },
        )
    }
}

impl Vst3Plugin for DirtyRackPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"DirtyRackPluginX";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

impl ClapPlugin for DirtyRackPlugin {
    const CLAP_ID: &'static str = "com.dirtyrack.synth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Instrument-like Modular Rack");
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
    const CLAP_MANUAL_URL: Option<&'static str> = Some("https://github.com/yasuno/DirtyRack");
    const CLAP_SUPPORT_URL: Option<&'static str> =
        Some("https://github.com/yasuno/DirtyRack/issues");
}

nih_export_vst3!(DirtyRackPlugin);
nih_export_clap!(DirtyRackPlugin);
