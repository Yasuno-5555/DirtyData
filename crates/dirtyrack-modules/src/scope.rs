//! Forensic Scope Module — 鑑識用オシロスコープ
//! 
//! 16ボイスすべての信号を個別に、または重ねて表示し、
//! ボイスごとの不完全さ（ドリフト等）による挙動の差異を視覚化する。

use crate::signal::{
    ForensicData, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection,
    RackDspNode, RackProcessContext, SignalType,
};
use std::collections::VecDeque;

pub struct ScopeModule {
    history: [VecDeque<f32>; 16],
    max_len: usize,
}

impl ScopeModule {
    pub fn new() -> Self {
        let mut history: [VecDeque<f32>; 16] = std::array::from_fn(|_| VecDeque::with_capacity(512));
        Self {
            history,
            max_len: 512,
        }
    }
}

impl RackDspNode for ScopeModule {
    fn process(
        &mut self,
        inputs: &[f32],
        _outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        for v in 0..16 {
            let val = inputs[v]; // Assuming 16-channel mono input at port 0
            if self.history[v].len() >= self.max_len {
                self.history[v].pop_front();
            }
            self.history[v].push_back(val);
        }
    }

    fn get_forensic_data(&self) -> Option<ForensicData> {
        let mut trace = Vec::with_capacity(self.max_len);
        for i in 0..self.max_len {
            let mut sample = [0.0; 16];
            for v in 0..16 {
                sample[v] = *self.history[v].get(i).unwrap_or(&0.0);
            }
            trace.push(sample);
        }
        
        let mut data = ForensicData::default();
        data.internal_state_summary = format!("Buffer: {} samples", self.max_len);
        data.signal_trace = Some(trace);
        Some(data)
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_scope",
        name: "FORENSIC SCOPE",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals {
            background_color: [20, 20, 25],
            text_color: [0, 200, 255],
            accent_color: [0, 150, 255],
            panel_texture: crate::signal::PanelTexture::MatteBlack,
        },
        tags: &["UTL", "VIS"],
        params: &[],
        ports: &[PortDescriptor {
            name: "IN",
            direction: PortDirection::Input,
            signal_type: SignalType::Audio,
            max_channels: 16,
            position: [0.5, 0.8],
        }],
        factory: |_| Box::new(ScopeModule::new()),
    }
}
