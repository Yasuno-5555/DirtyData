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
    pub fn new(_sample_rate: f32) -> Result<(Self, Output<VisualSnapshot>, f32), String> {
        let params = Arc::new(ArcSwap::from_pointee(Vec::new()));
        let (topo_tx, topo_rx) = unbounded::<TopologyUpdate>();
        let (param_tx, param_rx) = unbounded::<ParamChange>();
        let (aging_tx, aging_rx) = unbounded::<f32>();

        let (mut visual_in, visual_out) = triple_buffer::triple_buffer(&VisualSnapshot::new());

        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("No output device")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;

        let hardware_sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;

        // Request Low Latency Buffer (e.g. 128 or 256 samples)
        let mut stream_config: cpal::StreamConfig = config.into();
        stream_config.buffer_size = cpal::BufferSize::Fixed(256);

        let mut runner = RackRunner::new(hardware_sample_rate, SeedScope::Global(0xDE7E_B11D));
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
        let mut visual_throttle = 0;

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // 1. Check for topology updates (DRAIN QUEUE: Only apply the LATEST one)
                    let mut latest_update = None;
                    while let Ok(update) = topo_rx.try_recv() {
                        latest_update = Some(update);
                    }
                    if let Some(update) = latest_update {
                        current_snapshot = update.snapshot;
                        current_params = update.params;
                        runner.apply_snapshot(&mut current_snapshot, update.nodes);
                        // No data.fill(0.0) here to avoid clicks and redundant work
                    }

                    // 1.5. Check for parameter updates
                    while let Ok(change) = param_rx.try_recv() {
                        if let Some(idx) = current_snapshot
                            .node_ids
                            .iter()
                            .position(|&id| id == change.stable_id)
                        {
                            if idx < current_params.len() {
                                current_params[idx] = change.params;
                            }
                        }
                    }

                    // 2. Check for aging updates
                    while let Ok(new_aging) = aging_rx.try_recv() {
                        runner.ctx.aging = new_aging;
                    }

                    // Find Audio Out module index (using order-safe indexing)
                    let output_module_idx = current_snapshot
                        .node_type_ids
                        .iter()
                        .position(|id| id == "dirty_output");

                    for frame in data.chunks_mut(channels) {
                        runner.process_sample(&current_snapshot, &current_params);

                        let mut left = 0.0;
                        let mut right = 0.0;

                        if let Some(idx) = output_module_idx {
                            // --- MASTER OUT (PATCHED) ---
                            // Read from OUTPUT_BUFFERS of the AUDIO OUT module.
                            if let Some(node_outputs) = runner.output_buffers.get(idx) {
                                // Intelligent Mix Gain (0.1 ensures 16 voices sum to 1.6, which tanh handles gracefully)
                                let mix_gain = 0.1;
                                for i in 0..16 {
                                    left += node_outputs.get(i).copied().unwrap_or(0.0) * mix_gain;
                                    right +=
                                        node_outputs.get(16 + i).copied().unwrap_or(0.0) * mix_gain;
                                }
                            }
                        } else {
                            // --- FALLBACK (STANDALONE) ---
                            // If no AUDIO OUT module, output the very last node in the topological order.
                            if let Some(&last_idx) = current_snapshot.order.last() {
                                if let Some(node_outputs) = runner.output_buffers.get(last_idx) {
                                    let mix_gain = 0.1;
                                    for i in 0..16 {
                                        left +=
                                            node_outputs.get(i).copied().unwrap_or(0.0) * mix_gain;
                                    }
                                    right = left;
                                }
                            }
                        }

                        // Final Soft Clipping & Safety Clamp
                        // Perfectly linear up to 0.8, then smooth transition to 1.0.
                        let soft_clip = |x: f32| -> f32 {
                            let abs_x = x.abs();
                            if abs_x <= 0.8 {
                                x
                            } else {
                                let sign = if x > 0.0 { 1.0 } else { -1.0 };
                                sign * (0.8 + 0.2 * libm::tanhf((abs_x - 0.8) / 0.2))
                            }
                        };

                        left = soft_clip(left).clamp(-1.0, 1.0);
                        right = soft_clip(right).clamp(-1.0, 1.0);

                        if channels >= 2 {
                            frame[0] = left;
                            frame[1] = right;
                            for i in 2..channels {
                                frame[i] = 0.0;
                            }
                        } else if channels == 1 {
                            frame[0] = (left + right) * 0.5;
                        }
                    }

                    // 3. 鑑識データの収集 (間引き処理: ~40ms ごと @ 44.1kHz)
                    visual_throttle += data.len() / channels;
                    if visual_throttle >= 1024 {
                        visual_throttle = 0;
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
                    }
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

        Ok((engine, visual_out, hardware_sample_rate))
    }

    pub fn sync_aging(&self, aging: f32) -> Result<(), String> {
        self.aging_tx.send(aging).map_err(|e| e.to_string())
    }

    pub fn update_topology(
        &self,
        snapshot: GraphSnapshot,
        nodes: Vec<Box<dyn RackDspNode>>,
        params: Vec<Vec<f32>>,
    ) {
        // --- High-Performance Topology Swap ---
        // Instead of letting the audio thread do heavy lifting, we send the update.
        // The audio thread will now only swap the pointers, preserving the "Warm" nodes.
        let _ = self.topology_tx.send(TopologyUpdate {
            snapshot,
            nodes,
            params,
        });
    }

    pub fn update_module_parameters(&self, stable_id: u64, params: Vec<f32>) {
        let _ = self.param_tx.send(ParamChange { stable_id, params });
    }
}
