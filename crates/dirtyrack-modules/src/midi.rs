//! MIDI-CV Module — The External Bridge (Polyphonic)
//!
//! # 憲法遵守
//! - MIDIイベントをサンプル精度でCVに変換。
//! - 16チャンネル（1V/Oct, Gate, Vel, Trig）を1本のケーブルに多重化して出力。

use crate::signal::{
    PortDescriptor, PortDirection, RackDspNode, RackProcessContext,
    SignalType,
};

#[derive(Debug, Clone, Copy)]
struct Voice {
    pitch: f32,
    gate: f32,
    velocity: f32,
    modulation: f32, // Polyphonic modulation (e.g. from CLAP)
    trig_timer: usize,
    note: u8,
    note_id: i32,
    active: bool,
}

pub struct MidiCvModule {
    voices: [Voice; 16],
}

impl MidiCvModule {
    pub fn new(_sr: f32) -> Self {
        Self {
            voices: [Voice {
                pitch: 0.0,
                gate: 0.0,
                velocity: 0.0,
                modulation: 0.0,
                trig_timer: 0,
                note: 0,
                note_id: -1,
                active: false,
            }; 16],
        }
    }

    pub fn poly_modulate(&mut self, note_id: i32, value: f32) {
        if let Some(voice) = self
            .voices
            .iter_mut()
            .find(|v| v.active && v.note_id == note_id)
        {
            voice.modulation = value;
        }
    }

    pub fn note_on(&mut self, note: u8, note_id: i32, velocity: u8) {
        // Simple voice stealing: find first inactive or oldest
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.active) {
            voice.note = note;
            voice.note_id = note_id;
            voice.pitch = (note as f32 - 60.0) / 12.0; // 0V = C4
            voice.gate = 5.0;
            voice.velocity = (velocity as f32 / 127.0) * 5.0;
            voice.trig_timer = 44; // ~1ms at 44.1kHz
            voice.active = true;
        }
    }

    pub fn note_off(&mut self, note_id: i32) {
        if let Some(voice) = self
            .voices
            .iter_mut()
            .find(|v| v.active && v.note_id == note_id)
        {
            voice.gate = 0.0;
            voice.active = false;
        }
    }
}

impl RackDspNode for MidiCvModule {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for i in 0..16 {
            let v = &mut self.voices[i];

            // 1V/OCT (Port 0)
            outputs[0 * 16 + i] = v.pitch + v.modulation;
            // GATE (Port 1)
            outputs[1 * 16 + i] = v.gate;
            // TRIG (Port 2)
            if v.trig_timer > 0 {
                v.trig_timer -= 1;
                outputs[2 * 16 + i] = 5.0;
            } else {
                outputs[2 * 16 + i] = 0.0;
            }
            // VEL (Port 3)
            outputs[3 * 16 + i] = v.velocity;
            // MOD (Port 4)
            outputs[4 * 16 + i] = v.modulation;
        }
    }

    fn on_midi(&mut self, note: u8, note_id: i32, velocity: u8, is_on: bool) {
        if is_on {
            self.note_on(note, note_id, velocity);
        } else {
            self.note_off(note_id);
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_midi_cv",
        name: "MIDI-CV",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[],
        ports: &[
            PortDescriptor {
                name: "1V/OCT",
                direction: PortDirection::Output,
                signal_type: SignalType::VoltPerOct,
                max_channels: 16,
                position: [0.5, 0.15],
            },
            PortDescriptor {
                name: "GATE",
                direction: PortDirection::Output,
                signal_type: SignalType::Gate,
                max_channels: 16,
                position: [0.5, 0.32],
            },
            PortDescriptor {
                name: "TRIG",
                direction: PortDirection::Output,
                signal_type: SignalType::Trigger,
                max_channels: 16,
                position: [0.5, 0.49],
            },
            PortDescriptor {
                name: "VEL",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 16,
                position: [0.5, 0.66],
            },
            PortDescriptor {
                name: "MOD",
                direction: PortDirection::Output,
                signal_type: SignalType::UniCV,
                max_channels: 16,
                position: [0.5, 0.83],
            },
        ],
        factory: |sr| Box::new(MidiCvModule::new(sr)),
    }
}
