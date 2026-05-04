//! Audio Engine — NO-ALLOC / NO-LOCK / STATE-PRESERVING
//!
//! Phase 4: Triple-Buffer による視覚的投影と、
//! crossbeam-channel によるトポロジー更新を実装。

use crate::visual_data::{ModuleVisualState, VisualSnapshot};
use arc_swap::ArcSwap;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{unbounded, Sender};
use dirtyrack_modules::runner::{GraphSnapshot, RackRunner};
use dirtyrack_modules::{RackDspNode, SeedScope};
use std::sync::Arc;
use triple_buffer::Output;

pub enum AudioEvent {
    TopologyChanged,
}

pub struct TopologyUpdate {
    pub snapshot: GraphSnapshot,
    pub nodes: Vec<Box<dyn RackDspNode>>,
    pub params: Vec<Vec<f32>>,
}

pub struct ParamChange {
    pub stable_id: u64,
    pub params: Vec<f32>,
}

pub struct RackAudioEngine {
    #[allow(dead_code)]
    params: Arc<ArcSwap<Vec<Vec<f32>>>>,
    topology_tx: Sender<TopologyUpdate>,
    param_tx: Sender<ParamChange>,
    aging_tx: Sender<f32>,
    _stream: cpal::Stream,
}

impl RackAudioEngine {
    pub fn new(sample_rate: f32) -> Result<(Self, Output<VisualSnapshot>), String> {
        let params = Arc::new(ArcSwap::from_pointee(Vec::new()));
        let (topo_tx, topo_rx) = unbounded::<TopologyUpdate>();
        let (param_tx, param_rx) = unbounded::<ParamChange>();
        let (aging_tx, aging_rx) = unbounded::<f32>();

        let (mut visual_in, visual_out) = triple_buffer::triple_buffer(&VisualSnapshot::new());

        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("No output device")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;

        let mut runner = RackRunner::new(sample_rate, SeedScope::Global(0xDE7E_B11D));
        let mut current_snapshot = GraphSnapshot {
            order: Vec::new(),
            connections: Vec::new(),
            port_counts: Vec::new(),
            node_ids: Vec::new(),
            node_type_ids: Vec::new(),
            modulations: Vec::new(),
            forward_edges: Vec::new(),
            back_edges: Vec::new(),
        };
        let mut current_params = Vec::new();

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // 1. Check for topology updates
                    while let Ok(update) = topo_rx.try_recv() {
                        current_snapshot = update.snapshot;
                        current_params = update.params;
                        runner.apply_snapshot(&mut current_snapshot, update.nodes);
                    }

                    // 1.5. Check for parameter updates
                    while let Ok(change) = param_rx.try_recv() {
                        if let Some(idx) = current_snapshot.node_ids.iter().position(|&id| id == change.stable_id) {
                            if idx < current_params.len() {
                                current_params[idx] = change.params;
                            }
                        }
                    }

                    // 2. Check for aging updates
                    while let Ok(new_aging) = aging_rx.try_recv() {
                        runner.ctx.aging = new_aging;
                    }


                    // Find Audio Out module index
                    let output_node_idx = current_snapshot
                        .node_type_ids
                        .iter()
                        .position(|id| id == "dirty_output");

                    for frame in data.chunks_mut(2) {
                        runner.process_sample(&current_snapshot, &current_params);

                        let (left, right);
                        if let Some(idx) = output_node_idx {
                            left = runner.get_output(idx, 0);
                            right = runner.get_output(idx, 1);
                        } else {
                            if let Some(&last_node) = current_snapshot.order.last() {
                                left = runner.get_output(last_node, 0);
                                right = left;
                            } else {
                                left = 0.0;
                                right = 0.0;
                            }
                        }

                        let master_gain = 0.3;
                        frame[0] = (left * master_gain).clamp(-1.0, 1.0);
                        frame[1] = (right * master_gain).clamp(-1.0, 1.0);
                    }

                    // 3. 鑑識データの収集 (バッファごとに1回)
                    let mut visual_snapshot = VisualSnapshot::default();
                    for (i, &stable_id) in current_snapshot.node_ids.iter().enumerate() {
                        let mut state = ModuleVisualState::default();
                        if let Some(node) = runner.active_nodes.get(i) {
                            state.forensic = node.get_forensic_data();
                            if let Some(f) = &mut state.forensic {
                                f.personality_offsets = runner.node_personalities[i]; 
                                f.current_drift = runner.drift_engine.current_drift();
                                f.stats = runner.stats[i];
                            }
                        }
                        for p_idx in 0..current_snapshot.port_counts[i].1 {
                            state.outputs.push(runner.get_output(i, p_idx));
                        }
                        visual_snapshot.modules.insert(stable_id, state);
                    }
                    visual_in.write(visual_snapshot);
                },
                |err| eprintln!("Audio error: {}", err),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        let engine = Self {
            params,
            topology_tx: topo_tx,
            param_tx,
            aging_tx,
            _stream: stream,
        };

        Ok((engine, visual_out))
    }

    pub fn sync_aging(&self, aging: f32) -> Result<(), String> {
        self.aging_tx.send(aging).map_err(|e| e.to_string())
    }

    pub fn update_topology(&self, snapshot: GraphSnapshot, nodes: Vec<Box<dyn RackDspNode>>, params: Vec<Vec<f32>>) {
        // Wait-free Topology Update: We send the new nodes and snapshot to the audio thread
        // where it will be swapped safely.
        let _ = self.topology_tx.send(TopologyUpdate { snapshot, nodes, params });
    }

    pub fn update_module_parameters(&self, stable_id: u64, params: Vec<f32>) {
        let _ = self.param_tx.send(ParamChange { stable_id, params });
    }
}
