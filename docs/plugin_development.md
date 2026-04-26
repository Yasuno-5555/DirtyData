# DirtyData Plugin Development Guide

Welcome to the DirtyData ecosystem. This guide will help you build custom DSP nodes using the **DirtyData SDK**.

## 1. Prerequisites

- [Rust](https://www.rust-lang.org/) installed.
- `wasm32-unknown-unknown` target for Rust:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

## 2. Creating your Plugin Project

Create a new library crate:
```bash
cargo new my-awesome-dsp --lib
cd my-awesome-dsp
```

Add `dirtydata-sdk` to your `Cargo.toml`:
```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
dirtydata-sdk = { git = "https://github.com/Yasuno-5555/DirtyData" }
```

## 3. Implementing the DSP

In `src/lib.rs`:

```rust
use dirtydata_sdk::{DspPlugin, declare_plugin};

#[derive(Default)]
pub struct MyPlugin {
    gain: f32,
}

impl DspPlugin for MyPlugin {
    fn set_parameter(&mut self, id: u32, value: f32) {
        if id == 0 { self.gain = value; }
    }

    fn process(&mut self, in_l: f32, in_r: f32) -> [f32; 2] {
        [in_l * self.gain, in_r * self.gain]
    }
}

declare_plugin!(MyPlugin);
```

## 4. Building for DirtyData

Compile your plugin to WASM:
```bash
cargo build --target wasm32-unknown-unknown --release
```

## 5. Installation

Copy the generated `.wasm` file to the DirtyData plugin directory:
- **macOS/Linux**: `~/.dirtydata/plugins/`
- **Windows**: `%APPDATA%\DirtyData\plugins\`

## 6. Using via CLI
 
 1. Initialize your project: `dirty init`
 2. Add your WASM node to the topology:
    ```bash
    # Add a custom node via DSL or direct JSON edit
    # Point the 'path' config to your .wasm file or its BLAKE3 hash in CAS
    ```
 3. Verify and Audit: `dirty doctor`
 4. Compile to VST3: `dirty build --target vst3`

---

### SSS+: Advanced Integration

For "Experimental" status nodes, ensure you provide a **Confidence Metadata** block in your documentation to help the Constraint Engine verify your behavior.
