//! Audio Input Module — 外部世界からの入口。
//! オーディオインターフェースからの入力をラック内に引き込む。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};

pub struct InputModule {
    // 外部から注入される最新の入力サンプル
    pub external_in_l: f32,
    pub external_in_r: f32,
}

impl InputModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self {
            external_in_l: 0.0,
            external_in_r: 0.0,
        }
    }
}

impl RackDspNode for InputModule {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let gain = params[0];
        // 16ch対応: LはPort 0、RはPort 1。全ボイスに同じ信号を分配
        for v in 0..16 {
            outputs[0 * 16 + v] = self.external_in_l * gain;
            outputs[1 * 16 + v] = self.external_in_r * gain;
        }
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_input",
        name: "AUDIO IN",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 6,
        visuals: crate::signal::ModuleVisuals::default(),
        tags: &["Builtin"],
        params: &[ParamDescriptor {
            name: "GAIN",
            kind: ParamKind::Knob,
            response: ParamResponse::Smoothed { ms: 20.0 },
            min: 0.0,
            max: 2.0,
            default: 1.0,
            position: [0.5, 0.4],
            unit: "x",
        }],
        ports: &[
            PortDescriptor {
                name: "LEFT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.3, 0.8],
            },
            PortDescriptor {
                name: "RIGHT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 1,
                position: [0.7, 0.8],
            },
        ],
        factory: |sr| Box::new(InputModule::new(sr)),
    }
}
