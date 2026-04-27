# DirtyRack Third-Party Module SDK Documentation

Welcome to the "Deterministic Universe" of DirtyRack. This SDK is a toolbox for developing modular synth modules that are reproducible down to the bit level.

## Quick Start

Read the [Tutorial: Creating Your First Module](Tutorial_Creating_Modules.md) and build your first module in 5 minutes.

## 1. The Constitution

The following constraints are absolute when writing DirtyRack modules:

- **Complete Determinism**: Always generate the same output from the same input and seed. Do not rely on `std::time` or `/dev/random`.
- **NO-ALLOC**: Dynamic memory allocation (e.g., `Vec`, `Box`, `HashMap`) is prohibited within the `process` loop.
- **Use libm**: Use `libm` instead of `std` for mathematical functions to ensure consistent floating-point behavior across platforms.
- **Imperfection Integration**: Use the `ImperfectionData` provided by `RackProcessContext` to instill appropriate "fluctuation" and "individuality" into each of the 16 voices.
- **Forensic Transparency**: Implement `get_forensic_data` to expose the module's internal state to the GUI.

## 2. Core Interface

All modules must implement the `RackDspNode` trait.

```rust
pub trait RackDspNode: Send + Sync {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        ctx: &RackProcessContext,
    );

    /// Reporting forensic data (for Drift Inspector)
    fn get_forensic_data(&self) -> Option<ForensicData> { None }
    
    // ... other methods for persistence
}
```

### RackProcessContext
Contains critical metadata during execution:
- `aging`: Global aging parameter (0.0..1.0).
- `imperfection`: `personality` (static individual difference) and `drift` (dynamic thermal fluctuation) for 16 voices.

## 3. Dynamic Loading Mechanism

DirtyRack looks for a symbol named `get_dirty_module_descriptor`. Use the `export_dirty_module!` macro to register your module.

```rust
use dirtyrack_sdk::*;

struct MyModule { ... }
impl RackDspNode for MyModule { ... }

fn my_descriptor() -> &'static ModuleDescriptor {
    &ModuleDescriptor {
        id: "com.example.my_vco",
        name: "Super VCO",
        version: "1.1.0",
        manufacturer: "Example Corp",
        hp_width: 10,
        params: &[ ... ],
        ports: &[ ... ],
        factory: |sr| Box::new(MyModule::new(sr)),
    }
}

export_dirty_module!(my_descriptor);
```

## 4. Recommended Workflow

1. Add `dirtyrack-sdk` to your `Cargo.toml`.
2. Specify `crate-type = ["cdylib"]`.
3. Build with `cargo build --release`.
4. Place the generated `.so`/`.dll`/`.dylib` into DirtyRack's `modules` folder.

## 5. Auditing & Attribution

To support DirtyRack's powerful forensic auditing features, module developers should consider the following:

- **Parameter Traceability**: All module parameters must be manipulated through the `params` defined in `get_dirty_module_descriptor`. This ensures that `Intent-to-Sound Trace` functions automatically.
- **Providing Forensic Data**: Implement `get_forensic_data` to expose internal non-linear states (e.g., filter saturation levels, oscillator phase drift). This is the key to "cause identification" when deterministic differences occur.
- **Eliminating Nondeterminism**: Using external time or random values will be recorded as a "reality split" in the `Replay Divergence Map`, destroying the reliability of the patch.

---

> [!IMPORTANT]
> "The nondeterminism of a single developer destroys the hash of the entire patch."
> Always write code with reproducibility in mind.
