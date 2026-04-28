# DirtyData: Research-Grade Forensic Audio & Circuit Simulation Framework

DirtyData is a high-fidelity, headless audio engine built for **deterministic sound reconstruction**, **cryptographic causality**, and **ML-driven exploration**. 

Unlike traditional DAWs or DSP environments that treat sound as a transient stream, DirtyData treats every sample as a **verifiable lineage of intent** tracked via a Merkle DAG. It is the "Git of Sound," designed for researchers who demand absolute reproducibility and forensic integrity.

[日本語版はこちら (Japanese version)](README.ja.md)

---

## 🔬 Why DirtyData?

- **Deterministic Reality**: Same Patch + Same Seed = Identical Bit-stream. Always.
- **Forensic Lineage**: Every change is a node in a Merkle DAG. Audit your sound's history with `dirty doctor`.
- **The Reality/Observation Boundary**: Core DSP is locked in a high-performance Rust "Reality" layer, while research and ML happen in a flexible Python "Observation" layer.
- **Circuit Evolution**: Don't just tweak parameters; mutate the topology using evolutionary algorithms.

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
- **API Usage**: [Usage Guide](docs/API%20Usage.md)
