//! Certified Recorder Module — 監査ログ付きレンダラー
//! 
//! ただの録音ではなく、レンダリング結果のハッシュ値を生成し、
//! 再現性を「証明」するための監査データを生成する。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};
use std::fs::File;
use std::io::BufWriter;
use hound::{WavSpec, WavWriter};

pub struct RecorderModule {
    is_recording: bool,
    sample_rate: f32,
    hasher: blake3::Hasher,
    wav_writer: Option<WavWriter<BufWriter<File>>>,
    render_id: String,
    sample_count: u64,
    certificate_metadata: Option<serde_json::Value>,
}

impl RecorderModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            is_recording: false,
            sample_rate,
            hasher: blake3::Hasher::new(),
            wav_writer: None,
            render_id: String::new(),
            sample_count: 0,
            certificate_metadata: None,
        }
    }

    pub fn set_metadata(&mut self, meta: serde_json::Value) {
        self.certificate_metadata = Some(meta);
    }

    fn start_recording(&mut self) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.render_id = format!("render_{}", timestamp);
        
        let spec = WavSpec {
            channels: 2,
            sample_rate: self.sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        if let Ok(writer) = WavWriter::create(format!("{}.wav", self.render_id), spec) {
            self.wav_writer = Some(writer);
            self.is_recording = true;
            self.hasher = blake3::Hasher::new();
            self.sample_count = 0;
        }
    }

    fn stop_recording(&mut self) {
        if let Some(writer) = self.wav_writer.take() {
            let _ = writer.finalize();
            let hash = self.hasher.finalize();
            
            // Render Certificate の生成
            let mut cert = if let Some(meta) = self.certificate_metadata.take() {
                meta
            } else {
                serde_json::json!({})
            };

            cert["render_hash"] = serde_json::Value::String(hash.to_hex().to_string());
            cert["sample_count"] = serde_json::Value::Number(self.sample_count.into());
            cert["status"] = serde_json::Value::String("CERTIFIED".to_string());

            if let Ok(f) = File::create(format!("{}.dirtyrack.cert", self.render_id)) {
                let _ = serde_json::to_writer_pretty(f, &cert);
            }
        }
        self.is_recording = false;
    }
}

impl RackDspNode for RecorderModule {
    fn process(
        &mut self,
        inputs: &[f32],
        _outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let rec_button = params[0] > 0.5;

        if rec_button && !self.is_recording {
            self.start_recording();
        } else if !rec_button && self.is_recording {
            self.stop_recording();
        }

        if self.is_recording {
            if let Some(writer) = &mut self.wav_writer {
                for i in 0..16 {
                    let l = inputs[0 * 16 + i];
                    let r = inputs[1 * 16 + i];

                    // ハッシュの更新
                    self.hasher.update(&l.to_le_bytes());
                    self.hasher.update(&r.to_le_bytes());

                    // WAVの書き込み
                    let _ = writer.write_sample(l);
                    let _ = writer.write_sample(r);
                    self.sample_count += 1;
                }
            }
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_recorder",
        name: "CERTIFIED REC",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "UTL", "REC"],
        params: &[
            ParamDescriptor {
                name: "RECORD",
                kind: ParamKind::Button,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.5, 0.5],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN_L",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.3, 0.8],
            },
            PortDescriptor {
                name: "IN_R",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.7, 0.8],
            },
        ],
        factory: |sr| Box::new(RecorderModule::new(sr)),
    }
}
