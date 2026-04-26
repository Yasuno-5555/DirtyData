<<<<<<< Updated upstream
# DirtyRack Design Philosophy

DirtyRack is more than just a Eurorack simulator. It is an **experimental ground for harmonizing "absolute digital determinism" with "sensual analog uncertainty."**

## 1. The Constitution of Determinism

In the universe of DirtyRack, chance does not exist.

- **Duty of Reproducibility**: Every sound and every fluctuation is computable and reproducible. We guarantee that if the patch hash matches, the exact same sound will be produced on any machine in the world, even ten years from now.
- **Forensic Validation**: Every moment of the creative process is recorded as scientific evidence (Forensic Evidence).

## 2. Massive Polyphony (16 Voices)

DirtyRack standardizes "polyphonic cables" that bundle 16 channels into a single connection.

- **Signal Multiplexing**: With a VCV Rack-compatible 16ch architecture, a single patch achieves the density of an orchestra.
- **Voice-level Individuality**: Individual voices behave differently based on internal component variances (Personality), even with identical parameters.

## 3. Forensic Instrumentation

The mission of DirtyRack's Observation Layer is to scientifically prove "why this sound happened."

- **Visual Provenance**: Analog imperfections are not treated as "hidden flavors" but as visualized "personalities." The Drift Inspector dissects internal thermal states and individual variances in real-time.
- **Replay Divergence Map (In Production)**: If a hash mismatch occurs due to environmental or branch differences, this tool identifies exactly which node and which sample caused the "reality split." It is a "map" that forbids the collapse of determinism.
- **Intent-to-Sound Trace (In Production)**: Traces audio artifacts (like clipping or distortion) back to parameter changes, the patches that caused them, and ultimately to the user's operation intent (Intent) or commit hash.
- **Causal Analysis**: Goes beyond simple A/B comparison to quantitatively analyze the "causal impact" of parameter differences on audio characteristics (such as transients).

## 4. Controlled Instability

"Perfect digital" can sometimes mean musical death.

- **Aging Knob**: Control everything from "factory-new" shine to 20 years of "vintage decay" with a single knob. Drift, leakage, and response delays all "degrade" based on deterministic formulas.
- **Drift as Choice**: Redefines the fluctuations caused by thermal noise or component aging in analog circuits as "reproducible fluctuations" based on seeds rather than randomness.

## 5. Trust in Developers

Publishing the SDK and making the internal structure transparent is an invitation to transform users from "consumers" into "creators."

- **Open Source, Strict Rules**: Creative freedom is welcomed, but breaking the Constitution of Determinism is not permitted. This strict discipline is what guarantees the value of the DirtyRack platform.

---

> [!NOTE]
> Infusing the heat of analog into the cold sequences of 0s and 1s.
> DirtyRack is the answer to that impossible question.
=======
# DirtyData Core Design Philosophy

DirtyData is not "software for making music." It is a "runtime environment for providing accountability to the inherently chaotic act of creation."

This document explains the core ideas that support the system.

## 1. Don't just have State. Have History.

Traditional DAWs (Digital Audio Workstations) only save the "current state" in session files (.als, .logicx, etc.). Consequently, the causal relationship of "why is this parameter set like this?" is lost over time, turning the project into a giant black box.

In DirtyData, **every change is expressed as a patch (PatchSet) and applied in order via a Directed Acyclic Graph (DAG)**.
As a result, the **Provenance** of "how did this sound come to be, and from which patches and intentions?" is always provable.

## 2. Save "Meaning" instead of Operations

DirtyData is not a *Patch Manager*, but a *Meaning Manager*.

Instead of recording internal commands like "boost 2kHz on EQ by 3dB," it saves the **Intent** of "**boost vocal presence**."
Music is not an optimization problem, but a constrained compromise problem. Intents are linked to the graph as Constraints:

- `Must`: Do not crush vocal transients
- `Prefer`: Add analog warmth
- `Avoid`: Harsh high-frequency sibilance
- `Never`: Collapse mono compatibility

The system evaluates these Intents upon patch application, allowing you to review "what your past self was trying to achieve" later.

## 3. Observe before Control (Automating "Doubt")

In audio systems, there is no guarantee that values on the GUI match the actual output sound (due to hidden oversampling in plugins, random noise in analog modeling, etc.).

DirtyData **Observes** the external world before attempting to control it.
It calculates file sizes, extensions, and BLAKE3 hashes to constantly evaluate the discrepancy between the system's internal state (expectation) and the external world (reality).

The match rate (Confidence Score) is explicitly shown on the UI as a `Dirty State`.
- `100% Verified`: A state where hashes match perfectly.
- `Suspicious`: A state where external tampering is detected.

While it is impossible to make everything 100% Verified (excluding chaos), DirtyData **quantifies "what is suspicious" and provides visualization for humans to proceed while aware of the risks**.

## 4. Constitution: Explainable or Disposable

The absolute rule in operating DirtyData:

> **Every node, every plugin, and every routing must be fully Explainable for its existence, or otherwise be in a Disposable state where it can be discarded at any time.**

The `dirtydata doctor` command warns about "nodes with unclear reasons for existence (no linked Intent) and low impact" as Disposable Candidates.

## 5. Security Model (Trust Boundaries)

DirtyData strictly defines what can be trusted (Trust Boundary).
The system **does not trust**:

1. **Plugins (VST/AU, etc.)**
2. **Observer (The monitoring tool itself)**
3. **GUI / User**

User-edited DSL, external scripts, third-party extensions, or AI-generated PatchSets all possess a `TrustLevel`.
If an AI arbitrarily performs destructive acts like "inserting 20 limiters on the master bus" (something humans also tend to do), it is treated as `ReviewRequired` or `Quarantined`, preventing unintentional destruction of the production environment.

## 6. Performance Budget

Abandoning the illusion that "CPU is free," we clearly define Performance Budgets for each domain:

- **Sample Domain**: Audio callback (Hard real-time). No allocations. No locks. Monitored by a watchdog with timeouts.
- **Block Domain**: FFT and loudness analysis (Soft real-time).
- **Timeline Domain**: Management of rendering and incremental builds.
- **Background Domain**: Machine learning and batch processing. This processing is **absolutely forbidden** from blocking the Sample Domain.

## 7. Boundary Defense and Fallback (Sandbox)

To control "audible black boxes (VSTs, etc.)," DirtyData executes plugins within a separate process Sandbox (IPC boundary).

Even if a plugin crashes due to a memory access violation (Segfault), only the child process dies; the **DirtyData core remains unharmed**.
If a plugin outputs denormal numbers (NaN Storm), the system detects it at the boundary, instantly mutes the output, and falls back to the last saved safe `Frozen Asset` (such as a pre-rendered WAV).

This is the **true robustness** that DirtyData aims for.

## 8. Visual as Projection (GUI is a "Projection" of the IR)

In DirtyData, the GUI is not the system state itself, but merely a "temporary projection" of the truth (IR) existing in the Core.

- **The Forge UI**: The reason we built our own renderer instead of adopting existing node editor libraries is to allow the UI itself to express "DirtyState (uncertainty)."
- **Shared Visualization**: Waveforms (Oscilloscope) and volume (Meters) are "peeks into the internal operations" of the system, allowing users to verify the behavior of DSP that often becomes a black box, along with reliable history (Provenance).
- **Direct Interaction**: Operations on the GUI (dragging, connecting) are immediately translated and compiled into `UserAction`, carved into history as patches. This allows visual intuition and systemic rigor to coexist without contradiction.
>>>>>>> Stashed changes
