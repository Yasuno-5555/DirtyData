# DirtyRack Developer Guide

Welcome to the DirtyRack developer ecosystem. This guide covers everything you need to build, test, and distribute modules for the most deterministic modular synthesizer in the universe.

## 1. Core Philosophy: The Constitution

Every DirtyRack module is a "forensic artifact." To maintain the integrity of the Merkle DAG (patch history), all modules must adhere to these rules:

1.  **Bit-Perfect Determinism**: Given the same input samples, parameters, and `project_seed`, your module **must** produce identical output on every machine (Windows, Mac, Linux).
2.  **No Side Effects**: Never use `std::time`, `rand::thread_rng()`, or file I/O inside the `process()` loop. Use the provided `RackProcessContext`.
3.  **Real-time Safe**: No dynamic memory allocation (`Vec::new()`, `Box::new()`, etc.) or blocking locks during processing.
4.  **16-Voice Polyphony**: DirtyRack is 16-channel by default. Always process 16 channels or use SIMD (`f32x4`) for performance.

## 2. Setting Up Your Environment

You need the Rust toolchain installed.

```bash
# Add the Wasm target (Highly Recommended)
rustup target add wasm32-wasip1
```

## 3. Creating a Module (Wasm Target)

Wasm is the preferred way to distribute modules. It is safe, cross-platform, and runs in the `wasmtime` sandbox.

### Step 1: Initialize Project
```bash
cargo new my-chaos-gen --lib
cd my-chaos-gen
```

### Step 2: Cargo.toml
```toml
[package]
name = "my-chaos-gen"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"] # Necessary for Wasm or Shared Libs

[dependencies]
dirtyrack-sdk = { path = "../DirtyRack/crates/dirtyrack-sdk" } # Or from crates.io
```

### Step 3: Implementation (`src/lib.rs`)
```rust
use dirtydata_sdk::{DspPlugin, declare_plugin};

#[derive(Default)]
struct MyChaosGen {
    phase: f32,
    freq: f32,
}

impl DspPlugin for MyChaosGen {
    fn init(&mut self, _sample_rate: f32) {
        self.freq = 440.0;
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        if id == 0 {
            self.freq = 20.0 + value * 2000.0; // Simple mapping
        }
    }

    fn process(&mut self, _in_l: f32, _in_r: f32) -> [f32; 2] {
        self.phase += self.freq / 44100.0; // Ideally use stored sample_rate
        if self.phase > 1.0 { self.phase -= 1.0; }
        
        let out = (self.phase * 2.0 * 3.141592).sin();
        [out, out]
    }
}

// Export the Wasm entry points
declare_plugin!(MyChaosGen);
```

### Step 4: Build for Wasm
```bash
cargo build --target wasm32-wasip1 --release
```
The resulting `.wasm` file can be loaded directly into DirtyRack via a Foreign Node.

## 4. Using the Dirty CLI

The `dirty` CLI tool is your companion for forensic audio engineering.

### Initializing a Project
```bash
dirty init my_song
cd my_song
```

### Applying Patches
You can write high-level JSON patches to build your rack:
```bash
dirty patch my_patch.json
```

### Mutating Parameters
Use the evolution engine to find new sounds:
```bash
dirty log --graph # Get the Node ID
dirty mutate <NODE_ID> --level wild --epochs 100
dirty patch patch_<HASH>.json # Apply the mutation
```

### Auditing
Verify that your project hasn't been tampered with and is still deterministic:
```bash
dirty doctor
```

## 5. Advanced: Forensic Data

To help users understand "why" a sound changed, implement `get_forensic_data`. This allows the DirtyRack GUI to show internal state like filter saturation levels or hidden modulation nodes.

```rust
fn get_forensic_data(&self) -> Option<ForensicData> {
    let mut data = ForensicData::default();
    data.internal_state_summary = format!("Oscillator Phase: {:?}", self.phase);
    Some(data)
}
```

---

DirtyRack is more than a synth; it's a record of sound. Happy hacking.
