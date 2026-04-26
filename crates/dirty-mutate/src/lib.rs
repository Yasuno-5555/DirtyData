use dirtydata_core::ir::{Graph};
use dirtydata_core::patch::{Patch, Operation};
use dirtydata_core::types::{ConfigValue, PatchSource, TrustLevel, ConfigChange};
use anyhow::Result;
use rand::Rng;
use indicatif::ProgressBar;

pub struct Mutator;

impl Mutator {
    /// Run an evolutionary search for the "best" patch based on a fitness function.
    pub fn evolve(
        base_graph: &Graph,
        target_node_id: &str,
        epochs: usize,
        mutation_level: f64,
    ) -> Result<Patch> {
        let pb = ProgressBar::new(epochs as u64);
        pb.set_message("Evolving...");

        // Simplified: Just random walk for now
        let current_graph = base_graph.clone();
        let mut best_ops = Vec::new();

        for _ in 0..epochs {
            let mut ops = Vec::new();
            let mut rng = rand::thread_rng();

            // Mutate a random parameter of the target node
            if let Some((id, node)) = current_graph.topology.nodes.iter().find(|(_, n)| n.id.to_string().contains(target_node_id)) {
                for (key, val) in &node.config {
                    if let ConfigValue::Float(f) = val {
                        let delta = (rng.gen::<f64>() - 0.5) * mutation_level;
                        let mut config_delta = std::collections::BTreeMap::new();
                        config_delta.insert(key.clone(), ConfigChange {
                            old: Some(val.clone()),
                            new: Some(ConfigValue::Float(f + delta)),
                        });
                        
                        ops.push(Operation::ModifyConfig {
                            node_id: *id,
                            delta: config_delta,
                        });
                    }
                }
            }

            // In a real GA, we'd evaluate fitness here.
            // For now, we just keep the last one.
            best_ops = ops;
            pb.inc(1);
        }

        pb.finish_with_message("Evolution complete.");

        Ok(Patch::from_operations_with_provenance(
            best_ops,
            PatchSource::UserDirect,
            TrustLevel::Untrusted,
        ))
    }
}
