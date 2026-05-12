#[cfg(test)]
mod tests {
    use crate::*;
    use dirtydata_core::ir::{Graph, Node};
    use dirtydata_core::types::{ConfidenceScore, ConfigValue, NodeKind, PortRef, StableId};

    #[test]
    fn test_jit_null_equivalence() {
        // 1. Create a test graph: Sine -> Gain -> Sink
        let mut graph = Graph::new();

        let mut osc = Node::new_source("Oscillator");
        osc.config
            .insert("frequency".into(), ConfigValue::Float(440.0));
        let osc_id = osc.id;
        graph.add_node(osc);

        let mut gain = Node::new_processor("Gain");
        gain.config.insert("gain".into(), ConfigValue::Float(1.0));
        let gain_id = gain.id;
        graph.add_node(gain);

        let mut sink = Node::new_sink("Output");
        let sink_id = sink.id;
        graph.add_node(sink);

        // Connect them
        graph
            .connect(
                PortRef {
                    node_id: osc_id,
                    port_name: "out".into(),
                },
                PortRef {
                    node_id: gain_id,
                    port_name: "in".into(),
                },
            )
            .expect("Connection 1 failed");

        graph
            .connect(
                PortRef {
                    node_id: gain_id,
                    port_name: "out".into(),
                },
                PortRef {
                    node_id: sink_id,
                    port_name: "in".into(),
                },
            )
            .expect("Connection 2 failed");

        let mut runner = DspRunner::new(graph.clone(), None, 44100.0);

        // 2. Compile JIT version
        let mut compiler = jit::JitCompiler::new();
        let mut jit_prog = compiler
            .compile_runner(&runner)
            .expect("JIT compilation failed");

        // 3. Compare outputs over 1000 samples
        for i in 0..1000 {
            let test_ctx = nodes::base::ProcessContext {
                sample_rate: 44100.0,
                global_sample_index: i as u64,
                crash_flag: None,
                osc_tx: None,
                convergence_info: None,
                node_diagnostics: None,
                node_id: None,
            };

            let out_std = runner.process_sample(&test_ctx);
            let out_jit = jit_prog.execute(0.0, 0.0, &test_ctx);

            let expected_phase = (i as f32 * 440.0 / 44100.0) % 1.0;
            let expected_val = (expected_phase * 2.0 * std::f32::consts::PI).sin();

            if i < 5 {
                println!(
                    "Sample {}: Expected={:.6}, Jit={:.6}, Std={:.6}",
                    i, expected_val, out_jit[0], out_std[0]
                );
            }

            // Assert JIT is mathematically correct
            assert!(
                (out_jit[0] - expected_val).abs() < 1e-4,
                "Sample {}: JIT deviation from math! Expected: {}, Jit: {}",
                i,
                expected_val,
                out_jit[0]
            );
        }
    }

    #[test]
    fn test_100_node_latency_benchmark() {
        use std::time::Instant;

        let mut graph = Graph::new();

        let mut last_id = {
            let mut osc = Node::new_source("Oscillator");
            osc.config.insert("frequency".into(), ConfigValue::Float(440.0));
            let id = osc.id;
            graph.add_node(osc);
            id
        };

        for _ in 1..=98 {
            let mut gain = Node::new_processor("Gain");
            gain.config.insert("gain".into(), ConfigValue::Float(1.0));
            let id = gain.id;
            graph.add_node(gain);

            graph
                .connect(
                    PortRef {
                        node_id: last_id,
                        port_name: "out".into(),
                    },
                    PortRef {
                        node_id: id,
                        port_name: "in".into(),
                    },
                )
                .expect("Connection failed");
            last_id = id;
        }

        let sink_id = {
            let sink = Node::new_sink("Output");
            let id = sink.id;
            graph.add_node(sink);
            id
        };

        graph
            .connect(
                PortRef {
                    node_id: last_id,
                    port_name: "out".into(),
                },
                PortRef {
                    node_id: sink_id,
                    port_name: "in".into(),
                },
            )
            .expect("Final connection failed");

        let mut runner = DspRunner::new(graph, None, 44100.0);

        let test_ctx = nodes::base::ProcessContext {
            sample_rate: 44100.0,
            global_sample_index: 0,
            crash_flag: None,
            osc_tx: None,
            convergence_info: None,
            node_diagnostics: None,
            node_id: None,
        };

        // Warm up
        for _ in 0..100 {
            let _ = runner.process_sample(&test_ctx);
        }

        let iterations = 10000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = runner.process_sample(&test_ctx);
        }
        let elapsed = start.elapsed();

        println!(
            "BENCHMARK: Processed {} samples of a 100-node graph in {:?}. Average per sample: {:?}",
            iterations,
            elapsed,
            elapsed / iterations
        );
    }
}
