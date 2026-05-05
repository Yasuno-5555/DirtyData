use crate::runner::{GraphSnapshot, RackRunner};
use crate::signal::RackDspNode;
use crate::subpatch_io::{SubpatchInput, SubpatchOutput, SubpatchParameter};

pub struct CompositeModule {
    pub runner: RackRunner,
    pub snapshot: GraphSnapshot,
    pub initial_params: Vec<Vec<f32>>,
    /// Mapping from CompositeModule's input ports to internal SubpatchInput node indices
    pub input_mappings: Vec<usize>,
    /// Mapping from CompositeModule's output ports to internal SubpatchOutput node indices
    pub output_mappings: Vec<usize>,
    /// Mapping from CompositeModule's parameters to internal SubpatchParameter node indices
    pub param_mappings: Vec<usize>,
}

impl CompositeModule {
    pub fn new(
        sample_rate: f32,
        snapshot: GraphSnapshot,
        nodes: Vec<Box<dyn RackDspNode>>,
        initial_params: Vec<Vec<f32>>,
        seed: u64,
    ) -> Self {
        let mut runner = RackRunner::new(sample_rate, crate::signal::SeedScope::Global(seed));

        let mut input_mappings = Vec::new();
        let mut output_mappings = Vec::new();
        let mut param_mappings = Vec::new();

        // Find internal IO and Parameter nodes
        for (idx, node_id) in snapshot.node_type_ids.iter().enumerate() {
            if node_id == "subpatch_in" {
                input_mappings.push(idx);
            } else if node_id == "subpatch_out" {
                output_mappings.push(idx);
            } else if node_id == "subpatch_param" {
                param_mappings.push(idx);
            }
        }

        let mut snapshot_mut = snapshot.clone();
        runner.apply_snapshot(&mut snapshot_mut, nodes);

        Self {
            runner,
            snapshot: snapshot_mut,
            initial_params,
            input_mappings,
            output_mappings,
            param_mappings,
        }
    }
}

impl RackDspNode for CompositeModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &crate::signal::RackProcessContext,
    ) {
        // 1. Inject parent inputs into subpatch IO nodes
        for (i, &node_idx) in self.input_mappings.iter().enumerate() {
            if let Some(node) = self.runner.active_nodes.get_mut(node_idx) {
                if let Some(sub_in) = node.as_any_mut().downcast_mut::<SubpatchInput>() {
                    for v in 0..16 {
                        sub_in.signals[v] = inputs[i * 16 + v];
                    }
                }
            }
        }

        // 2. Inject parent parameters into subpatch Parameter nodes (Macro Knobs)
        for (i, &node_idx) in self.param_mappings.iter().enumerate() {
            if let Some(val) = params.get(i) {
                if let Some(node) = self.runner.active_nodes.get_mut(node_idx) {
                    if let Some(sub_param) = node.as_any_mut().downcast_mut::<SubpatchParameter>() {
                        sub_param.value = *val;
                    }
                }
            }
        }

        // 3. Step the subpatch runner
        self.runner
            .process_sample(&self.snapshot, &self.initial_params);

        // 4. Extract subpatch IO signals to parent outputs
        for (i, &node_idx) in self.output_mappings.iter().enumerate() {
            if let Some(node) = self.runner.active_nodes.get_mut(node_idx) {
                if let Some(sub_out) = node.as_any_mut().downcast_mut::<SubpatchOutput>() {
                    for v in 0..16 {
                        outputs[i * 16 + v] = sub_out.signals[v];
                    }
                }
            }
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
