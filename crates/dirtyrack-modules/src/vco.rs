//! VCO Module — Polyphonic Oscillator with SIMD optimization
//!
//! # 憲法遵守
//! - 入力がポリフォニック（16ch）の場合、全チャンネルを並列またはループで処理。
//! - 1V/Oct 入力を各ボイスの周波数に変換。

use dirtyrack_sdk::*;
use crate::signal::{voct_to_hz, SmoothedParam, TriggerDetector};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
struct VcoVoiceState {
    phase: f32,
}

#[dirty_module]
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
        let freq_knob = params.get(0).copied().unwrap_or(0.0);
        let fine = params.get(1).copied().unwrap_or(0.0);
        let fm_amt = params.get(2).copied().unwrap_or(0.0);
        let pw_knob = params.get(3).copied().unwrap_or(0.0);
        let level = params.get(4).copied().unwrap_or(0.2);

        self.freq_smooth.set(freq_knob);
        self.pw_smooth.set(pw_knob);
        let jitter = _ctx.imperfection.drift.get(0).copied().unwrap_or(0.0);
        let freq_val = self.freq_smooth.next(jitter);
        let pw_val = self.pw_smooth.next(jitter);

        // Scale detuning by aging
        let p_mult = 0.0001 + _ctx.aging * 0.05;

        for i in 0..16 {
            let voct_in = inputs.get(0 * 16 + i).copied().unwrap_or(0.0);
            let fm_in = inputs.get(1 * 16 + i).copied().unwrap_or(0.0);
            let pw_cv_in = inputs.get(2 * 16 + i).copied().unwrap_or(0.0);
            let sync_in = inputs.get(3 * 16 + i).copied().unwrap_or(0.0);

            let p_offset = _ctx.imperfection.personality.get(i).copied().unwrap_or(0.0) * p_mult;
            
            // Apply drift and aging using the new SDK trait
            let total_voltage = (freq_val + voct_in + fine + p_offset)
                .apply_drift(i, _ctx)
                .apply_aging(i, _ctx);

            let freq_hz = voct_to_hz(total_voltage + fm_in * fm_amt);
            let pw = (pw_val + pw_cv_in * 0.1).clamp(0.01, 0.99);

            if self.sync_detectors[i].process(sync_in) {
                self.phases[i] = 0.0;
            }

            let dt = freq_hz / self.sample_rate;
            self.phases[i] = (self.phases[i] + dt).fract();

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

            let gain = 5.0 * level;
            // SINE
            outputs[0 * 16 + i] = libm::sinf(self.phases[i] * 2.0 * std::f32::consts::PI) * gain;
            // SAW
            outputs[1 * 16 + i] =
                (self.phases[i] * 2.0 - 1.0 - polyblep(self.phases[i], dt)) * gain;
            // TRI
            let tri = (self.phases[i] * 2.0 - 1.0).abs() * 2.0 - 1.0;
            outputs[2 * 16 + i] = tri * gain;
            // SQUARE
            let mut sq = if self.phases[i] < pw { 1.0 } else { -1.0 };
            sq += polyblep(self.phases[i], dt);
            sq -= polyblep((self.phases[i] + (1.0 - pw)).fract(), dt);
            outputs[3 * 16 + i] = sq * gain;
        }
    }
    fn get_forensic_data(&self) -> Option<ForensicData> {
        let mut data = ForensicData::default();
        data.thermal_heat = self.heat;
        data.internal_state_summary = format!(
            "VCO Active Voices: {}",
            self.phases.iter().filter(|&&p| p > 0.0).count()
        );
        Some(data)
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self.as_any_mut_impl()
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
            ParamDescriptor {
                name: "LEVEL",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 },
                min: 0.0,
                max: 1.0,
                default: 0.2,
                position: [0.5, 0.4],
                unit: "V",
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
