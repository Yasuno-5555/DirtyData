use crate::nodes::base::{DspNode, NodeState, ProcessContext};
use dirtydata_core::types::{ConfigSnapshot, NodeKind, StableId};
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, ExportSection, Function, FunctionSection, Instruction, MemorySection,
    Module as WasmModule, TypeSection, ValType,
};
use wasmtime::*;

pub struct JitStoreData {
    pub legacy_nodes: Vec<Box<dyn DspNode>>,
    pub configs: Vec<ConfigSnapshot>,
    pub sample_rate: f32,
    pub global_sample_index: u64,
}

pub struct JitProgram {
    store: Store<JitStoreData>,
    instance: Instance,
    process_fn: TypedFunc<(f32, f32, f32, u64), (f32, f32)>,
    parameter_map: HashMap<(StableId, String), usize>,
    legacy_node_ids: Vec<StableId>,
}

impl JitProgram {
    pub fn new(
        engine: &Engine,
        wasm_bytes: &[u8],
        legacy_nodes: Vec<Box<dyn DspNode>>,
        legacy_node_ids: Vec<StableId>,
        configs: Vec<ConfigSnapshot>,
        parameter_map: HashMap<(StableId, String), usize>,
    ) -> anyhow::Result<Self> {
        let data = JitStoreData {
            legacy_nodes,
            configs,
            sample_rate: 44100.0,
            global_sample_index: 0,
        };
        let mut store = Store::new(engine, data);
        let module = Module::new(engine, wasm_bytes)?;
        let mut linker = Linker::new(engine);

        linker.func_wrap("host", "sin", |_: Caller<'_, JitStoreData>, x: f32| x.sin())?;
        linker.func_wrap("host", "tanh", |_: Caller<'_, JitStoreData>, x: f32| {
            x.tanh()
        })?;

        linker.func_wrap(
            "host",
            "call_legacy",
            |mut caller: Caller<'_, JitStoreData>,
             node_idx: i32,
             in_ptr: i32,
             in_len: i32,
             out_ptr: i32,
             out_len: i32| {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return,
                };
                let mut inputs = vec![0.0f32; in_len as usize];
                let mut outputs = vec![[0.0f32; 2]; out_len as usize];
                let mem_data = mem.data(&caller);
                for i in 0..in_len as usize {
                    let offset = (in_ptr as usize) + i * 4;
                    if offset + 4 <= mem_data.len() {
                        inputs[i] =
                            f32::from_le_bytes(mem_data[offset..offset + 4].try_into().unwrap());
                    }
                }
                let ctx = ProcessContext {
                    sample_rate: caller.data().sample_rate,
                    global_sample_index: caller.data().global_sample_index,
                    crash_flag: None,
                    osc_tx: None,
                    convergence_info: None,
                    node_diagnostics: None,
                    node_id: None,
                };
                let (node, config) = {
                    let d = caller.data_mut();
                    (
                        &mut d.legacy_nodes[node_idx as usize],
                        &d.configs[node_idx as usize],
                    )
                };
                node.process(&inputs, &mut outputs, config, &ctx);
                let mem_data = mem.data_mut(&mut caller);
                for i in 0..out_len as usize {
                    let offset = (out_ptr as usize) + i * 8;
                    if offset + 8 <= mem_data.len() {
                        mem_data[offset..offset + 4].copy_from_slice(&outputs[i][0].to_le_bytes());
                        mem_data[offset + 4..offset + 8]
                            .copy_from_slice(&outputs[i][1].to_le_bytes());
                    }
                }
            },
        )?;

        linker.func_wrap(
            "host",
            "call_mna",
            |mut caller: Caller<'_, JitStoreData>,
             node_idx: i32,
             in_ptr: i32,
             in_len: i32,
             out_ptr: i32,
             out_len: i32| {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return,
                };
                let mut inputs = vec![0.0f32; in_len as usize];
                let mem_data = mem.data(&caller);
                for i in 0..in_len as usize {
                    let offset = (in_ptr as usize) + i * 4;
                    if offset + 4 <= mem_data.len() {
                        inputs[i] =
                            f32::from_le_bytes(mem_data[offset..offset + 4].try_into().unwrap());
                    }
                }

                let data = caller.data_mut();
                let node = &mut data.legacy_nodes[node_idx as usize];
                let mut outputs = vec![[0.0f32; 2]; out_len as usize];

                node.process(
                    &inputs,
                    &mut outputs,
                    &data.configs[node_idx as usize],
                    &ProcessContext {
                        sample_rate: data.sample_rate,
                        global_sample_index: data.global_sample_index,
                        crash_flag: None,
                        osc_tx: None,
                        convergence_info: None,
                        node_diagnostics: None,
                        node_id: None,
                    },
                );

                let mem_data = mem.data_mut(&mut caller);
                for i in 0..out_len as usize {
                    let offset = (out_ptr as usize) + i * 8;
                    if offset + 8 <= mem_data.len() {
                        mem_data[offset..offset + 4].copy_from_slice(&outputs[i][0].to_le_bytes());
                        mem_data[offset + 4..offset + 8]
                            .copy_from_slice(&outputs[i][1].to_le_bytes());
                    }
                }
            },
        )?;

        let instance = linker.instantiate(&mut store, &module)?;
        let process_fn =
            instance.get_typed_func::<(f32, f32, f32, u64), (f32, f32)>(&mut store, "process")?;
        Ok(Self {
            store,
            instance,
            process_fn,
            parameter_map,
            legacy_node_ids,
        })
    }

    pub fn set_parameter(&mut self, node_id: &StableId, param: &str, value: f32) {
        // 1. Update native JIT parameters in WASM memory
        if let Some(&offset) = self.parameter_map.get(&(*node_id, param.to_string())) {
            if let Some(Extern::Memory(mem)) = self.instance.get_export(&mut self.store, "memory") {
                let mem_data = mem.data_mut(&mut self.store);
                if offset + 4 <= mem_data.len() {
                    mem_data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                }
            }
        }

        // 2. Update legacy nodes in the store
        if let Some(idx) = self.legacy_node_ids.iter().position(|id| id == node_id) {
            let data = self.store.data_mut();
            data.legacy_nodes[idx].update_parameter(param, value);
        }
    }

    pub fn reset(&mut self) {
        let data = self.store.data_mut();
        for node in &mut data.legacy_nodes {
            node.inject_state(&NodeState::Empty);
        }
        // TODO: Reset state memory in WASM if necessary (e.g. oscillators)
        if let Some(Extern::Memory(mem)) = self.instance.get_export(&mut self.store, "memory") {
            let mem_data = mem.data_mut(&mut self.store);
            // Reset state block (assumed at 0..65536)
            for i in 0..65536 {
                if i < mem_data.len() {
                    mem_data[i] = 0;
                }
            }
        }
    }

    pub fn set_parameter_by_name(&mut self, name: &str, value: f32) {
        let keys: Vec<(StableId, String)> = self
            .parameter_map
            .keys()
            .filter(|(_, k)| k == name)
            .cloned()
            .collect();
        for (id, k) in keys {
            self.set_parameter(&id, &k, value);
        }
    }

    pub fn latency_samples(&self) -> u32 {
        self.store
            .data()
            .legacy_nodes
            .iter()
            .map(|n| n.latency())
            .max()
            .unwrap_or(0)
    }

    #[inline(always)]
    pub fn execute(&mut self, input_l: f32, input_r: f32, ctx: &ProcessContext) -> [f32; 2] {
        let data = self.store.data_mut();
        data.sample_rate = ctx.sample_rate;
        data.global_sample_index = ctx.global_sample_index;
        match self.process_fn.call(
            &mut self.store,
            (input_l, input_r, ctx.sample_rate, ctx.global_sample_index),
        ) {
            Ok((l, r)) => [l, r],
            Err(e) => {
                tracing::error!("JIT execution error: {}", e);
                [0.0, 0.0]
            }
        }
    }
}

pub struct JitCompiler {
    engine: Engine,
    register_map: HashMap<StableId, u32>,
    state_map: HashMap<StableId, u32>,
    parameter_map: HashMap<(StableId, String), usize>,
    next_local: u32,
    next_state_offset: u32,
    next_param_offset: usize,
}

impl JitCompiler {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        let engine = Engine::new(&config).unwrap();
        Self {
            engine,
            register_map: HashMap::new(),
            state_map: HashMap::new(),
            parameter_map: HashMap::new(),
            next_local: 4,
            next_state_offset: 0,
            next_param_offset: 131072, // Param block starts after state and scratch
        }
    }

    pub fn compile_runner(&mut self, runner: &crate::DspRunner) -> anyhow::Result<JitProgram> {
        let mut module = WasmModule::new();
        let mut types = TypeSection::new();
        let mut functions = FunctionSection::new();
        let mut exports = ExportSection::new();
        let mut code = CodeSection::new();
        let mut memory = MemorySection::new();

        types.ty().function(vec![ValType::F32], vec![ValType::F32]);
        types.ty().function(
            vec![ValType::F32, ValType::F32, ValType::F32, ValType::I64],
            vec![ValType::F32, ValType::F32],
        );
        types.ty().function(
            vec![
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
            ],
            vec![],
        );

        memory.memory(wasm_encoder::MemoryType {
            minimum: 4, // 256KB
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        let mut imports = wasm_encoder::ImportSection::new();
        imports.import("host", "sin", wasm_encoder::EntityType::Function(0));
        imports.import("host", "tanh", wasm_encoder::EntityType::Function(0));
        imports.import("host", "call_legacy", wasm_encoder::EntityType::Function(2));
        imports.import("host", "call_mna", wasm_encoder::EntityType::Function(2));

        exports.export("process", wasm_encoder::ExportKind::Func, 4);
        exports.export("memory", wasm_encoder::ExportKind::Memory, 0);

        let mut func = Function::new(vec![(1024, ValType::F32)]);
        let graph = runner.get_graph();
        let mut legacy_nodes = Vec::new();
        let mut legacy_node_ids = Vec::new();
        let mut configs = Vec::new();
        let scratch_in = 65536;
        let scratch_out = 65536 + 1024;

        for (id, _) in &runner.nodes {
            self.register_map.insert(*id, self.next_local);
            self.next_local += 1;
            if let Some(node_ir) = graph.nodes.get(id) {
                if node_ir.kind == NodeKind::Source || node_ir.kind == NodeKind::InputProxy {
                    // Input nodes are mapped to input parameters (local 0 and 1)
                }
                if let Some(name) = node_ir.config.get("name").and_then(|v| v.as_string()) {
                    if name == "Oscillator" {
                        self.state_map.insert(*id, self.next_state_offset);
                        self.next_state_offset += 4;
                    }
                }
            }
        }

        for (id, node_impl) in &runner.nodes {
            let out_reg = *self.register_map.get(id).unwrap();
            let mut natively_lowered = false;

            if let Some(node_ir) = graph.nodes.get(id) {
                let name = node_ir.config.get("name").and_then(|v| v.as_string());
                match name.as_deref().map(|s| s.as_str()) {
                    Some("Oscillator") => {
                        natively_lowered = true;
                        let state_offset = *self.state_map.get(id).unwrap();
                        let param_offset = self.next_param_offset;
                        self.parameter_map
                            .insert((*id, "frequency".to_string()), param_offset);
                        self.next_param_offset += 4;

                        func.instruction(&Instruction::I32Const(state_offset as i32));
                        func.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));
                        func.instruction(&Instruction::LocalTee(1023));
                        func.instruction(&Instruction::F32Const(2.0 * std::f32::consts::PI));
                        func.instruction(&Instruction::F32Mul);
                        func.instruction(&Instruction::Call(0)); // host.sin
                        func.instruction(&Instruction::LocalSet(out_reg));

                        func.instruction(&Instruction::I32Const(state_offset as i32));
                        func.instruction(&Instruction::LocalGet(1023));

                        // Load frequency from parameter memory
                        func.instruction(&Instruction::I32Const(param_offset as i32));
                        func.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));

                        func.instruction(&Instruction::LocalGet(2)); // sample_rate
                        func.instruction(&Instruction::F32Div);
                        func.instruction(&Instruction::F32Add);
                        func.instruction(&Instruction::LocalTee(1023));
                        func.instruction(&Instruction::LocalGet(1023));
                        func.instruction(&Instruction::F32Trunc);
                        func.instruction(&Instruction::F32Sub);
                        func.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));
                    }
                    Some("Gain") => {
                        natively_lowered = true;
                        let param_offset = self.next_param_offset;
                        self.parameter_map
                            .insert((*id, "gain".to_string()), param_offset);
                        self.next_param_offset += 4;

                        func.instruction(&Instruction::LocalGet(self.find_input_reg(graph, id)));

                        // Load gain from parameter memory
                        func.instruction(&Instruction::I32Const(param_offset as i32));
                        func.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));

                        func.instruction(&Instruction::F32Mul);
                        func.instruction(&Instruction::LocalSet(out_reg));
                    }
                    Some("Add") => {
                        natively_lowered = true;
                        let in_regs = self.find_all_input_regs(graph, id);
                        if in_regs.is_empty() {
                            func.instruction(&Instruction::F32Const(0.0));
                        } else {
                            func.instruction(&Instruction::LocalGet(in_regs[0]));
                            for &r in &in_regs[1..] {
                                func.instruction(&Instruction::LocalGet(r));
                                func.instruction(&Instruction::F32Add);
                            }
                        }
                        func.instruction(&Instruction::LocalSet(out_reg));
                    }
                    Some("Multiply") => {
                        natively_lowered = true;
                        let in_regs = self.find_all_input_regs(graph, id);
                        if in_regs.len() < 2 {
                            func.instruction(&Instruction::F32Const(0.0));
                        } else {
                            func.instruction(&Instruction::LocalGet(in_regs[0]));
                            for &r in &in_regs[1..] {
                                func.instruction(&Instruction::LocalGet(r));
                                func.instruction(&Instruction::F32Mul);
                            }
                        }
                        func.instruction(&Instruction::LocalSet(out_reg));
                    }
                    _ if node_ir.kind == NodeKind::Source
                        || node_ir.kind == NodeKind::InputProxy =>
                    {
                        natively_lowered = true;
                        // Map Input to local 0 (L) or 1 (R)
                        let channel = node_ir
                            .config
                            .get("channel")
                            .and_then(|v| v.as_float())
                            .unwrap_or(0.0) as u32;
                        func.instruction(&Instruction::LocalGet(channel.min(1)));
                        func.instruction(&Instruction::LocalSet(out_reg));
                    }
                    _ if node_ir.kind == NodeKind::Sink
                        || node_ir.kind == NodeKind::OutputProxy =>
                    {
                        natively_lowered = true;
                        func.instruction(&Instruction::LocalGet(self.find_input_reg(graph, id)));
                        func.instruction(&Instruction::LocalSet(out_reg));
                    }
                    _ => {}
                }
            }

            if !natively_lowered {
                let node_idx = legacy_nodes.len() as i32;
                legacy_nodes.push(dyn_clone::clone_box(&**node_impl));
                legacy_node_ids.push(*id);
                configs.push(graph.nodes.get(id).unwrap().config.clone());
                let in_regs = self.find_all_input_regs(graph, id);
                for (i, &reg) in in_regs.iter().enumerate() {
                    func.instruction(&Instruction::I32Const(scratch_in as i32 + (i as i32 * 4)));
                    func.instruction(&Instruction::LocalGet(reg));
                    func.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }));
                }
                func.instruction(&Instruction::I32Const(node_idx));
                func.instruction(&Instruction::I32Const(scratch_in as i32));
                func.instruction(&Instruction::I32Const(in_regs.len() as i32));
                func.instruction(&Instruction::I32Const(scratch_out as i32));
                func.instruction(&Instruction::I32Const(1));

                let name = graph
                    .nodes
                    .get(id)
                    .unwrap()
                    .config
                    .get("name")
                    .and_then(|v| v.as_string());
                if name.as_deref().map(|s| s.as_str()) == Some("CircuitModule") {
                    func.instruction(&Instruction::Call(3)); // host.call_mna
                } else {
                    func.instruction(&Instruction::Call(2)); // host.call_legacy
                }

                func.instruction(&Instruction::I32Const(scratch_out as i32));
                func.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                func.instruction(&Instruction::LocalSet(out_reg));
            }
        }

        if let Some((last_id, _)) = runner.nodes.last() {
            let last_reg = *self.register_map.get(last_id).unwrap();
            func.instruction(&Instruction::LocalGet(last_reg));
            func.instruction(&Instruction::LocalGet(last_reg));
        } else {
            func.instruction(&Instruction::F32Const(0.0));
            func.instruction(&Instruction::F32Const(0.0));
        }
        func.instruction(&Instruction::End);

        functions.function(1);
        code.function(&func);
        module.section(&types);
        module.section(&imports);
        module.section(&functions);
        module.section(&memory);
        module.section(&exports);
        module.section(&code);

        let mut prog = JitProgram::new(
            &self.engine,
            &module.finish(),
            legacy_nodes,
            legacy_node_ids,
            configs,
            self.parameter_map.clone(),
        )?;

        // Initialize default parameters
        for (id, _node_impl) in &runner.nodes {
            if let Some(node_ir) = graph.nodes.get(id) {
                for (key, val) in &node_ir.config {
                    if let Some(f) = val.as_float() {
                        prog.set_parameter(id, key, f as f32);
                    }
                }
            }
        }

        Ok(prog)
    }

    fn find_input_reg(&self, graph: &dirtydata_core::ir::Graph, node_id: &StableId) -> u32 {
        for edge in graph.edges.values() {
            if edge.target.node_id == *node_id {
                if let Some(&src) = self.register_map.get(&edge.source.node_id) {
                    return src;
                }
            }
        }
        0
    }
    fn find_all_input_regs(
        &self,
        graph: &dirtydata_core::ir::Graph,
        node_id: &StableId,
    ) -> Vec<u32> {
        let mut regs = Vec::new();
        for edge in graph.edges.values() {
            if edge.target.node_id == *node_id {
                if let Some(&src) = self.register_map.get(&edge.source.node_id) {
                    regs.push(src);
                }
            }
        }
        regs
    }
}
