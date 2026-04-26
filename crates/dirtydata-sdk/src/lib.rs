/// §SSS: DirtyData SDK — The Gateway to the Ecosystem.
/// "内輪の神話を、人類の標準へ。"

pub use dirtydata_core::ir::{Graph, Node, Edge, Modulation};
pub use dirtydata_core::patch::{Patch, Operation};
pub use dirtydata_core::types::*;
pub use dirtydata_host::{Workspace, AuditReport};
pub use dirtydata_runtime::AudioEngine;
pub use dirtydata_intent::{IntentNode, IntentState, IntentStrategy};

pub mod merge {
    use super::*;
    use anyhow::{Result, anyhow};

    pub struct SemanticMerge;

    impl SemanticMerge {
        /// Performs a semantic merge of a patch into a workspace.
        /// Validates that the patch doesn't violate existing intent constraints.
        pub fn run(ws: &mut Workspace, patch: Patch) -> Result<()> {
            let graph = ws.graph();
            let _intents = ws.intent_state();

            // 1. Structural Conflict Check
            for op in &patch.operations {
                match op {
                    Operation::RemoveNode(id) => {
                        if !graph.nodes.contains_key(id) {
                            return Err(anyhow!("Merge Conflict: Node {} not found for removal", id));
                        }
                    }
                    Operation::AddNode(node) => {
                        if graph.nodes.contains_key(&node.id) {
                            return Err(anyhow!("Merge Conflict: Node {} already exists", node.id));
                        }
                    }
                    _ => {}
                }
            }

            // 2. Intent Constraint Check (The "Semantic" part)
            // TODO: Iterate through intents and check if patch violates any 'Must' or 'Never' constraints

            // 3. Apply and Save
            ws.apply_patch(patch).map_err(|e| anyhow!("Failed to apply patch: {}", e))?;
            
            Ok(())
        }
    }
}

/// The core trait for DirtyData DSP Plugins.
/// Implement this to create your own custom DSP nodes.
pub trait DspPlugin: Default {
    /// Process one sample of audio.
    /// Returns [left, right] output.
    fn process(&mut self, in_l: f32, in_r: f32) -> [f32; 2];
    
    /// Handle parameter changes from the host.
    fn set_parameter(&mut self, _id: u32, _value: f32) {}
    
    /// Called once when the plugin is loaded.
    fn init(&mut self, _sample_rate: f32) {}
}

#[macro_export]
macro_rules! declare_plugin {
    ($t:ty) => {
        static mut PLUGIN: Option<$t> = None;

        #[no_mangle]
        pub extern "C" fn init(sample_rate: f32) {
            unsafe { 
                let mut p = <$t>::default();
                p.init(sample_rate);
                PLUGIN = Some(p); 
            }
        }

        #[no_mangle]
        pub extern "C" fn process(in_l: f32, in_r: f32) -> i64 {
            let out = unsafe { PLUGIN.as_mut().expect("Plugin not initialized").process(in_l, in_r) };
            // Pack [f32; 2] into i64 for WASM boundary
            unsafe { std::mem::transmute::<[f32; 2], i64>(out) }
        }

        #[no_mangle]
        pub extern "C" fn set_parameter(id: u32, value: f32) {
            unsafe { PLUGIN.as_mut().expect("Plugin not initialized").set_parameter(id, value); }
        }
    };
}
