use crate::nodes::base::{DspNode, NodeState, ProcessContext};
use dirtydata_core::types::{StableId, ConfigSnapshot};
use std::collections::HashMap;

/// DirtyData Primitive ISA (Instruction Set Architecture).
/// "ノードを解体せよ。原子こそが不変。"
#[derive(Clone, Debug)]
pub enum DspOp {
    // --- Memory & Data ---
    LoadConst { val: f32, out: usize },
    Copy { src: usize, dst: usize },

    // --- Basic Math ---
    Add { a: usize, b: usize, out: usize },
    Mul { a: usize, b: usize, out: usize },
    Sin { src: usize, out: usize },
    
    // --- State & History ---
    /// Increments a value by delta and wraps (for phases)
    Accumulate { reg: usize, delta_reg: usize, wrap: f32 },
    
    // --- Non-linear & Safety ---
    Tanh { src: usize, out: usize },
    
    // --- SSS+: Constraint Engine Integration ---
    /// Asserts that a register value is within bounds.
    /// Triggers a runtime hint if violated.
    AssertRange { reg: usize, min: f32, max: f32, node_id: StableId },

    // --- Foreign Bridge ---
    /// Call an opaque DspNode implementation (The necessary evil)
    CallLegacy { node_idx: usize, input_regs: Vec<usize>, output_regs: Vec<usize> },
}

pub struct JitProgram {
    pub ops: Vec<DspOp>,
    pub registers: Vec<[f32; 2]>,
    pub legacy_nodes: Vec<Box<dyn DspNode>>,
    /// Maps StableId to a diagnostic message for constraints
    pub constraint_violations: HashMap<StableId, String>,
}

impl JitProgram {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            registers: vec![[0.0; 2]; 1024], // Increased register file
            legacy_nodes: Vec::new(),
            constraint_violations: HashMap::new(),
        }
    }

    #[inline(always)]
    pub fn execute(&mut self, ctx: &ProcessContext) -> [f32; 2] {
        for op in &self.ops {
            match op {
                DspOp::LoadConst { val, out } => {
                    self.registers[*out] = [*val, *val];
                }
                DspOp::Copy { src, dst } => {
                    self.registers[*dst] = self.registers[*src];
                }
                DspOp::Add { a, b, out } => {
                    let v1 = self.registers[*a];
                    let v2 = self.registers[*b];
                    self.registers[*out] = [v1[0] + v2[0], v1[1] + v2[1]];
                }
                DspOp::Mul { a, b, out } => {
                    let v1 = self.registers[*a];
                    let v2 = self.registers[*b];
                    self.registers[*out] = [v1[0] * v2[0], v1[1] * v2[1]];
                }
                DspOp::Sin { src, out } => {
                    let v = self.registers[*src];
                    self.registers[*out] = [
                        (v[0] * 2.0 * std::f32::consts::PI).sin(),
                        (v[1] * 2.0 * std::f32::consts::PI).sin()
                    ];
                }
                DspOp::Accumulate { reg, delta_reg, wrap } => {
                    let mut v = self.registers[*reg];
                    let d = self.registers[*delta_reg];
                    for i in 0..2 {
                        v[i] = (v[i] + d[i]) % *wrap;
                    }
                    self.registers[*reg] = v;
                }
                DspOp::Tanh { src, out } => {
                    let v = self.registers[*src];
                    self.registers[*out] = [v[0].tanh(), v[1].tanh()];
                }
                DspOp::AssertRange { reg, min, max, node_id } => {
                    let v = self.registers[*reg];
                    if v[0] < *min || v[0] > *max || v[1] < *min || v[1] > *max {
                        if let Some(diag) = ctx.node_diagnostics {
                            diag.insert(*node_id, crate::DiagnosticRecord {
                                message: format!("Constraint Violation: Value {:.2} out of [{}, {}]", v[0], min, max),
                                severity: crate::DiagnosticSeverity::Warning,
                                timestamp: ctx.global_sample_index,
                            });
                        }
                    }
                }
                DspOp::CallLegacy { node_idx, input_regs, output_regs } => {
                    let node = &mut self.legacy_nodes[*node_idx];
                    // Map registers to flat inputs
                    let mut inputs = vec![0.0; input_regs.len()];
                    for (i, &reg) in input_regs.iter().enumerate() {
                        inputs[i] = self.registers[reg][0];
                    }
                    
                    let mut outputs = vec![[0.0; 2]; output_regs.len()];
                    node.process(&inputs, &mut outputs, &ConfigSnapshot::new(), ctx);
                    
                    for (i, &reg) in output_regs.iter().enumerate() {
                        self.registers[reg] = outputs[i];
                    }
                }
            }
        }
        self.registers[0] // Final Master Output
    }
}

pub struct JitCompiler {
    register_map: HashMap<StableId, usize>,
    next_register: usize,
    /// Differential Freeze Cache: Map subgraph hash to asset path
    pub freeze_cache: HashMap<[u8; 32], std::path::PathBuf>,
}

impl JitCompiler {
    pub fn new() -> Self {
        Self {
            register_map: HashMap::new(),
            next_register: 1, // 0 is reserved for master output
            freeze_cache: HashMap::new(),
        }
    }

    pub fn compile_runner(&mut self, runner: &crate::DspRunner) -> JitProgram {
        let mut program = JitProgram::new();
        let graph = runner.get_graph();
        let sample_rate = 44100.0;
        
        // 1. Register Allocation
        for (id, _) in &runner.nodes {
            self.register_map.insert(*id, self.next_register);
            self.next_register += 1;
        }

        // 2. Lowering Loop
        for (id, _node_impl) in &runner.nodes {
            let out_reg = *self.register_map.get(id).unwrap();
            
            if let Some(node_ir) = graph.nodes.get(id) {
                match &node_ir.kind {
                    dirtydata_core::types::NodeKind::Source => {
                        // --- LOWERING: Sine Oscillator ---
                        let freq = node_ir.config.get("frequency").and_then(|v| v.as_float()).unwrap_or(440.0) as f32;
                        
                        let delta_reg = self.next_register; self.next_register += 1;
                        program.ops.push(DspOp::LoadConst { val: freq / sample_rate, out: delta_reg });
                        program.ops.push(DspOp::Accumulate { reg: out_reg, delta_reg, wrap: 1.0 });
                        program.ops.push(DspOp::Sin { src: out_reg, out: out_reg });
                    }
                    dirtydata_core::types::NodeKind::Processor => {
                        // --- LOWERING: Gain ---
                        let gain = node_ir.config.get("gain").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
                        
                        let mut in_reg = 0;
                        for edge in graph.edges.values() {
                            if edge.target.node_id == *id {
                                if let Some(&src) = self.register_map.get(&edge.source.node_id) {
                                    in_reg = src; break;
                                }
                            }
                        }
                        
                        let gain_reg = self.next_register; self.next_register += 1;
                        program.ops.push(DspOp::LoadConst { val: gain, out: gain_reg });
                        program.ops.push(DspOp::Mul { a: in_reg, b: gain_reg, out: out_reg });
                    }
                    _ => {
                        program.ops.push(DspOp::CallLegacy { 
                            node_idx: program.legacy_nodes.len(),
                            input_regs: vec![],
                            output_regs: vec![out_reg] 
                        });
                    }
                }

                // --- SSS+: Automatic Constraint Monitoring ---
                program.ops.push(DspOp::AssertRange { 
                    reg: out_reg, 
                    min: -2.0, 
                    max: 2.0, 
                    node_id: *id 
                });
            }
        }

        if let Some(last_id) = runner.nodes.last().map(|(id, _)| id) {
            let last_reg = *self.register_map.get(last_id).unwrap();
            program.ops.push(DspOp::Copy { src: last_reg, dst: 0 });
        }
        program.ops.push(DspOp::Tanh { src: 0, out: 0 });

        // --- SSS: Optimization Pass ---
        let optimizer = JitOptimizer::new();
        optimizer.optimize(&mut program);

        program
    }
}

pub struct JitOptimizer {}

impl JitOptimizer {
    pub fn new() -> Self { Self {} }

    pub fn optimize(&self, program: &mut JitProgram) {
        self.common_subexpression_elimination(program);
        self.constant_folding(program);
        self.dead_code_elimination(program);
    }

    fn common_subexpression_elimination(&self, program: &mut JitProgram) {
        #[derive(Hash, PartialEq, Eq)]
        enum OpIdentity {
            Sin { src: usize },
            Add { a: usize, b: usize },
            Mul { a: usize, b: usize },
            Tanh { src: usize },
        }

        let mut available_expressions: HashMap<OpIdentity, usize> = HashMap::new();
        let mut i = 0;
        while i < program.ops.len() {
            let identity = match &program.ops[i] {
                DspOp::Sin { src, .. } => Some(OpIdentity::Sin { src: *src }),
                DspOp::Add { a, b, .. } => Some(OpIdentity::Add { a: *a, b: *b }),
                DspOp::Mul { a, b, .. } => Some(OpIdentity::Mul { a: *a, b: *b }),
                DspOp::Tanh { src, .. } => Some(OpIdentity::Tanh { src: *src }),
                _ => None,
            };

            if let Some(id) = identity {
                if let Some(&prev_out) = available_expressions.get(&id) {
                    // Duplicate found! Replace with Copy
                    let current_out = match &program.ops[i] {
                        DspOp::Sin { out, .. } | DspOp::Add { out, .. } | DspOp::Mul { out, .. } | DspOp::Tanh { out, .. } => *out,
                        _ => unreachable!(),
                    };
                    program.ops[i] = DspOp::Copy { src: prev_out, dst: current_out };
                } else {
                    let out = match &program.ops[i] {
                        DspOp::Sin { out, .. } | DspOp::Add { out, .. } | DspOp::Mul { out, .. } | DspOp::Tanh { out, .. } => *out,
                        _ => unreachable!(),
                    };
                    available_expressions.insert(id, out);
                }
            }
            i += 1;
        }
    }

    fn constant_folding(&self, program: &mut JitProgram) {
        // Simple 1-pass constant folder
        let mut constants: HashMap<usize, f32> = HashMap::new();
        let mut i = 0;
        while i < program.ops.len() {
            let mut removed = false;
            match &program.ops[i] {
                DspOp::LoadConst { val, out } => {
                    constants.insert(*out, *val);
                }
                DspOp::Add { a, b, out } => {
                    if let (Some(&v1), Some(&v2)) = (constants.get(a), constants.get(b)) {
                        let result = v1 + v2;
                        constants.insert(*out, result);
                        program.ops[i] = DspOp::LoadConst { val: result, out: *out };
                    }
                }
                DspOp::Mul { a, b, out } => {
                    if let (Some(&v1), Some(&v2)) = (constants.get(a), constants.get(b)) {
                        let result = v1 * v2;
                        constants.insert(*out, result);
                        program.ops[i] = DspOp::LoadConst { val: result, out: *out };
                    }
                }
                _ => {
                    // Non-constant op clears output register from constant map
                    // (Simplified: in real SSA this isn't needed)
                }
            }
            if !removed { i += 1; }
        }
    }

    fn dead_code_elimination(&self, program: &mut JitProgram) {
        let mut used_registers = std::collections::HashSet::new();
        used_registers.insert(0); // Master output is always used
        
        // Work backwards to find used registers
        for op in program.ops.iter().rev() {
            match op {
                DspOp::Add { a, b, out } | DspOp::Mul { a, b, out } => {
                    if used_registers.contains(out) {
                        used_registers.insert(*a);
                        used_registers.insert(*b);
                    }
                }
                DspOp::Sin { src, out } | DspOp::Tanh { src, out } | DspOp::Copy { src, dst: out } => {
                    if used_registers.contains(out) {
                        used_registers.insert(*src);
                    }
                }
                DspOp::Accumulate { reg, delta_reg, .. } => {
                    used_registers.insert(*reg);
                    used_registers.insert(*delta_reg);
                }
                DspOp::AssertRange { reg, .. } => {
                    used_registers.insert(*reg); // Assert counts as a use
                }
                _ => {}
            }
        }

        // Remove unused ops
        program.ops.retain(|op| {
            match op {
                DspOp::Add { out, .. } | DspOp::Mul { out, .. } | DspOp::Sin { out, .. } | DspOp::LoadConst { out, .. } => {
                    used_registers.contains(out)
                }
                _ => true // Keep control flow / assertions
            }
        });
    }
}
