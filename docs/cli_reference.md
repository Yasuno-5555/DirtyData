# DirtyRack Forensic CLI Reference (Draft)

`dirtydata-cli` is the only official interface connecting humans and the system.
Under the philosophy that "every state must be explainable, or disposable," various commands are provided.

## Basic Commands

### `dirtydata init`
Initializes the current directory as a DirtyData project.
Generates the `.dirtydata/` directory and creates the default `main` branch.

### `dirtydata status`
Visually displays the current graph state, node/edge counts, recent patch history, Active Intents, and the system's "Confidence Score."
This is the most critical command for verifying how well the project is in a "correctly explainable state."

### `dirtydata doctor`
Diagnoses project health, detecting and warning about errors, Confidence Debt, and "Disposable nodes."

### `dirtydata snapshot <NAME>`
Saves the current state as a named snapshot.

## Patch Operations

In DirtyData, all state changes are performed through patches.

### `dirtydata patch apply <FILE> [--intent <INTENT_ID>]`
Applies a patch file in JSON format.
Internally compiles `UserAction` into `Operation`, advances the graph Revision, and updates the current branch HEAD.

### `dirtydata patch list`
Lists the history of patches applied to the current branch.

### `dirtydata patch replay [--verify]`
Replays all patches recorded in the current history from the beginning and verifies if the final state matches the current graph exactly (is deterministic).

## Timeline and Branching

### `dirtydata branch [NAME]`
Forks a new branch. If NAME is omitted, it lists current branches.

### `dirtydata checkout <NAME>`
Switches to the specified branch.
Transitions to another state instantly by swapping the IR pointer.

## Daemon and Monitoring (Observer & Runtime)

### `dirtydata daemon`
Starts background monitoring of project directory changes and real-time audio playback (cpal).

### `dirtydata observe`
Recalculates hashes and timestamps of external file systems (WAV files, etc.).

### `dirtydata repair <NODE_NAME>`
Redefines the current state of an external file as "correct" in response to "unintended hash mismatches" detected by the Observer.

### `dirtydata gui`
Launches the Graphical Projector.

## Intent Management

### `dirtydata intent add <DESCRIPTION> [--must <...>] [--prefer <...>] [--avoid <...>] [--never <...>]`
Registers a new Intent (intent/constraint) with the system.

### `dirtydata intent list`
Lists all Intents currently registered in the system.

### `dirtydata intent attach <INTENT_ID> <PATCH_ID>`
Links an Intent to an existing patch.

## Advanced Operations and Simulation

### `dirtydata mutate <NODE> [--tier <TIER>] [--count <N>]`
Evolves (Mutates) the parameters of a specified node.
- **Tier**: safe, wild, radioactive, forbidden (default: safe)
- **Count**: Number of iterations (default: 10)

### `dirtydata freeze <NODE_NAME> [--length <SEC>]`
Freezes a node's output as a deterministic asset (WAV).

### `dirtydata null-test [--length <SEC>]`
Performs a mathematical null test to prove engine determinism.

### `dirtydata install <CRATE_NAME> [--version <VER>]`
Installs an external DSP crate into the ecosystem.

### `dirtydata preset export/import`
Exports or imports node configurations as presets.

## Output and Export

### `dirtydata render [--output <FILE>] [--length <SEC>] [--sample-rate <HZ>]`
Renders the current graph offline (Deterministic Bounce) and outputs it as a WAV file.

### `dirtydata export <FORMAT>`
Exports the graph in various formats.
- `dsl`: Human-readable Surface DSL format.
- `json`: JSON format.
- `clap`: CLAP plugin format.
