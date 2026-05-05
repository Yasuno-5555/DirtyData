//! Circuit Module — Sandbox for MNA-based circuit simulation
//!
//! ユーザーが定義した電子回路（抵抗、コンデンサ、真空管等）を
//! DirtyRack のモジュールとして実行。

use crate::signal::{PortDescriptor, PortDirection, RackDspNode, RackProcessContext, SignalType};
use dirtydata_dsp_circuit::{CircuitElement, MnaSolver, NodeId};
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
    solvers: [MnaSolver; 16],
    definition: Arc<CircuitDefinition>,
    sample_rate: f32,
}

impl CircuitModule {
    pub fn new(sample_rate: f32) -> Self {
        let dt = 1.0 / sample_rate as f64;
        let mut solvers: [MnaSolver; 16] = std::array::from_fn(|_| MnaSolver::new(dt));

        // デフォルト回路: 単純な分圧器 (Test用)
        for solver in &mut solvers {
            solver.set_num_nodes(3);
            solver.add_element(CircuitElement::VoltageSource {
                pos: NodeId(1),
                neg: NodeId(0),
                voltage: 0.0,
            }); // IN
            solver.add_element(CircuitElement::Resistor {
                a: NodeId(1),
                b: NodeId(2),
                value: 1000.0,
                tolerance: 0.0,
                material: dirtydata_dsp_circuit::Material::MetalFilm,
            });
            solver.add_element(CircuitElement::Resistor {
                a: NodeId(2),
                b: NodeId(0),
                value: 1000.0,
                tolerance: 0.0,
                material: dirtydata_dsp_circuit::Material::MetalFilm,
            });
        }

        let def = Arc::new(CircuitDefinition {
            elements: solvers[0].elements.clone(),
            num_nodes: 3,
            input_mappings: vec![1],  // Node 1 is Input
            output_mappings: vec![2], // Node 2 is Output
        });

        Self {
            solvers,
            definition: def,
            sample_rate,
        }
    }

    pub fn update_definition(&mut self, def: CircuitDefinition) {
        let dt = 1.0 / self.sample_rate as f64;
        for solver in &mut self.solvers {
            let mut new_solver = MnaSolver::new(dt);
            new_solver.set_num_nodes(def.num_nodes);
            for el in &def.elements {
                new_solver.add_element(el.clone());
            }
            *solver = new_solver;
        }
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
        for v in 0..16 {
            let solver = &mut self.solvers[v];

            // 1. Set inputs from rack ports to voltage sources
            for (i, &node_idx) in self.definition.input_mappings.iter().enumerate() {
                if i >= 4 {
                    break;
                }
                let val = inputs[i * 16 + v] as f64;

                if let Some(el) = solver.elements.iter_mut().find(|el| match el {
                    CircuitElement::VoltageSource { pos, .. } => pos.0 == node_idx,
                    _ => false,
                }) {
                    if let CircuitElement::VoltageSource { voltage, .. } = el {
                        *voltage = val;
                    }
                }
            }

            // 2. Solve the circuit
            let state = solver.solve();

            // 3. Extract voltages to rack output ports
            for (i, &node_idx) in self.definition.output_mappings.iter().enumerate() {
                if i >= 4 {
                    break;
                }
                let val = state.voltages.get(node_idx).copied().unwrap_or(0.0);
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
            panel_texture: crate::signal::PanelTexture::MatteBlack, knob_style: crate::signal::KnobStyle::ClassicSilver,
        },
        tags: &["Builtin", "Circuit", "Simulation", "Sandbox"],
        params: &[], // エディタ側で制御するため、固定パラメータはなし
        ports: &[
            PortDescriptor {
                name: "IN 1",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.1, 0.8],
            },
            PortDescriptor {
                name: "IN 2",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.3, 0.8],
            },
            PortDescriptor {
                name: "IN 3",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.8],
            },
            PortDescriptor {
                name: "IN 4",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.7, 0.8],
            },
            PortDescriptor {
                name: "OUT 1",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.1, 0.9],
            },
            PortDescriptor {
                name: "OUT 2",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.3, 0.9],
            },
            PortDescriptor {
                name: "OUT 3",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.9],
            },
            PortDescriptor {
                name: "OUT 4",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.7, 0.9],
            },
        ],
        factory: |sr| Box::new(CircuitModule::new(sr)),
    }
}
