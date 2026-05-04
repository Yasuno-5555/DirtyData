//! Rack Runner — Audio Thread Execution Engine
//!
//! # 実行モデル: Hybrid Cached Push
//! 1. トポロジカルソート済みのノード配列を走査。
//! 2. `ArcSwap` によるロックフリーなスナップショット差し替え。
//! 3. BTreeMap 排除によるインデックスベースの高速アクセス。

use crate::drift_engine::VoiceDriftEngine;
use crate::signal::{RackDspNode, RackProcessContext, SeedScope};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Clone)]
pub struct Connection {
    pub from_module: usize,
    pub from_port: usize,
    pub to_module: usize,
    pub to_port: usize,
}

#[derive(Clone)]
pub struct ModEntry {
    pub param_idx: usize,
    pub src_module_idx: usize,
    pub src_port_idx: usize,
    pub amount: f32,
}

#[derive(Clone)]
pub struct GraphSnapshot {
    pub order: Vec<usize>,
    pub connections: Vec<Connection>,
    pub port_counts: Vec<(usize, usize)>,
    pub node_ids: Vec<u64>,
    pub node_type_ids: Vec<String>,
    pub forward_edges: Vec<Vec<Connection>>,
    pub back_edges: Vec<Connection>,
    pub modulations: Vec<Vec<ModEntry>>, // [module_idx] -> Vec<ModEntry>
}

pub struct RackRunner {
    pub sample_rate: f32,
    pub ctx: RackProcessContext,
    pub active_nodes: Vec<Box<dyn RackDspNode>>,
    pub input_buffers: Vec<Vec<f32>>,
    pub output_buffers: Vec<Vec<f32>>,
    pub modulated_params: Vec<Vec<f32>>, // [module_idx][param_idx]
    pub stats: Vec<crate::signal::EngineStats>,
    pub drift_engine: VoiceDriftEngine,
    pub node_personalities: Vec<[f32; 16]>,
}

impl RackRunner {
    pub fn new(sample_rate: f32, seed_scope: SeedScope) -> Self {
        let seed = match seed_scope {
            SeedScope::Global(s) | SeedScope::Module(s) | SeedScope::Voice(s) => s,
        };
        Self {
            sample_rate,
            ctx: RackProcessContext::new(sample_rate, seed),
            active_nodes: Vec::new(),
            input_buffers: Vec::new(),
            output_buffers: Vec::new(),
            modulated_params: Vec::new(),
            stats: Vec::new(),
            drift_engine: VoiceDriftEngine::new(seed),
            node_personalities: Vec::new(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: &mut GraphSnapshot, nodes: Vec<Box<dyn RackDspNode>>) {
        let mut order_map = vec![0; nodes.len()];
        for (i, &node_idx) in snapshot.order.iter().enumerate() {
            order_map[node_idx] = i;
        }

        let mut forward = vec![Vec::new(); nodes.len()];
        let mut back = Vec::new();

        for conn in &snapshot.connections {
            if order_map[conn.from_module] < order_map[conn.to_module] {
                forward[conn.from_module].push(conn.clone());
            } else {
                back.push(conn.clone());
            }
        }
        snapshot.forward_edges = forward;
        snapshot.back_edges = back;

        self.active_nodes = nodes;
        self.output_buffers = snapshot
            .port_counts
            .iter()
            .map(|(_, out)| vec![0.0; out * 16])
            .collect();
        self.input_buffers = snapshot
            .port_counts
            .iter()
            .map(|(inp, _)| vec![0.0; inp * 16])
            .collect();
        
        self.modulated_params = vec![vec![0.0; 64]; self.active_nodes.len()];
        self.stats = vec![crate::signal::EngineStats::default(); self.active_nodes.len()];

        self.node_personalities = Vec::with_capacity(self.active_nodes.len());
        for i in 0..self.active_nodes.len() {
            let node_id = snapshot.node_ids.get(i).copied().unwrap_or(0);
            let mut personalities = [0.0; 16];
            for v in 0..16 {
                let mut hasher = ChaCha8Rng::seed_from_u64(
                    self.ctx.project_seed ^ (node_id as u64) ^ (v as u64),
                );
                personalities[v] = hasher.gen_range(-1.0..1.0);
            }
            self.node_personalities.push(personalities);
        }
        self.ctx.sample_index = 0;
    }

    pub fn process_sample(&mut self, snapshot: &GraphSnapshot, base_params: &[Vec<f32>]) {
        self.drift_engine.process(&mut self.ctx.imperfection.drift);
        for d in &mut self.ctx.imperfection.drift {
            *d *= 0.1 + self.ctx.aging * 2.0;
        }

        // 1. Clear all inputs
        for buf in &mut self.input_buffers {
            buf.fill(0.0);
        }

        // 2. Add feedback (backward edges)
        for conn in &snapshot.back_edges {
            let src_start = conn.from_port * 16;
            let dst_start = conn.to_port * 16;
            let src_node = &self.output_buffers[conn.from_module];
            let dst_node = &mut self.input_buffers[conn.to_module];
            for v in 0..16 {
                dst_node[dst_start + v] += src_node[src_start + v];
            }
        }

        // 3. Process in topological order
        for &idx in &snapshot.order {
            if let Some(node) = self.active_nodes.get_mut(idx) {
                self.ctx.imperfection.personality = self.node_personalities[idx];

                // --- Apply Modulations ---
                // Copy base params to modulated_params first
                if let Some(base) = base_params.get(idx) {
                    let target = &mut self.modulated_params[idx];
                    let len = base.len().min(target.len());
                    target[..len].copy_from_slice(&base[..len]);
                }

                // Apply modulations on top
                for mod_entry in &snapshot.modulations[idx] {
                    if let Some(src_buf) = self.output_buffers.get(mod_entry.src_module_idx) {
                        let mod_val = src_buf[mod_entry.src_port_idx * 16]; 
                        self.modulated_params[idx][mod_entry.param_idx] += mod_val * mod_entry.amount;
                    }
                }

                let ins = &self.input_buffers[idx];
                let outs = &mut self.output_buffers[idx];
                node.process(ins, outs, &self.modulated_params[idx], &self.ctx);

                // --- High-End Signal Integrity Sentry & Stats ---
                let stats = &mut self.stats[idx];
                let mut current_energy = 0.0;
                for val in outs.iter_mut() {
                    let abs_val = val.abs();
                    current_energy += abs_val;
                    
                    // Peak detection
                    if abs_val > stats.peak_db {
                        stats.peak_db = abs_val;
                    }

                    if val.is_nan() || val.is_infinite() {
                        stats.clipping_count += 1;
                        *val = 0.0;
                    } else if abs_val > 5.0 {
                        // Clipping detection (Eurorack ±5V standard)
                        stats.clipping_count += 1;
                    } else if abs_val < 1e-15 && abs_val > 0.0 {
                        // Denormal Protection: Flush to zero
                        stats.denormal_count += 1;
                        *val = 0.0;
                    }
                    
                    // DC Offset tracking (Simple leaky integrator)
                    stats.dc_offset = stats.dc_offset * 0.999 + (*val) * 0.001;

                    // Safety Clamp (±24V)
                    *val = val.clamp(-24.0, 24.0);
                }
                stats.energy_delta = current_energy;

                // 4. Zero-Latency Push
                for conn in &snapshot.forward_edges[idx] {
                    let src_start = conn.from_port * 16;
                    let dst_start = conn.to_port * 16;
                    let src_node = &self.output_buffers[conn.from_module];
                    let dst_node = &mut self.input_buffers[conn.to_module];
                    for v in 0..16 {
                        dst_node[dst_start + v] += src_node[src_start + v];
                    }
                }
            }
        }

        self.ctx.advance();
    }

    pub fn get_output(&self, module_idx: usize, port_idx: usize) -> f32 {
        if let Some(buf) = self.output_buffers.get(module_idx) {
            return *buf.get(port_idx * 16).unwrap_or(&0.0);
        }
        0.0
    }
}
