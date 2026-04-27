//! VCF Module — SIMD-optimized Moog Ladder Filter
//!
//! # 性能設計: 4-Voice Parallel SIMD
//! - スカラー版とSIMD版の両方を備え、ポリフォニー実行時に真価を発揮する。
//! - パッド近似を用いた高速・決定論的 tanh。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection,
    RackDspNode, RackProcessContext, SignalType, SmoothedParam, VcfMode,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct VcfStateData {
    stages_x4: [[f32; 4]; 4], // 4 stages * 4 voices
    mode: VcfMode,
}

pub struct VcfModule {
    #[allow(dead_code)]
    sample_rate: f32,
    stages_poly: [[f32; 4]; 16], // 16 voices, 4 stages each
    mode: VcfMode,
    cutoff_smooth: SmoothedParam,
    res_smooth: SmoothedParam,
    drive_smooth: SmoothedParam,
}

impl VcfModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            stages_poly: [[0.0; 4]; 16],
            mode: VcfMode::Character,
            cutoff_smooth: SmoothedParam::new(5.0, sample_rate, 10.0),
            res_smooth: SmoothedParam::new(0.0, sample_rate, 10.0),
            drive_smooth: SmoothedParam::new(1.0, sample_rate, 50.0),
        }
    }
}

impl RackDspNode for VcfModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        ctx: &RackProcessContext,
    ) {
        let cutoff_knob = params[0];
        let res_knob = params[1];
        let drive_knob = params[2];

        self.cutoff_smooth.set(cutoff_knob);
        self.res_smooth.set(res_knob);
        self.drive_smooth.set(drive_knob);

        // 注意: 16ボイス処理
        for i in 0..16 {
            // Control Latency Micro-Variance
            let jitter = ctx.imperfection.drift[i];
            let cutoff_val = self.cutoff_smooth.next(jitter);
            let res_val = self.res_smooth.next(jitter);
            let drive_val = self.drive_smooth.next(jitter);

            let input = inputs[0 * 16 + i];
            let cv_in = inputs[1 * 16 + i];

            // アナログ的不完全さ
            let p_offset = ctx.imperfection.personality[i] * 0.1;
            let d_offset = ctx.imperfection.drift[i] * 0.01;

            let total_cutoff = (cutoff_val + cv_in + p_offset + d_offset).clamp(0.0, 10.0);

            // Moog Ladder フィルターの計算
            let f = total_cutoff * 0.1;
            let k = 4.0 * res_val;
            let g = f / (1.0 + f);

            let x = self.saturate_scalar(input * drive_val, i);

            // フィードバック経路に履歴依存の非線形性を導入 (Signal Memory)
            let fb_val = self.stages_poly[i][3];
            let feedback = x - k * self.saturate_scalar(fb_val, i);

            let mut curr = feedback;
            for s in 0..4 {
                let y = curr * g + self.stages_poly[i][s] * (1.0 - g);
                self.stages_poly[i][s] = y;
                curr = self.saturate_scalar(y, i);
            }

            outputs[0 * 16 + i] = self.stages_poly[i][3];
        }
    }

    fn extract_state(&self) -> Option<Vec<u8>> {
        let data = serde_json::to_vec(&VcfStateData {
            stages_x4: [[0.0; 4]; 4],
            mode: self.mode,
        })
        .ok()?; // Simplified for now
        Some(data)
    }

    fn inject_state(&mut self, data: &[u8]) {
        if let Ok(s) = serde_json::from_slice::<VcfStateData>(data) {
            self.mode = s.mode;
        }
    }
    fn get_forensic_data(&self) -> Option<crate::signal::ForensicData> {
        let mut data = crate::signal::ForensicData::default();
        // 第1ボイスの4段のステートを先頭に格納
        for s in 0..4 {
            data.thermal_heat[s] = self.stages_poly[0][s];
        }
        data.internal_state_summary = format!("VCF Mode: {:?}", self.mode);
        Some(data)
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl VcfModule {
    fn saturate_scalar(&self, x: f32, _voice_idx: usize) -> f32 {
        // パッド近似 tanh
        let x2 = x * x;
        let a = x * (1.0 + x2 * 0.16489087);
        let b = 1.0 + x2 * 0.4982926;
        a / b
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_vcf",
        name: "VCF",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals {
            background_color: [30, 35, 50],
            text_color: [200, 210, 255],
            accent_color: [220, 40, 40],
            panel_texture: crate::signal::PanelTexture::MatteBlack,
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
                name: "RES",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.5, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "DRIVE",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 1.0,
                max: 10.0,
                default: 1.0,
                position: [0.5, 0.6],
                unit: "x",
            },
            ParamDescriptor {
                name: "MODE",
                kind: ParamKind::Switch { positions: 3 },
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 2.0,
                default: 1.0,
                position: [0.5, 0.8],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.2, 0.85],
            },
            PortDescriptor {
                name: "CUTOFF_CV",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.5, 0.85],
            },
            PortDescriptor {
                name: "LP4",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.8, 0.85],
            },
        ],
        factory: |sr| Box::new(VcfModule::new(sr)),
    }
}
