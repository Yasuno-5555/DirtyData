# Tutorial: Creating Your First DirtyRack Module

In this tutorial, you will learn how to participate in the DirtyRack ecosystem by creating a simple "Gain" module that doubles the input signal.

## Step 1: Create the Project

First, create a new Rust library project.

```bash
cargo new my-dirty-module --lib
cd my-dirty-module
```

## Step 2: Configure Cargo.toml

DirtyRack loads dynamic libraries (.so, .dll, .dylib). You also need to add the SDK to your dependencies.

Open `Cargo.toml` and edit it as follows:

```toml
[package]
name = "my-dirty-module"
version = "0.1.0"
edition = "2021"

[lib]
# IMPORTANT: Specify that the project should be built as a dynamic library
crate-type = ["cdylib"]

[dependencies]
# Use the stable SDK
dirtyrack-sdk = "0.1" 
```

## Step 3: Implement the Code (`src/lib.rs`)

Delete everything in `src/lib.rs` and paste the following code for the "Gain Module."

```rust
use dirtyrack_sdk::*;

// 1. Define the module's data structure
struct MyGainModule {
    // Memory allocation within the process loop is prohibited, 
    // so any necessary buffers must be held here.
}

impl MyGainModule {
    pub fn new(_sample_rate: f32) -> Self {
        Self {}
    }
}

// 2. Implement the DSP logic
impl RackDspNode for MyGainModule {
    fn process(
        &mut self,
        inputs: &[f32],      // Array of input voltages (16 voices)
        outputs: &mut [f32], // Array of output voltages (16 voices)
        params: &[f32],      // Array of knob values
        ctx: &RackProcessContext, // Information on aging, imperfections, etc.
    ) {
        let gain_knob = params[0];
        
        // DirtyRack is always 16ch polyphonic.
        for i in 0..16 {
            // Retrieve voice-specific personality from ctx.imperfection
            let p_offset = ctx.imperfection.personality[i] * 0.05;
            let gain = (gain_knob + p_offset).max(0.0);
            
            outputs[0 * 16 + i] = inputs[0 * 16 + i] * gain;
        }
    }
}

// 3. Define the module's "face" (descriptor)
fn my_descriptor() -> &'static ModuleDescriptor {
    &ModuleDescriptor {
        id: "com.yourname.gain", // Globally unique ID
        name: "My First Gain",   // Name displayed in the browser
        version: "1.1.0",
        manufacturer: "Independent Crafter",
        hp_width: 4,             // Module width (1HP = 5.08mm)
        
        // --- Customize the look! ---
        visuals: ModuleVisuals {
            background_color: [50, 60, 70], // Deep blue-grey
            text_color: [255, 255, 255],    // White text
            accent_color: [0, 255, 150],    // Vivid emerald accent
            panel_texture: PanelTexture::MatteBlack,
        },
        
        // Parameter (knob) definitions
        params: &[
            ParamDescriptor {
                name: "GAIN",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0, max: 2.0, default: 1.0,
                position: [0.5, 0.5], // Position on the faceplate [x, y]
                unit: "x",
            },
        ],
        
        // Port (input/output) definitions
        ports: &[
            PortDescriptor { name: "IN", direction: PortDirection::Input, signal_type: SignalType::Audio, position: [0.5, 0.2] },
            PortDescriptor { name: "OUT", direction: PortDirection::Output, signal_type: SignalType::Audio, position: [0.5, 0.8] },
        ],
        
        // Factory function for DirtyRack to generate the module
        factory: |sr| Box::new(MyGainModule::new(sr)),
    }
}

// 4. Export to the DirtyRack Universe
export_dirty_module!(my_descriptor);
```

## Step 4: Build and Install

Now it's time to build. Make sure to use `--release` to enable optimizations.

```bash
cargo build --release
```

If the build is successful, library files will be generated in the `target/release/` folder:
- macOS: `libmy_dirty_module.dylib`
- Linux: `libmy_dirty_module.so`
- Windows: `my_dirty_module.dll`

Copy this file to the `modules/` folder (create it if it doesn't exist) located in the same directory as the DirtyRack executable.

## Step 5: Verify in DirtyRack

Launch DirtyRack and open the "Add Module" browser. Your **"My First Gain"** should be listed there!

---

## Tips for Next Steps

- **Tuning Imperfections**: Use `ctx.imperfection` to give each voice a subtly different character.
- **Forensic Transparency**: Implement `get_forensic_data` so users can peek at your module's "secrets" through the inspector.
- **The Constitution**: When in doubt, return to the "Constitution" in `docs/SDK_Documentation.md`.
