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
    history: Vec<f32>,
    write_pos: usize,
    x: f32,
    dt: f32,
}

impl MackeyGlassModule {
    pub fn new(sample_rate: f32) -> Self {
        let max_tau_samples = (sample_rate * 0.1) as usize; // 100ms max tau
        Self {
            history: vec![0.5; max_tau_samples],
            write_pos: 0,
            x: 0.5,
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
        let tau = params[2]; // 10 .. 100 (in samples equivalent for simplicity)
        let n = 10.0;
        let speed = params[3];

        let tau_idx = (self.write_pos + self.history.len()
            - (tau as usize).min(self.history.len() - 1))
            % self.history.len();
        let x_tau = self.history[tau_idx];

        let dx = (a * x_tau) / (1.0 + libm::powf(x_tau, n)) - b * self.x;
        self.x += dx * self.dt * speed;

        self.history[self.write_pos] = self.x;
        self.write_pos = (self.write_pos + 1) % self.history.len();

        outputs[0] = (self.x - 0.8) * 5.0; // Scaled to Eurorack
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
