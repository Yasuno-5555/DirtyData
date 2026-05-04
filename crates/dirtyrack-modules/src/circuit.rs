//! Circuit Module — Sandbox for MNA-based circuit simulation
//!
//! ユーザーが定義した電子回路（抵抗、コンデンサ、真空管等）を
//! DirtyRack のモジュールとして実行。

use crate::signal::{
    PortDescriptor, PortDirection, RackDspNode, RackProcessContext, SignalType,
};
use dirtydata_dsp_circuit::{MnaSolver, CircuitElement, NodeId, CircuitState};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CircuitDefinition {
    pub elements: Vec<CircuitElement>,
    pub num_nodes: usize,
    /// どのノードをラックのポートに接続するか
    /// (PortIndex -> NodeId)
    pub input_mappings: Vec<usize>, 
    pub output_mappings: Vec<usize>,
}

pub struct CircuitModule {
    solver: MnaSolver,
    definition: Arc<CircuitDefinition>,
    sample_rate: f32,
    last_state: CircuitState,
}

impl CircuitModule {
    pub fn new(sample_rate: f32) -> Self {
        let mut solver = MnaSolver::new(1.0 / sample_rate as f64);
        
        // デフォルト回路: 単純な分圧器 (Test用)
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::VoltageSource { pos: NodeId(1), neg: NodeId(0), voltage: 0.0 }); // IN
        solver.add_element(CircuitElement::Resistor { a: NodeId(1), b: NodeId(2), value: 1000.0, tolerance: 0.0, material: dirtydata_dsp_circuit::Material::MetalFilm });
        solver.add_element(CircuitElement::Resistor { a: NodeId(2), b: NodeId(0), value: 1000.0, tolerance: 0.0, material: dirtydata_dsp_circuit::Material::MetalFilm });

        let def = Arc::new(CircuitDefinition {
            elements: solver.elements.clone(),
            num_nodes: 3,
            input_mappings: vec![1], // Node 1 is Input
            output_mappings: vec![2], // Node 2 is Output
        });

        let last_state = solver.solve();

        Self {
            solver,
            definition: def,
            sample_rate,
            last_state,
        }
    }

    pub fn update_definition(&mut self, def: CircuitDefinition) {
        let mut solver = MnaSolver::new(1.0 / self.sample_rate as f64);
        solver.set_num_nodes(def.num_nodes);
        for el in &def.elements {
            solver.add_element(el.clone());
        }
        self.solver = solver;
        self.definition = Arc::new(def);
    }
}

impl RackDspNode for CircuitModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        _params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        // 現在はモノフォニック実装 (ボイス0のみを使用)
        // 回路内の電圧源(VoltageSource)を探して、入力を流し込む
        for (i, &node_idx) in self.definition.input_mappings.iter().enumerate() {
            if i >= 4 { break; } // 最大4入力
            let val = inputs[i * 16] as f64;
            
            // 該当する電圧源を検索
            if let Some(el) = self.solver.elements.iter_mut().find(|el| {
                if let CircuitElement::VoltageSource { pos, .. } = el {
                    pos.0 == node_idx
                } else {
                    false
                }
            }) {
                if let CircuitElement::VoltageSource { voltage, .. } = el {
                    *voltage = val;
                }
            }
        }

        // シミュレーション実行
        self.last_state = self.solver.solve();

        // 出力ノードの電位を取得
        for (i, &node_idx) in self.definition.output_mappings.iter().enumerate() {
            if i >= 4 { break; } // 最大4出力
            let val = self.last_state.voltages.get(node_idx).copied().unwrap_or(0.0);
            for v in 0..16 {
                outputs[i * 16 + v] = val as f32;
            }
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn extract_state(&self) -> Option<Vec<u8>> {
        serde_json::to_vec(&*self.definition).ok()
    }
    
    fn inject_state(&mut self, state: &[u8]) {
        if let Ok(def) = serde_json::from_slice::<CircuitDefinition>(state) {
            self.update_definition(def);
        }
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_circuit",
        name: "Circuit Sandbox",
        version: "0.1.0",
        manufacturer: "DirtyRack",
        hp_width: 12,
        visuals: crate::signal::ModuleVisuals {
            background_color: [40, 45, 50],
            accent_color: [0, 255, 150],
            text_color: [200, 220, 200],
            panel_texture: crate::signal::PanelTexture::MatteBlack,
        },
        tags: &["Builtin", "Circuit", "Simulation", "Sandbox"],
        params: &[], // エディタ側で制御するため、固定パラメータはなし
        ports: &[
            PortDescriptor { name: "IN 1", direction: PortDirection::Input, signal_type: SignalType::Audio, max_channels: 16, position: [0.1, 0.8] },
            PortDescriptor { name: "IN 2", direction: PortDirection::Input, signal_type: SignalType::Audio, max_channels: 16, position: [0.3, 0.8] },
            PortDescriptor { name: "IN 3", direction: PortDirection::Input, signal_type: SignalType::Audio, max_channels: 16, position: [0.5, 0.8] },
            PortDescriptor { name: "IN 4", direction: PortDirection::Input, signal_type: SignalType::Audio, max_channels: 16, position: [0.7, 0.8] },
            PortDescriptor { name: "OUT 1", direction: PortDirection::Output, signal_type: SignalType::Audio, max_channels: 16, position: [0.1, 0.9] },
            PortDescriptor { name: "OUT 2", direction: PortDirection::Output, signal_type: SignalType::Audio, max_channels: 16, position: [0.3, 0.9] },
            PortDescriptor { name: "OUT 3", direction: PortDirection::Output, signal_type: SignalType::Audio, max_channels: 16, position: [0.5, 0.9] },
            PortDescriptor { name: "OUT 4", direction: PortDirection::Output, signal_type: SignalType::Audio, max_channels: 16, position: [0.7, 0.9] },
        ],
        factory: |sr| Box::new(CircuitModule::new(sr)),
    }
}
