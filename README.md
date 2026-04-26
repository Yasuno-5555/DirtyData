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
