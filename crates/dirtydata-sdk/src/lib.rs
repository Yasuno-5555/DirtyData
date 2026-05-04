/// §SSS: DirtyData SDK — The Gateway to the Ecosystem.
/// "内輪の神話を、人類の標準へ。"
pub use dirtydata_core::ir::{Edge, Graph, Modulation, Node};
pub use dirtydata_core::patch::{Operation, Patch};
pub use dirtydata_core::types::*;
pub use dirtydata_host::{AuditReport, Workspace};
pub use dirtydata_intent::{IntentNode, IntentState, IntentStrategy};
pub use dirtydata_runtime::nodes as builtin;
pub use dirtydata_runtime::AudioEngine;

// DSP re-exports (Feature-gated)
#[cfg(feature = "bbd")]
pub use dirtydata_dsp_bbd as bbd;
#[cfg(feature = "chaos")]
pub use dirtydata_dsp_chaos as chaos;
#[cfg(feature = "circuit")]
pub use dirtydata_dsp_circuit as circuit;
#[cfg(feature = "clipper")]
pub use dirtydata_dsp_clipper as clipper;
#[cfg(feature = "control")]
pub use dirtydata_dsp_control as control;
#[cfg(feature = "cv")]
pub use dirtydata_dsp_cv as cv;
#[cfg(feature = "destruction")]
pub use dirtydata_dsp_destruction as destruction;
#[cfg(feature = "ks")]
pub use dirtydata_dsp_ks as ks;
#[cfg(feature = "matrix")]
pub use dirtydata_dsp_matrix as matrix;
#[cfg(feature = "modal")]
pub use dirtydata_dsp_modal as modal;
#[cfg(feature = "reaction")]
pub use dirtydata_dsp_reaction as reaction;
#[cfg(feature = "spectral")]
pub use dirtydata_dsp_spectral as spectral;
#[cfg(feature = "spring")]
pub use dirtydata_dsp_spring as spring;
#[cfg(feature = "svf")]
pub use dirtydata_dsp_svf as svf;
#[cfg(feature = "tape")]
pub use dirtydata_dsp_tape as tape;
#[cfg(feature = "vocal")]
pub use dirtydata_dsp_vocal as vocal;
#[cfg(feature = "wdf")]
pub use dirtydata_dsp_wdf as wdf;
#[cfg(feature = "zdf")]
pub use dirtydata_dsp_zdf as zdf;

pub mod merge {
    use super::*;
    use anyhow::{anyhow, Result};

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
                            return Err(anyhow!(
                                "Merge Conflict: Node {} not found for removal",
                                id
                            ));
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
            ws.apply_patch(patch)
                .map_err(|e| anyhow!("Failed to apply patch: {}", e))?;

            Ok(())
        }
    }
}

/// §SSS: NodeFactory — Fluent API for building the forensic soundscape.
pub struct NodeFactory;

impl NodeFactory {
    pub fn oscillator(freq: f32) -> Node {
        let mut n = Node::new_processor("Oscillator");
        n.config
            .insert("frequency".into(), ConfigValue::Float(freq as f64));
        n
    }

    pub fn bit_crush(bits: f32, sr_div: f32) -> Node {
        let mut n = Node::new_processor("BitCrush");
        n.config
            .insert("bits".into(), ConfigValue::Float(bits as f64));
        n.config
            .insert("sr_div".into(), ConfigValue::Float(sr_div as f64));
        n
    }

    pub fn spectral_freeze() -> Node {
        Node::new_processor("SpectralFreeze")
    }

    pub fn wave_shaper(amount: f32) -> Node {
        let mut n = Node::new_processor("WaveShaper");
        n.config
            .insert("amount".into(), ConfigValue::Float(amount as f64));
        n
    }

    pub fn bbd_delay(delay_ms: f32) -> Node {
        let mut n = Node::new_processor("BbdDelay");
        n.config
            .insert("delay_ms".into(), ConfigValue::Float(delay_ms as f64));
        n
    }

    pub fn tape_machine() -> Node {
        Node::new_processor("TapeMachine")
    }

    pub fn chua_circuit() -> Node {
        Node::new_processor("Chua")
    }

    pub fn diode_clipper() -> Node {
        Node::new_processor("DiodeClipper")
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
            let out = unsafe {
                PLUGIN
                    .as_mut()
                    .expect("Plugin not initialized")
                    .process(in_l, in_r)
            };
            // Pack [f32; 2] into i64 for WASM boundary
            unsafe { std::mem::transmute::<[f32; 2], i64>(out) }
        }

        #[no_mangle]
        pub extern "C" fn set_parameter(id: u32, value: f32) {
            unsafe {
                PLUGIN
                    .as_mut()
                    .expect("Plugin not initialized")
                    .set_parameter(id, value);
            }
        }
    };
}
