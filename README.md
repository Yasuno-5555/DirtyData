<<<<<<< Updated upstream
# DirtyRack

**Deterministic Eurorack Simulator & Modular DSP Engine**

DirtyRack is a **high-precision deterministic Eurorack simulator** that merges bit-perfect reproducibility with physical modular interaction. It is designed for artists and engineers who love the "chaos" and "fluctuations" of analog but demand digital "Accountability."

More than just a synthesizer, it is a **Forensic Audio Infrastructure** that treats sound as a diagnosable medical case. It freezes the entire state of a patch, performance gestures, and the flow of time, making them verifiable via hash values.

## Core Design Philosophy

*   **Deterministic Chaos (Gehenna Engine)**: Handles chaos attractors and non-linear feedback while guaranteeing 1-bit accuracy for the same seed and input.
*   **Massive Polyphony (16ch)**: Native support for VCV Rack-compatible 16-channel polyphonic cables. A single connection yields orchestral density.
*   **Acoustic Forensics**: A forensic layer that proves "why this sound happened." The **Patch MRI** allows real-time dissection of internal thermal states and individual component variances.
*   **The Spec is Absolute**: Adheres to the **.dirtyrack Open Specification**, ensuring that your sound design is portable, verifiable, and permanent.
*   **Forensic Observation**: Implementations like the **Divergence Map** and **Provenance Timeline** allow users to trace the causality of sound design choices.

## Key Features

1.  **Gehenna Parallel Engine**: A second-generation parallel DSP engine optimized for SIMD, delivering deterministic analog "personality" across 16 voices.
2.  **Patch MRI (Pathology Scan)**: Visualize signal trauma in real-time. Detect clipping (Glow), energy density (Heatmap), and DC Drift (Aura) directly on module faceplates.
3.  **Provenance Timeline**: A chronological map of every parameter change and snapshot. Traces the "intent" behind the sound.
4.  **Forensic Inspector**: Deeply analyze the internal state of modules. Includes an **Explain Why** button that generates medical-style diagnostic reports for signal abnormalities.
5.  **.dirtyrack Spec v1.0**: A standardized format for patches (`.dirty`) and audit certificates (`.dirty.cert`). Enables "Acoustic Notarization."
6.  **Differential Audit**: Compare two snapshots or renders at sample precision. Detect exactly what changed between iterations.
7.  **Verification CLI**: A command-line utility for CI/CD and production environments. Run `dirty verify` to ensure the integrity of your audio output.

### Distribution & Formats

- **Standalone App**: Provides a `.app` bundle for macOS. Ready to use via drag-and-drop to `/Applications`.
- **VST3 / CLAP Plugin**: Operates as a 16ch polyphonic modular inside DAWs (Ableton, Bitwig, Reaper, etc.).
- **CLI Tool**: A command-line interface for deterministic offline rendering and hash verification.

## Quick Start

### Standalone (macOS)
```bash
# Copy DirtyRack.app from the root to /Applications
open ./DirtyRack.app
```

### Plugin Deployment
Place the generated `DirtyRack.clap` from the root into your plugin folder.

**To use as VST3**:
1. Create a directory structure: `DirtyRack.vst3/Contents/MacOS/`.
2. Copy and rename `DirtyRack.clap` to `DirtyRack` inside that directory.

## Project Structure

```text
DirtyRack/
├── crates/
│   ├── dirtyrack-sdk/      # SDK for third-party developers. Core traits and SIMD utilities.
│   ├── dirtyrack-modules/  # Deterministic DSP module arsenal (VCO, VCF, Chaos, etc.)
│   ├── dirtyrack-gui/      # egui-based "Projector." Triple-Buffer synchronization.
│   └── dirtyrack-core/     # Deterministic foundation. DAG engine managing causality.
├── docs/                   # SDK documentation, architecture, philosophy.
└── modules/                # Directory for third-party dynamic libraries (.so, .dll).
```

## Documentation

- [Design Philosophy](docs/design_philosophy.md)
- [Architecture](docs/architecture.md)
- [SDK Documentation](docs/SDK_Documentation.md)
- [Creating Your First Module (Tutorial)](docs/Tutorial_Creating_Modules.md)
- [Japanese README (日本語版)](docs/README_JP.md)

## License

MIT License
=======
# DirtyData: Headless Forensic Audio Workbench

> "GUI is for tourists. CLI is for the Judiciary."

DirtyData is a headless forensic audio engine designed for deterministic sound reconstruction, semantic versioning, and evolutionary patching. It treats sound as a verifiable lineage of intent rather than a transient buffer.

[日本語版はこちら (Japanese version)](README.ja.md)

---

## 🏛 The Architecture

### Layer 1: The Nervous System (`dirty-core`)
The invisible backbone of forensic sound.
- **Forensic IR**: Deterministic topology representation.
- **Merkle DAG**: Cryptographically verifiable history.
- **Semantic Merge**: Conflict resolution based on intent priority.
- **JIT Compiler**: High-performance DSP execution.

### Layer 2: The Judiciary (`dirty` CLI)
The primary interface for the forensic engineer. Designed for Neovim + Tmux workflows.

```bash
# Initialize a new forensic record
dirty init

# Audit the forensic integrity
dirty doctor

# View the semantic lineage and intent chain
dirty log --graph

# Headless batch mutation (Evolutionary Search)
dirty mutate tb303 --level radioactive --epochs 10000

# Transmute IR to a standalone Vst3/Clap plugin
dirty build --target vst3
```

## 🛠 Workflow

1.  **Edit**: Modify `topology.ir` or `.dsl` files directly in your preferred editor.
2.  **Verify**: Run `dirty verify` to ensure spec adherence and hash integrity.
3.  **Commit**: Apply changes via `dirty patch` to create a new Merkle link.
4.  **Manifest**: Build production-ready binaries with `dirty build`.

## 📜 Specifications
- **RFC 001**: [The Forensic Standard](docs/RFC_001_DIRTYDATA_SPEC.md)
- **Architecture**: [Core Design](docs/architecture.md)
- **Plugin Development**: [SDK Guide](docs/plugin_development.md)
>>>>>>> Stashed changes
