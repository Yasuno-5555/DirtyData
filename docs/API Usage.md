# DirtyData API Usage Guide

This guide explains how to use the `dirtydata-sdk` to interact with the DirtyData engine, manage workspaces, and build custom DSP nodes.

## 1. Installation

Add `dirtydata-sdk` to your `Cargo.toml`:

```toml
[dependencies]
dirtydata-sdk = { path = "../path/to/dirtydata-sdk" }
```

## 2. Basic Workflow

The typical workflow involves opening a **Workspace**, defining a **Graph** via **Patches**, and running it on the **AudioEngine**.

### Initialize Workspace

A workspace manages the state and history (Forensic Records) of a project.

```rust
use dirtydata_sdk::Workspace;

fn main() -> anyhow::Result<()> {
    // Open or create a workspace in the current directory
    let mut ws = Workspace::open(".")?;
    
    println!("Workspace Root: {:?}", ws.root_hash());
    Ok(())
}
```

### Building a Graph

Use `NodeFactory` to create nodes and `Patch` to assemble them.

```rust
use dirtydata_sdk::{NodeFactory, Patch, Operation};
use dirtydata_sdk::types::StableId;

fn create_simple_synth() -> Patch {
    let osc_id = StableId::new();
    let sink_id = StableId::new();
    
    let mut patch = Patch::new();
    
    // 1. Add Nodes
    patch.add_operation(Operation::AddNode(NodeFactory::oscillator(440.0)));
    patch.add_operation(Operation::AddNode(NodeFactory::bit_crush(8.0, 2.0)));
    
    // 2. Connect them
    // (Actual API might use specific connection methods depending on crate versions)
    
    patch
}
```

## 3. Real-time Audio Execution

`AudioEngine` handles the low-level interaction with the sound device and provides glitch-free graph hotswapping.

```rust
use dirtydata_sdk::{AudioEngine, SharedState};
use std::sync::Arc;

fn start_engine() {
    let shared_state = Arc::new(SharedState::new());
    let (midi_tx, midi_rx) = crossbeam_channel::unbounded();
    
    let engine = AudioEngine::new(shared_state, midi_rx);
    
    // Update parameters in real-time
    // engine.update_parameter(node_id, "frequency".to_string(), 880.0);
}
```

## 4. Creating Custom DSP Nodes (Plugins)

You can extend DirtyData by implementing the `DspPlugin` trait.

```rust
use dirtydata_sdk::{DspPlugin, declare_plugin};

#[derive(Default)]
struct MyDistortion {
    drive: f32,
}

impl DspPlugin for MyDistortion {
    fn init(&mut self, sample_rate: f32) {
        println!("Initialized at {}Hz", sample_rate);
    }

    fn process(&mut self, in_l: f32, in_r: f32) -> [f32; 2] {
        let out_l = (in_l * self.drive).tanh();
        let out_r = (in_r * self.drive).tanh();
        [out_l, out_r]
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        if id == 0 { self.drive = value; }
    }
}

// Export as a WASM-compatible plugin
declare_plugin!(MyDistortion);
```

## 5. Semantic Merging

DirtyData allows merging different project states while respecting "Intents".

```rust
use dirtydata_sdk::merge::SemanticMerge;

fn merge_collaboration(ws: &mut Workspace, incoming_patch: Patch) -> anyhow::Result<()> {
    SemanticMerge::run(ws, incoming_patch)?;
    Ok(())
}
```
