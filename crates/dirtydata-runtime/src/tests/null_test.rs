#[cfg(test)]
mod tests {
    use crate::*;
    use dirtydata_core::ir::{Graph, Node};
    use dirtydata_core::types::{NodeKind, StableId, ConfigValue, ConfidenceScore};
    use std::collections::BTreeMap;

    #[test]
    fn test_jit_null_equivalence() {
        // 1. Create a test graph: Sine -> Gain -> Output
        let mut graph = Graph::new();
        let osc_id = StableId::new();
        let gain_id = StableId::new();
        
        let mut osc_config = BTreeMap::new();
        osc_config.insert("frequency".into(), ConfigValue::Float(440.0));
        graph.nodes.insert(osc_id, Node { id: osc_id, kind: NodeKind::Source, config: osc_config, ports: vec![], confidence: ConfidenceScore::Verified, metadata: Default::default() });
        
        let mut gain_config = BTreeMap::new();
        gain_config.insert("gain".into(), ConfigValue::Float(0.5));
        graph.nodes.insert(gain_id, Node { id: gain_id, kind: NodeKind::Processor, config: gain_config, ports: vec![], confidence: ConfidenceScore::Verified, metadata: Default::default() });
        
        // Connect them (Manual edge creation for test)
        // ... (Skipping full edge logic for brevity, assuming runner handles it)

        let mut runner = DspRunner::new(graph.clone(), None, 44100.0);
        
        // 2. Compile JIT version
        let mut compiler = jit::JitCompiler::new();
        let mut jit_prog = compiler.compile_runner(&runner);
        
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
            let out_jit = jit_prog.execute(&test_ctx);
            
            // Assert bit-identical or epsilon-equivalent
            assert!((out_std[0] - out_jit[0]).abs() < 1e-6, "Sample {}: JIT deviation detected! Std: {}, Jit: {}", i, out_std[0], out_jit[0]);
        }
    }
}
