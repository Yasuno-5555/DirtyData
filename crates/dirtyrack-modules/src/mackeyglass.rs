//! Mackey-Glass Module — Delay Differential Equation Chaos
//!
//! # 憲法遵守
//! - `x'(t) = a * x(t-tau) / (1 + x(t-tau)^n) - b * x(t)`
//! - 履歴バッファを使用して遅延項を計算。
//! - パラメータ tau によってカオスの複雑さが劇的に変化。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SignalType,
};

pub struct MackeyGlassModule {
    history: [Vec<f32>; 16],
    write_pos: [usize; 16],
    x: [f32; 16],
    dt: f32,
}

impl MackeyGlassModule {
    pub fn new(sample_rate: f32) -> Self {
        let max_tau_samples = (sample_rate * 0.1) as usize; // 100ms max tau
        Self {
            history: std::array::from_fn(|_| vec![0.5; max_tau_samples]),
            write_pos: [0; 16],
            x: [0.5; 16],
            dt: 0.1, // Fixed step for stability
        }
    }
}

impl RackDspNode for MackeyGlassModule {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let a = params[0]; // 0.2
        let b = params[1]; // 0.1
        let tau = params[2]; // 10 .. 100
        let n = 10.0;
        let speed = params[3];

        for v in 0..16 {
            let history_len = self.history[v].len();
            let tau_idx = (self.write_pos[v] + history_len - (tau as usize).min(history_len - 1))
                % history_len;
            let x_tau = self.history[v][tau_idx];

            let dx = (a * x_tau) / (1.0 + libm::powf(x_tau, n)) - b * self.x[v];
            self.x[v] += dx * self.dt * speed;

            self.history[v][self.write_pos[v]] = self.x[v];
            self.write_pos[v] = (self.write_pos[v] + 1) % history_len;

            outputs[0 * 16 + v] = (self.x[v] - 0.8) * 5.0; // Scaled to Eurorack
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_chaos_mg",
        name: "MackeyGlass",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[
            ParamDescriptor {
                name: "A",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.1,
                max: 0.5,
                default: 0.2,
                position: [0.5, 0.2],
                unit: "",
            },
            ParamDescriptor {
                name: "B",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.05,
                max: 0.2,
                default: 0.1,
                position: [0.5, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "TAU",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 10.0,
                max: 1000.0,
                default: 200.0,
                position: [0.5, 0.6],
                unit: "",
            },
            ParamDescriptor {
                name: "SPEED",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 1.0,
                position: [0.5, 0.8],
                unit: "x",
            },
        ],
        ports: &[PortDescriptor {
            name: "OUT",
            direction: PortDirection::Output,
            signal_type: SignalType::BiCV,
            max_channels: 1,
            position: [0.5, 0.95],
        }],
        factory: |sr| Box::new(MackeyGlassModule::new(sr)),
    }
}
