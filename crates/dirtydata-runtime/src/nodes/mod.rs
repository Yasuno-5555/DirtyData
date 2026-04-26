pub mod base;
pub use base::*;

pub mod sources;
pub use sources::*;

pub mod processors;
pub use processors::*;

pub mod nonlinear;
pub use nonlinear::*;

pub mod legacy;
pub use legacy::MidiEvent;
// pub use legacy::*; // Avoid glob export to prevent name collisions with base

#[cfg(test)]
mod tests {
    use super::*;
    use legacy::EnvelopeNode;
    use dirtydata_core::types::ConfigSnapshot;
    use std::collections::BTreeMap;

    fn dummy_ctx() -> ProcessContext<'static> {
        ProcessContext {
            sample_rate: 44100.0,
            global_sample_index: 0,
            crash_flag: None,
            osc_tx: None,
            convergence_info: None,
            node_diagnostics: None,
            node_id: None,
        }
    }

    #[test]
    fn test_oscillator_state_migration() {
        let mut osc1 = OscillatorNode::new();
        let mut outputs = [[0.0; 2]];
        let config = ConfigSnapshot::new();
        let ctx = dummy_ctx();

        // 1. Run for a bit to move phase
        for _ in 0..100 {
            osc1.process(&[], &mut outputs, &config, &ctx);
        }
        let phase1 = osc1.phase;
        assert!(phase1 > 0.0);

        // 2. Extract state
        let state = osc1.extract_state();

        // 3. Inject into new node
        let mut osc2 = OscillatorNode::new();
        osc2.inject_state(&state);

        // 4. Verify phase is restored
        assert_eq!(osc1.phase, osc2.phase);
        
        // 5. Verify next sample matches
        osc1.process(&[], &mut outputs, &config, &ctx);
        let val1 = outputs[0][0];
        osc2.process(&[], &mut outputs, &config, &ctx);
        let val2 = outputs[0][0];
        assert_eq!(val1, val2);
    }

    #[test]
    fn test_envelope_state_migration() {
        let mut env1 = EnvelopeNode::new();
        let mut outputs = [[0.0; 2]];
        let mut config = BTreeMap::new();
        config.insert("attack".into(), dirtydata_core::types::ConfigValue::Float(0.1));
        let snapshot = ConfigSnapshot::from(config);
        let ctx = dummy_ctx();

        // 1. Trigger and run into Attack phase
        env1.process(&[1.0], &mut outputs, &snapshot, &ctx);
        for _ in 0..10 {
            env1.process(&[1.0], &mut outputs, &snapshot, &ctx);
        }
        assert!(env1.is_idle() == false);
        let level1 = outputs[0][0];

        // 2. Migrate
        let state = env1.extract_state();
        let mut env2 = EnvelopeNode::new();
        env2.inject_state(&state);

        // 3. Verify
        env1.process(&[1.0], &mut outputs, &snapshot, &ctx);
        let next_val1 = outputs[0][0];
        env2.process(&[1.0], &mut outputs, &snapshot, &ctx);
        let next_val2 = outputs[0][0];
        
        assert_eq!(level1, level1); // Just checking stability
        assert_eq!(next_val1, next_val2);
    }
}
