use dirtydata_core::types::ConfigSnapshot;
use super::base::*;
use dirtydata_host::PluginHost;

pub struct ForeignNode {
    host: Option<PluginHost>,
    plugin_name: String,
    buffer_size: usize,
    in_buffer: Vec<f32>,
    out_buffer: Vec<f32>,
    buffer_idx: usize,
    has_crashed: bool,
}

impl Clone for ForeignNode {
    fn clone(&self) -> Self {
        Self {
            host: None,
            plugin_name: self.plugin_name.clone(),
            buffer_size: self.buffer_size,
            in_buffer: self.in_buffer.clone(),
            out_buffer: self.out_buffer.clone(),
            buffer_idx: self.buffer_idx,
            has_crashed: self.has_crashed,
        }
    }
}

impl ForeignNode {
    pub fn new(plugin_name: String, buffer_size: usize) -> Self {
        Self {
            host: None,
            plugin_name,
            buffer_size,
            in_buffer: vec![0.0; buffer_size * 2],
            out_buffer: vec![0.0; buffer_size * 2],
            buffer_idx: 0,
            has_crashed: false,
        }
    }
}

impl DspNode for ForeignNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        if self.has_crashed { return; }
        let host = self.host.get_or_insert_with(|| PluginHost::new(&self.plugin_name, self.buffer_size).expect("Failed to load plugin"));
        if inputs.len() >= 2 {
            self.in_buffer[self.buffer_idx] = inputs[0];
            self.in_buffer[self.buffer_idx + self.buffer_size] = inputs[1];
        }
        outputs[0][0] = self.out_buffer[self.buffer_idx];
        outputs[0][1] = self.out_buffer[self.buffer_idx + self.buffer_size];
        self.buffer_idx += 1;
        if self.buffer_idx >= self.buffer_size {
            if let Err(e) = host.process(&self.in_buffer, &mut self.out_buffer) {
                tracing::error!("Plugin {} crashed: {}", self.plugin_name, e);
                self.has_crashed = true;
            }
            self.buffer_idx = 0;
        }
    }
    fn update_parameter(&mut self, param: &str, value: f32) {
        if let Some(host) = &mut self.host {
            if let Ok(id) = param.parse::<u32>() { let _ = host.set_parameter(id, value); }
        }
    }
}

#[derive(Clone)]
pub struct InputProxyNode { pub value: f32 }
impl InputProxyNode { pub fn new() -> Self { Self { value: 0.0 } } }
impl DspNode for InputProxyNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        outputs[0] = [self.value, self.value];
    }
    fn update_parameter(&mut self, _param: &str, value: f32) { self.value = value; }
}

#[derive(Clone)]
pub struct OutputProxyNode;
impl OutputProxyNode { pub fn new() -> Self { Self } }
impl DspNode for OutputProxyNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        let val = inputs.get(0).cloned().unwrap_or(0.0);
        outputs[0] = [val, val];
    }
}

#[derive(Clone)]
pub struct SubGraphNode {
    runner: Option<crate::DspRunner>,
    last_graph_hash: String,
}

impl SubGraphNode {
    pub fn new() -> Self {
        Self { runner: None, last_graph_hash: String::new() }
    }
}

impl DspNode for SubGraphNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let graph_json = config.get("graph_json").and_then(|v| v.as_string()).map(|s| s.as_str()).unwrap_or("");
        let hash = blake3::hash(graph_json.as_bytes()).to_string();

        if hash != self.last_graph_hash && !graph_json.is_empty() {
            if let Ok(graph) = serde_json::from_str::<dirtydata_core::ir::Graph>(&graph_json) {
                self.runner = Some(crate::DspRunner::new(graph, None, ctx.sample_rate));
                self.last_graph_hash = hash;
            }
        }

        if let Some(runner) = &mut self.runner {
            let mut proxy_ids = Vec::new();
            for (id, n) in &runner.get_graph().nodes {
                if n.kind == dirtydata_core::types::NodeKind::InputProxy {
                    proxy_ids.push(*id);
                }
            }
            for (id, node) in runner.nodes_mut() {
                if proxy_ids.contains(id) {
                    node.update_parameter("value", inputs.get(0).cloned().unwrap_or(0.0));
                }
            }
            
            let sub_out = runner.process_sample(ctx);
            outputs[0] = sub_out;
        } else {
            for o in outputs { *o = [0.0, 0.0]; }
        }
    }
}

pub struct WasmNode {
    instance: Option<wasmtime::Instance>,
    store: Option<wasmtime::Store<()>>,
    process_fn: Option<wasmtime::TypedFunc<(f32, f32), i64>>,
    failed: bool,
}

impl Clone for WasmNode {
    fn clone(&self) -> Self {
        Self {
            instance: None,
            store: None,
            process_fn: None,
            failed: self.failed,
        }
    }
}

impl WasmNode {
    pub fn new() -> Self {
        Self { instance: None, store: None, process_fn: None, failed: false }
    }

    fn init(&mut self, path: &str) -> anyhow::Result<()> {
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::from_file(&engine, path)?;
        let mut store = wasmtime::Store::new(&engine, ());
        let instance = wasmtime::Instance::new(&mut store, &module, &[])?;
        let process_fn = instance.get_typed_func::<(f32, f32), i64>(&mut store, "process")?;
        
        self.instance = Some(instance);
        self.store = Some(store);
        self.process_fn = Some(process_fn);
        Ok(())
    }
}

impl DspNode for WasmNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, _ctx: &ProcessContext) {
        if self.instance.is_none() && !self.failed {
            if let Some(path) = config.get("path").and_then(|v| v.as_string()) {
                if let Err(e) = self.init(path) {
                    eprintln!("Failed to init WasmNode: {}", e);
                    self.failed = true;
                }
            }
        }

        if let (Some(store), Some(f)) = (self.store.as_mut(), self.process_fn.as_mut()) {
            for i in 0..outputs.len() {
                let in_l = inputs.get(i * 2).cloned().unwrap_or(0.0);
                let in_r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0);
                
                match f.call(&mut *store, (in_l, in_r)) {
                    Ok(res) => {
                        let out_l = f32::from_bits((res >> 32) as u32);
                        let out_r = f32::from_bits(res as u32);
                        outputs[i] = [out_l, out_r];
                    }
                    Err(_) => {
                        outputs[i] = [in_l, in_r];
                    }
                }
            }
        } else {
            for i in 0..outputs.len() {
                outputs[i] = [
                    inputs.get(i * 2).cloned().unwrap_or(0.0),
                    inputs.get(i * 2 + 1).cloned().unwrap_or(0.0)
                ];
            }
        }
    }
}
