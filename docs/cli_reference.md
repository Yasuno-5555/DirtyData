# DirtyData Forensic CLI Reference

`dirty` is the only official interface connecting humans and the system.
Under the philosophy that "every state must be explainable, or disposable," various commands are provided.

## Basic Commands

### `dirty init`
Initializes the current directory as a DirtyData project.
Generates the `.dirtydata/` directory and creates the default `main` branch.

### `dirty status`
Visually displays the current graph state, node/edge counts, recent patch history, Active Intents, and the system's "Confidence Score."
This is the most critical command for verifying how well the project is in a "correctly explainable state."

### `dirty doctor`
Diagnoses project health, detecting and warning about errors, Confidence Debt, and "Disposable nodes."

### `dirty snapshot <NAME>`
Saves the current state as a named snapshot.

## Patch Operations

In DirtyData, all state changes are performed through patches.

### `dirty patch apply <FILE> [--intent <INTENT_ID>]`
Applies a patch file in JSON format.
Internally compiles `UserAction` into `Operation`, advances the graph Revision, and updates the current branch HEAD.

### `dirty patch list`
Lists the history of patches applied to the current branch.

### `dirty patch replay [--verify]`
Replays all patches recorded in the current history from the beginning and verifies if the final state matches the current graph exactly (is deterministic).

## Timeline and Branching

### `dirty branch [NAME]`
Forks a new branch. If NAME is omitted, it lists current branches.

### `dirty checkout <NAME>`
Switches to the specified branch.
Transitions to another state instantly by swapping the IR pointer.

## Daemon and Monitoring (Observer & Runtime)

### `dirty daemon`
Starts background monitoring of project directory changes and real-time audio playback (cpal).

### `dirty observe`
Recalculates hashes and timestamps of external file systems (WAV files, etc.).

### `dirty repair <NODE_NAME>`
Redefines the current state of an external file as "correct" in response to "unintended hash mismatches" detected by the Observer.

## Intent Management

### `dirty intent add <DESCRIPTION> [--must <...>] [--prefer <...>] [--avoid <...>] [--never <...>]`
Registers a new Intent (intent/constraint) with the system.

### `dirty intent list`
Lists all Intents currently registered in the system.

### `dirty intent attach <INTENT_ID> <PATCH_ID>`
Links an Intent to an existing patch.

## Advanced Operations and Simulation

### `dirty mutate <NODE> [--level <LEVEL>] [--count <N>]`
Evolves (Mutates) the parameters of a specified node.
- **Level**: safe, wild, radioactive (default: wild)
- **Count**: Number of iterations (default: 100)

### `dirty freeze <NODE_NAME> [--length <SEC>]`
Freezes a node's output as a deterministic asset (WAV).

### `dirty null-test [--length <SEC>]`
Performs a mathematical null test to prove engine determinism.

### `dirty install <CRATE_NAME> [--version <VER>]`
Installs an external DSP crate into the ecosystem.

### `dirty preset export/import`
Exports or imports node configurations as presets.

## Forensic and Interactive Patching

These commands enable complex patch construction and analysis directly from the terminal.

### `dirtyrack new <PATCH> [--template <NAME>]`
Creates a new patch file. Templates include `basic` (output only) and `complete` (example routing).

### `dirtyrack inspect <PATCH>`
Displays a forensic summary of the patch: modules (with IDs/aliases), connections, and versioning.

### `dirtyrack snapshot <PATCH> [--id <STABLE_ID>]`
Exposes the internal DSP state, thermal heat, and signal levels of modules. Used for deep forensic analysis.

### `dirtyrack add-module <PATCH> <MODULE_ID> [X] [ROW]`
Adds a module to the patch at specified rack coordinates.

### `dirtyrack connect <PATCH> <FROM_ID> <FROM_PORT> <TO_ID> <TO_PORT>`
Wires two modules together.

### `dirtyrack set-param <PATCH> <ID> <NAME> <VALUE>`
Precisely sets a parameter value.

### `dirtyrack alias <PATCH> <ID> <NAME>`
Assigns a human-readable alias to a module. This name can be used in other commands (like `rm` or `set-param`).

### `dirtyrack rm <PATCH> <ID>`
Removes a module and all its associated cables. Supports both numeric IDs and aliases.

### `dirtyrack shell <PATCH>`
Enters the **Interactive Patching Shell**.
Provides a REPL environment for rapid construction and analysis.

- **Hierarchical Navigation**:
  - `ls`: Lists modules in the current patch level.
  - `cd <ID/ALIAS>`: Enters a `CompositeModule` (subpatch) to edit its internal circuit.
  - `cd ..`: Returns to the parent patch level.
  - `pwd`: Displays the current editing path.
- **Editing Operations**:
  - `add <MODULE_ID>`, `rm <ID/ALIAS>`: Standard module lifecycle commands.
  - `connect <FROM> <PORT> <TO> <PORT>` (or `conn`): Signal routing.
  - `set <ID> <PARAM> <VALUE>`: Precision parameter adjustment.
  - `multiply <COUNT> <MODULE_ID>` (or `mul`): Horizontally clones and deploys multiple modules in one command.
- **Monitoring**:
  - `play`: Real-time audio playback.
  - `render`: Deterministic offline bounce.

---

## GUI Integrated Commands (Summoner HUD)

DirtyRack GUI bridges the gap between intuitive mouse-driven patching and high-efficiency CLI control.

### `The SUMMONER`
Pressing `Space` or `Enter` in the GUI editor opens the **Summoner Bar** at the mouse cursor position.

- **Ad-hoc Deployment**: Type `add dirty_vco` and hit Enter to instantly "summon" the module **exactly where your mouse is pointing**. No browsing required.
- **Hybrid Wiring**: Use the mouse for spatial positioning and the keyboard for rapid-fire wiring (e.g., `conn 1 out 2 in`). This workflow is significantly faster than traditional mouse-only dragging.
- **Swarm Summoning**: `mul 16 dirty_vco` allows you to deploy and align 16 oscillators across the rack in seconds, providing an instant foundation for massive polyphonic patches.

---

## Hierarchical Subpatching

DirtyRack supports encapsulating complex circuits into reusable `.dirtyrack` files, which can be deployed as custom modules.

- **Composite Module**: A node that executes an external patch file recursively within the main engine.
- **IO Bridging**:
  - `subpatch_in / subpatch_out`: Bridges audio and CV signals across the hierarchy boundary.
  - `subpatch_param (Macro Knob)`: Acts as a bridge for parameter inputs from the parent patch. It provides a control voltage (CV) to the internal circuit (e.g., DirtyData MNA solvers), allowing you to consolidate complex internal parameters into a single macro control.

---

## Output and Export

### `dirtyrack render [--output <FILE>] [--length <SEC>] [--sample-rate <HZ>]`
Renders the current graph offline (Deterministic Bounce) and outputs it as a WAV file.

### `dirtyrack verify <AUDIO_WAV> <CERT_JSON>`
Performs **Acoustic Notarization**. Verifies that the given `.wav` file was indeed generated by the patch state described in the `.dirtyrack.cert` file.
