//! Offline Renderer — Deterministic Batch Execution
//!
//! GUI やオーディオ・コールバックに依存せず、最高速でパッチをレンダリングし、
//! 指定されたサンプル数だけオーディオ・バッファを出力。

use crate::runner::{GraphSnapshot, RackRunner};
use crate::signal::{RackDspNode, SeedScope};

pub struct OfflineRenderer {
    runner: RackRunner,
    snapshot: GraphSnapshot,
    params: Vec<Vec<f32>>,
}

impl OfflineRenderer {
    pub fn new(
        sample_rate: f32,
        seed_scope: SeedScope,
        snapshot: GraphSnapshot,
        nodes: Vec<Box<dyn RackDspNode>>,
        params: Vec<Vec<f32>>,
    ) -> Self {
        let mut runner = RackRunner::new(sample_rate, seed_scope);
        let mut snapshot = snapshot;
        runner.apply_snapshot(&mut snapshot, nodes);
        Self { runner, snapshot, params }
    }

    /// 指定されたサンプル数だけレンダリングし、ステレオバッファとハッシュを返す
    pub fn render_block(&mut self, samples: usize, output_module: usize) -> (Vec<(f32, f32)>, String) {
        let mut buffer = Vec::with_capacity(samples);
        let mut hasher = blake3::Hasher::new();

        for _ in 0..samples {
            self.runner.process_sample(&self.snapshot, &self.params);
            let l = self.runner.get_output(output_module, 0);
            let r = self.runner.get_output(output_module, 1);
            
            hasher.update(&l.to_le_bytes());
            hasher.update(&r.to_le_bytes());
            
            buffer.push((l, r));
        }
        (buffer, hasher.finalize().to_hex().to_string())
    }
}

pub struct DeepAuditor {
    engine_a: RackRunner,
    engine_b: RackRunner,
    snapshot: GraphSnapshot,
    params: Vec<Vec<f32>>,
}

impl DeepAuditor {
    pub fn new(
        sample_rate: f32,
        seed: u64,
        snapshot: GraphSnapshot,
        nodes_a: Vec<Box<dyn RackDspNode>>,
        nodes_b: Vec<Box<dyn RackDspNode>>,
        params: Vec<Vec<f32>>,
    ) -> Self {
        let mut engine_a = RackRunner::new(sample_rate, SeedScope::Global(seed));
        let mut engine_b = RackRunner::new(sample_rate, SeedScope::Global(seed));
        let mut snapshot = snapshot;
        engine_a.apply_snapshot(&mut snapshot, nodes_a);
        engine_b.apply_snapshot(&mut snapshot, nodes_b);
        Self { engine_a, engine_b, snapshot, params }
    }

    pub fn find_divergence(&mut self, max_samples: usize) -> Option<(usize, usize, f32, f32)> {
        for s in 0..max_samples {
            self.engine_a.process_sample(&self.snapshot, &self.params);
            self.engine_b.process_sample(&self.snapshot, &self.params);

            for &idx in &self.snapshot.order {
                let out_a = &self.engine_a.output_buffers[idx];
                let out_b = &self.engine_b.output_buffers[idx];
                
                for (_v, (&a, &b)) in out_a.iter().zip(out_b.iter()).enumerate() {
                    if (a - b).abs() > 1e-7 {
                        return Some((s, idx, a, b));
                    }
                }
            }
        }
        None
    }
}
