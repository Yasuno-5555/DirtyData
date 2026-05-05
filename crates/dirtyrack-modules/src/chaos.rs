//! Chaos Module — Lorenz Attractor CV Generator
//!
//! # 憲法遵守
//! - 常微分方程式のオイラー法による決定論的解法。
//! - X, Y, Z の3軸をCVとして出力。
//! - 時間とともに「生き物のように」変動する非周期的な信号。

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SignalType,
};

pub struct ChaosModule {
    x: [f32; 16],
    y: [f32; 16],
    z: [f32; 16],
    dt: f32,
}

impl ChaosModule {
    pub fn new(_sr: f32) -> Self {
        Self {
            x: std::array::from_fn(|i| 0.1 + i as f32 * 0.001),
            y: [0.0; 16],
            z: [0.0; 16],
            dt: 0.001, // 固定ステップで決定論を維持
        }
    }
}

impl RackDspNode for ChaosModule {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let sigma = params[0]; // 10.0
        let rho = params[1]; // 28.0
        let beta = params[2]; // 8.0/3.0
        let speed = params[3]; // 1.0

        for v in 0..16 {
            let dx = sigma * (self.y[v] - self.x[v]);
            let dy = self.x[v] * (rho - self.z[v]) - self.y[v];
            let dz = self.x[v] * self.y[v] - beta * self.z[v];

            self.x[v] += dx * self.dt * speed;
            self.y[v] += dy * self.dt * speed;
            self.z[v] += dz * self.dt * speed;

            // Scaling to Eurorack levels (+/- 5V)
            outputs[0 * 16 + v] = self.x[v] * 0.2;
            outputs[1 * 16 + v] = self.y[v] * 0.2;
            outputs[2 * 16 + v] = (self.z[v] - 25.0) * 0.2;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> BuiltinModuleDescriptor {
    BuiltinModuleDescriptor {
        id: "dirty_chaos_lorenz",
        name: "Lorenz",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[
            ParamDescriptor {
                name: "SIGMA",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 20.0,
                default: 10.0,
                position: [0.5, 0.2],
                unit: "",
            },
            ParamDescriptor {
                name: "RHO",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 50.0,
                default: 28.0,
                position: [0.5, 0.4],
                unit: "",
            },
            ParamDescriptor {
                name: "BETA",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 5.0,
                default: 2.66,
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
        ports: &[
            PortDescriptor {
                name: "OUT X",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.2, 0.9],
            },
            PortDescriptor {
                name: "OUT Y",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.5, 0.9],
            },
            PortDescriptor {
                name: "OUT Z",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.8, 0.9],
            },
        ],
        factory: |sr| Box::new(ChaosModule::new(sr)),
    }
}
