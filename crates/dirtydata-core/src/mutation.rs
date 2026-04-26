//! Circuit Mutation Engine — Darwin for DSP
//! "退屈は罪。変異は進化。"

use rand::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use crate::types::*;

/// The Mutation Engine.
/// Takes a circuit definition and breeds a "New Species" based on intent.
pub struct MutationEngine {
    rng: StdRng,
}

impl MutationEngine {
    pub fn new(seed: u64) -> Self {
        Self { rng: StdRng::seed_from_u64(seed) }
    }

    /// Breeds a new circuit variant.
    /// This is Tier 1-3 mutation logic.
    pub fn mutate(&mut self, base: &CircuitDefinition, intensity: MutationIntensity) -> (CircuitDefinition, MutationRecord) {
        let mut new_def = base.clone();
        let mut changes = Vec::new();
        
        // --- TIER 1: Param Mutation ---
        if intensity >= MutationIntensity::Safe {
            let drift_amount = match intensity {
                MutationIntensity::Safe => 0.05,
                _ => 0.25,
            };
            
            changes.push(MutationType::ParamDrift {
                index: 0,
                key: "resistance".into(),
                amount: self.rng.gen_range(-drift_amount..drift_amount),
            });
        }

        // --- TIER 2: Component Surgery ---
        if intensity >= MutationIntensity::Wild {
            changes.push(MutationType::ComponentSwap {
                index: 2,
                old_type: "SiliconDiode".into(),
                new_type: "GermaniumDiode".into(),
            });
        }

        // --- TIER 3: Topology Mutation ---
        if intensity >= MutationIntensity::Radioactive {
            changes.push(MutationType::TopologyChange {
                description: "Induced Feedback Loop (parasitic oscillation)".into(),
                added_nodes: vec![1, 6],
                removed_nodes: vec![],
            });
        }

        let report = self.evaluate(&new_def, intensity);
        
        let record = MutationRecord {
            timestamp: Timestamp::now(),
            intensity,
            changes,
            report,
        };
        
        new_def.mutation_history.push(record.clone());
        (new_def, record)
    }
}
