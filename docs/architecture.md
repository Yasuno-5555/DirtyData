# DirtyRack Architecture

<<<<<<< Updated upstream
DirtyRack employs a multi-layered architecture designed to balance deterministic audio computation with asynchronous visual projection.
=======
This is a technical reference regarding the internal structure (Architecture) of DirtyData.
The system is divided into multiple crates (layers), each with strict roles and boundaries.
>>>>>>> Stashed changes

## 1. Gehenna Engine: Parallel Deterministic DSP (`dirtyrack-modules`)

<<<<<<< Updated upstream
The second-generation engine, responsible for all acoustic operations with bit-perfect reproducibility.

- **16-Channel SIMD Polyphony**: Native support for VCV Rack-compatible 16-channel multiplexed cables. High-density polyphonic operations are executed in parallel using SIMD (`wide::f32x4` x4).
- **Deterministic Voice Drift**: A specialized drift engine simulates analog instability (1/f noise) deterministically. The same seed produces the exact same "analog personality" across all instances.
- **No-Alloc Process Loop**: Completely eliminates memory allocation within the audio callback. Uses pre-allocated topological buffers.

## 2. Forensic Observation & MRI Layer (`dirtyrack-gui`)
=======
Defined in the `dirtydata-core` crate, this is the system's single Source of Truth.

- **`Graph`**: Holds the structure of the entire project. It contains all `Nodes`, `Edges`, `Modulations`, and the history of applied `PatchIds`.
- **`Node`**: Components such as audio sources (`Source`), effects (`Processor`), external plugins (`Foreign`), and **modularized circuits (`CircuitModule`)**.
- **`Edge`**: Connections (routing) between nodes. In addition to normal connections, feedback connections (1-sample delay) are explicitly distinguished.
- **`Modulation`**: "Cable-less" modulation assignments to node parameters (Bitwig style).
- **`ConfigSnapshot`**: Node parameters. `BTreeMap` is used to guarantee deterministic ordering.

It is **forbidden** for the GUI or users to directly rewrite the IR. All state changes must be applied through a `Patch`.
>>>>>>> Stashed changes

The GUI acts as a "Medical Projector" for dissecting the signal chain.

<<<<<<< Updated upstream
- **Triple-Buffer Visual Projection**: The engine writes `VisualSnapshot` data including peak levels, clipping counts, DC offsets, and energy density.
- **Patch MRI Overlay**: Signal pathologies are projected directly onto module faceplates (Clipping Glow, Heatmaps, Aura).
- **Explain Why Engine**: A diagnostic system that correlates engine statistics to human-readable reports, identifying issues like feedback runaway or denormal storms.
- **Provenance Timeline**: Records every committed parameter change and state snapshot into a `CausalityLog` for auditing.

## 3. Plugin Host Integration (`dirtyrack-plugin`)
=======
DirtyData features a branch management system inspired by Git.

- Enables ultra-fast "moving between parallel worlds" by simply switching IR pointers (HEAD and refs) without duplicating physical audio or session files.
- `Storage` manages `.dirtydata/refs/heads/` and `.dirtydata/HEAD`, tracking which `PatchId` ancestry each branch belongs to.
>>>>>>> Stashed changes

Wraps the DirtyRack core into a DAW-compatible plugin via the `nih-plug` framework.

<<<<<<< Updated upstream
- **VST3 / CLAP Support**: Maps MIDI notes and polyphonic modulations from the DAW into internal 16ch signals through the `MidiCvModule`.
- **Headless Mode**: The same deterministic engine operates in GUI-less CLI mode or during background rendering within a DAW.

## 4. The Shared SDK (`dirtyrack-sdk`)

The foundation for blurring the boundary between built-in and third-party modules.

- **Stable C-ABI**: Provides a stable function call interface for dynamically loaded external modules.
- **Common Traits**: Through the `RackDspNode` trait, third-party modules are executed with the exact same priority and precision as built-in ones.

## 5. State Extraction & Preservation

A mechanism to ensure sound does not stop even during a hot-reload of a patch.

- **`extract_state()` / `inject_state()`**: When the module topology is updated, oscillator phases and filter states are transferred between old and new modules sharing the same ID. This allows for continuous performance while reconfiguring the patch.

## 6. DAG-Based Routing

Patches are managed as Directed Acyclic Graphs (DAGs).

- **Topological Sorting**: The processing order is automatically calculated based on cable connections.
- **Sample-Accurate Modulation**: All CV and audio signals are propagated with sample-level precision.
- **Feedback Compensation**: Deterministically manages delays in feedback loops.

## 7. Deterministic Auditing & Intent Layer (`dirtydata-*`)

A meta-layer that manages the "causality" behind audio reality.

- **`dirtydata-observer`**: Monitors deterministic breaks (Divergence) at the sampling level and generates a `DivergenceMap`.
- **`dirtydata-intent`**: Structures user actions as "Intents." Tracks "who changed specific sounds and for what purpose" (Attribution).
- **`dirtydata-runtime`**: Executes ultra-high-speed comparison rendering offline to scientifically extract minute differences between two branches.

---

## Data Flow

```mermaid
graph LR
    User[User Interaction] --> GUI[GUI Projector]
    GUI -->|Topology Update| Engine[Audio Engine]
    Engine -->|Triple-Buffer| VisualData[Visual Snapshot]
    VisualData --> GUI
    Engine -->|Audio Out| Device[Audio Device]
=======
The `dirtydata-runtime` crate converts the IR graph into an actual "audible state".

- **`cpal`**: Communicates directly with the OS audio device and starts a real-time callback thread.
- **`arc-swap`**: Implements lock-free double buffering. When a user adds a new effect via `dirtydata patch apply`, it atomically switches to the new DSP graph pointer safely without ever blocking the audio callback (zero glitches).

## 4. Circuit Module & Mutation History

DirtyData records the evolution of nodes as a "Circuit", not just a list of DSP nodes.

- **`CircuitModule`**: A "predefined circuit" combining multiple basic nodes. It possesses DNA registered in the `CircuitRegistry`.
- **`MutationHistory`**: Records the history of how a circuit has evolved (Mutated).
    - **Tier 1: Safe**: Minute drifts in parameters.
    - **Tier 2: Wild**: Swapping of components.
    - **Tier 3: Radioactive**: Changes to the circuit topology itself.
    - **Tier 4: Forbidden**: Evolution beyond the stability boundary.

## 5. Plugin Sandbox (IPC Boundary)

`dirtydata-host` protects the core system from unstable third-party plugins like VSTs.

- Plugins run as **independent child processes** called `dirtydata-plugin-worker`.
- Audio buffers are passed via RPC communication over `stdin` / `stdout`.
- The sandbox instantly detects if a child process returns `NaN` (NaN Storm) or crashes (Segfault), and safely **falls back to a Frozen Asset (currently a silent buffer)**.

## 6. Observer Daemon

The `dirtydata-observer` and the CLI `daemon` subcommand monitor discrepancies between the system and the "external world" (file system, etc.).

- **Observe before Control**: Recalculates BLAKE3 hashes and timestamps of external audio files (WAV, etc.) before changing the system state.
- **Hot-Reloading**: Real-time detection of changes to `.dirtydata/ir/current.json` using the `notify` crate, automatically updating the audio engine graph.
- If an external file is manually modified, it is immediately detected, and the Confidence Score is dropped to `Suspicious` with a warning.


## 8. The VoiceStack (Polyphony)

DirtyData transcends monophonic limitations to achieve dynamic polyphony.
Internally implemented as replicas of `SubGraph` nodes, where commands (NoteOn/Off) are distributed to specific instances by a voice allocator.

## 9. The Conductor (Sequencer & CV-Command)

DirtyData's sequencer employs the **CV-Command Protocol**, embedding commands directly within the audio signal.

- **Left Channel**: Command code (NoteOn/Off, etc.).
- **Right Channel**: Payload (Note number, velocity, etc.).
Since these are treated as audio signals, "performance information itself" can be modulated and processed by DSP nodes like Delays or LFOs.

## 10. State Preservation (Inception-style Hot-swapping)

Prevents oscillator phases or envelope states from resetting during graph hot-swaps.

- **`extract_state()` / `inject_state()`**: For nodes with matching `StableId` between old and new graphs, internal dynamic states are extracted and injected into the new instance. This maintains audio continuity (Zero-Glitch) while rewriting node configurations during performance.


## Crate Dependencies

```mermaid
graph TD
    CLI[dirty] --> Core[dirtydata-core]
    CLI --> Observer[dirtydata-observer]
    CLI --> Intent[dirtydata-intent]
    CLI --> Runtime[dirtydata-runtime]
    CLI --> Exporter[dirty-exporter]
    CLI --> Mutate[dirty-mutate]
    
    Runtime --> Core
    Runtime --> Host[dirtydata-host]
    
    Host --> Core
>>>>>>> Stashed changes
```
