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

## Output and Export

### `dirty render [--output <FILE>] [--length <SEC>] [--sample-rate <HZ>]`
Renders the current graph offline (Deterministic Bounce) and outputs it as a WAV file.

### `dirty export <FORMAT>`
Exports the graph in various formats.
- `dsl`: Human-readable Surface DSL format.
- `json`: JSON format.
- `vst3`: VST3 plugin format.
- `clap`: CLAP plugin format.
