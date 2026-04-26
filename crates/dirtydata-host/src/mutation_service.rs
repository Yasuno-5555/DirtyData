//! Mutation Service — Asynchronous Circuit Evolution
//! "UIを止めるな。進化を止めるな。"

use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use dirtydata_core::mutation::MutationEngine;
use dirtydata_core::types::*;

#[derive(Debug, Clone)]
pub struct MutationPreview {
    pub new_def: CircuitDefinition,
    pub record: MutationRecord,
}

pub struct MutationService {
    engine: Arc<Mutex<MutationEngine>>,
}

impl MutationService {
    pub fn new(seed: u64) -> Self {
        Self {
            engine: Arc::new(Mutex::new(MutationEngine::new(seed))),
        }
    }

    /// Requests a mutation asynchronously.
    /// Returns a receiver that will yield the preview once the Micro-Simulation completes.
    /// This prevents the UI thread from freezing during complex MNA evaluations.
    pub async fn request_mutation(
        &self,
        base: CircuitDefinition,
        intensity: MutationIntensity,
    ) -> oneshot::Receiver<MutationPreview> {
        let (tx, rx) = oneshot::channel();
        let engine = self.engine.clone();

        tokio::spawn(async move {
            // Perform the mutation and evaluation in a background thread.
            // Tier 3/4 mutations may take significant time in Newton-Raphson loops.
            let mut engine = engine.lock().await;
            let (new_def, record) = engine.mutate(&base, intensity);
            
            let preview = MutationPreview {
                new_def,
                record,
            };

            let _ = tx.send(preview);
        });

        rx
    }
}
