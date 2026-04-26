use crate::nodes::ProcessContext;
use crate::DspRunner;
use dirtydata_core::ir::Graph;
use dirtydata_observer::divergence::{DivergenceMap, DivergencePoint};

/// A renderer for offline (faster than real-time) audio generation.
pub struct OfflineRenderer {
    runner: DspRunner,
    sample_rate: f32,
}

impl OfflineRenderer {
    pub fn new(graph: Graph, sample_rate: f32) -> Self {
        // Offline rendering currently doesn't support live MIDI input
        let runner = DspRunner::new(graph, None, sample_rate);
        Self { runner, sample_rate }
    }

    /// Renders the specified duration of audio.
    /// Returns interleaved stereo samples [L, R, L, R, ...].
    pub fn render(&mut self, duration_secs: f32) -> Vec<f32> {
        let num_samples = (duration_secs * self.sample_rate) as usize;
        let mut output = Vec::with_capacity(num_samples * 2);

        for i in 0..num_samples {
            let ctx = ProcessContext {
                sample_rate: self.sample_rate,
                global_sample_index: i as u64,
                crash_flag: None,
                osc_tx: None,
                convergence_info: None,
                node_diagnostics: None,
                node_id: None,
            };
            let sample = self.runner.process_sample(&ctx);
            output.push(sample[0]);
            output.push(sample[1]);
        }
        output
    }

    /// Verifies that two independent runs produce the exact same output.
    pub fn verify_determinism(graph: Graph, duration_secs: f32, sample_rate: f32) -> Result<bool, String> {
        let mut r1 = Self::new(graph.clone(), sample_rate);
        let mut r2 = Self::new(graph, sample_rate);
        
        let out1 = r1.render(duration_secs);
        let out2 = r2.render(duration_secs);
        
        if out1.len() != out2.len() {
            return Err("Output length mismatch between identical runs".into());
        }
        
        for (s1, s2) in out1.iter().zip(out2.iter()) {
            if (*s1 - *s2).abs() > 0.0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Compares two graphs sample-by-sample and node-by-node.
    /// This is the heart of the "Replay Divergence Map".
    pub fn compare(
        graph_a: Graph,
        graph_b: Graph,
        duration_secs: f32,
        sample_rate: f32,
    ) -> DivergenceMap {
        let mut r_a = DspRunner::new(graph_a, None, sample_rate);
        let mut r_b = DspRunner::new(graph_b, None, sample_rate);

        let num_samples = (duration_secs * sample_rate) as usize;
        let mut map = DivergenceMap::new();

        for i in 0..num_samples {
            let ctx = ProcessContext {
                sample_rate,
                global_sample_index: i as u64,
                crash_flag: None,
                osc_tx: None,
                convergence_info: None,
                node_diagnostics: None,
                node_id: None,
            };

            r_a.process_sample(&ctx);
            r_b.process_sample(&ctx);

            // Compare outputs of all nodes that exist in both runners
            let ids: Vec<_> = r_a.get_graph().nodes.keys().cloned().collect();
            for id in ids {
                if let (Some(out_a), Some(out_b)) = (r_a.node_outputs.get(&id), r_b.node_outputs.get(&id)) {
                    for (p_idx, (v_a, v_b)) in out_a.iter().zip(out_b.iter()).enumerate() {
                        let diff_l = (v_a[0] - v_b[0]).abs();
                        let diff_r = (v_a[1] - v_b[1]).abs();
                        let mag = diff_l.max(diff_r);

                        if mag > 1e-7 { // Tolerance for floating point
                            map.add_point(DivergencePoint {
                                sample_index: i as u64,
                                node_id: id,
                                node_name: "Unknown".into(), // Should fetch from graph
                                port_idx: p_idx,
                                expected_value: *v_a,
                                actual_value: *v_b,
                                diff_magnitude: mag,
                            });
                            
                            if map.points.len() > 100 {
                                return map; // Cap it for now
                            }
                        }
                    }
                }
            }
        }

        map
    }
}
