//! VCO Module — Polyphonic Oscillator with SIMD optimization
//!
//! # 憲法遵守
//! - 入力がポリフォニック（16ch）の場合、全チャンネルを並列またはループで処理。
//! - 1V/Oct 入力を各ボイスの周波数に変換。

use crate::signal::{
    voct_to_hz, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection,
    RackDspNode, RackProcessContext, SignalType, SmoothedParam, TriggerDetector,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
struct VcoVoiceState {
    phase: f32,
}

pub struct VcoModule {
    phases: [f32; 16],
    sample_rate: f32,
    sync_detectors: [TriggerDetector; 16],
    freq_smooth: SmoothedParam,
    pw_smooth: SmoothedParam,
    heat: [f32; 16],
}

impl VcoModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phases: [0.0; 16],
            sample_rate,
            sync_detectors: [TriggerDetector::new(); 16],
            freq_smooth: SmoothedParam::new(5.0, sample_rate, 10.0),
            pw_smooth: SmoothedParam::new(0.5, sample_rate, 10.0),
            heat: [0.0; 16],
        }
    }
}

impl RackDspNode for VcoModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let freq_knob = params[0];
        let fine = params[1];
        let fm_amt = params[2];
        let pw_knob = params[3];

        self.freq_smooth.set(freq_knob);
        self.pw_smooth.set(pw_knob);
        let jitter = _ctx.imperfection.drift[0];
        let freq_val = self.freq_smooth.next(jitter);
        let pw_val = self.pw_smooth.next(jitter);

        for i in 0..16 {
            let voct_in = inputs[0 * 16 + i];
            let fm_in = inputs[1 * 16 + i];
            let pw_cv_in = inputs[2 * 16 + i];
            let sync_in = inputs[3 * 16 + i];

            let p_offset = _ctx.imperfection.personality[i] * 0.05;
            let d_offset = _ctx.imperfection.drift[i] * 0.005;

            // 熱ドリフト (Signal Memory)
            // 高周波数ほど熱を持ち、ピッチがわずかに下がる (物理的な熱膨張的な挙動)
            let h_offset = self.heat[i] * -0.002;
            let freq_hz_pre =
                voct_to_hz(freq_val + voct_in + fine + p_offset + d_offset + h_offset);
            self.heat[i] += (freq_hz_pre / 1000.0) * 0.000001; // 蓄熱
            self.heat[i] *= 0.99999; // 放熱

            let pitch_voltage = freq_val + voct_in + fine + p_offset + d_offset + h_offset;
            let total_voltage = pitch_voltage + fm_in * fm_amt;
            let freq_hz = voct_to_hz(total_voltage);
            let pw = (pw_val + pw_cv_in * 0.1).clamp(0.01, 0.99);

            if self.sync_detectors[i].process(sync_in) {
                self.phases[i] = 0.0;
            }

            let dt = freq_hz / self.sample_rate;
            self.phases[i] = (self.phases[i] + dt).fract();

            // Simplified oscillator for polyphonic context (PolyBLEP is expensive but here we use simple for now)
            let polyblep = |t: f32, dt: f32| -> f32 {
                if t < dt {
                    let t = t / dt;
                    t + t - t * t - 1.0
                } else if t > 1.0 - dt {
                    let t = (t - 1.0) / dt;
                    t * t + t + t + 1.0
                } else {
                    0.0
                }
            };

            // Outputs are also polyphonic (16 slots per port)
            // SINE
            outputs[0 * 16 + i] = libm::sinf(self.phases[i] * 2.0 * std::f32::consts::PI) * 5.0;
            // SAW
            outputs[1 * 16 + i] = (self.phases[i] * 2.0 - 1.0 - polyblep(self.phases[i], dt)) * 5.0;
            // TRI
            let tri = (self.phases[i] * 2.0 - 1.0).abs() * 2.0 - 1.0;
            outputs[2 * 16 + i] = tri * 5.0;
            // SQUARE
            let mut sq = if self.phases[i] < pw { 1.0 } else { -1.0 };
            sq += polyblep(self.phases[i], dt);
            sq -= polyblep((self.phases[i] + (1.0 - pw)).fract(), dt);
            outputs[3 * 16 + i] = sq * 5.0;
        }
    }
    fn get_forensic_data(&self) -> Option<crate::signal::ForensicData> {
        let mut data = crate::signal::ForensicData::default();
        data.thermal_heat = self.heat;
        data.internal_state_summary = format!(
            "VCO Active Voices: {}",
            self.phases.iter().filter(|&&p| p > 0.0).count()
        );
        Some(data)
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_vco",
        name: "VCO",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 10,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin", "OSC", "VCO"],
        params: &[
            ParamDescriptor {
                name: "FREQ",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 10.0,
                default: 5.0,
                position: [0.3, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "FINE",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: -0.1,
                max: 0.1,
                default: 0.0,
                position: [0.7, 0.2],
                unit: "V",
            },
            ParamDescriptor {
                name: "FM_AMT",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.0,
                position: [0.3, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "PW",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.01,
                max: 0.99,
                default: 0.5,
                position: [0.7, 0.4],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "V/OCT",
                direction: PortDirection::Input,
                signal_type: SignalType::VoltPerOct,
                max_channels: 16,
                position: [0.2, 0.7],
            },
            PortDescriptor {
                name: "FM",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 16,
                position: [0.4, 0.7],
            },
            PortDescriptor {
                name: "PW_CV",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 16,
                position: [0.6, 0.7],
            },
            PortDescriptor {
                name: "SYNC",
                direction: PortDirection::Input,
                signal_type: SignalType::Trigger,
                max_channels: 16,
                position: [0.8, 0.7],
            },
            PortDescriptor {
                name: "SINE",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.2, 0.9],
            },
            PortDescriptor {
                name: "SAW",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.4, 0.9],
            },
            PortDescriptor {
                name: "TRI",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.6, 0.9],
            },
            PortDescriptor {
                name: "SQUARE",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.8, 0.9],
            },
        ],
        factory: |sr| Box::new(VcoModule::new(sr)),
    }
}
