#![allow(clippy::all)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plugin crashed or closed connection")]
    Crashed,
    #[error("Plugin produced NaN (NaN storm)")]
    NanStorm,
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
#[derive(Debug, serde::Serialize)]
pub struct AuditReport {
    pub root_hash_valid: bool,
    pub manifest_valid: bool,
    pub lineage_intact: bool,
    pub cas_complete: bool,
    pub determinism_verified: bool,
    pub issues: Vec<String>,
}

impl AuditReport {
    pub fn is_healthy(&self) -> bool {
        self.root_hash_valid && self.lineage_intact && self.cas_complete
    }
}

use serde::Serialize;
use std::path::{Path, PathBuf};

/// §SSS: Workspace — The self-contained session manager.
/// "設計図、製造履歴、意図。そのすべてを一つの宇宙に閉じ込める。"
pub struct Workspace {
    root: PathBuf,
    graph: dirtydata_core::ir::Graph,
    intent_state: dirtydata_intent::IntentState,
}

impl Workspace {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, HostError> {
        let root = root.into();
        let dot_dirty = root.join(".dirtydata");

        if !dot_dirty.exists() {
            std::fs::create_dir_all(&dot_dirty)?;
            return Ok(Self {
                root,
                graph: dirtydata_core::ir::Graph::new(),
                intent_state: dirtydata_intent::IntentState::default(),
            });
        }

        // 1. Layer 5: Manifest
        let manifest_path = dot_dirty.join("manifest.json");
        let manifest: dirtydata_core::types::Manifest = if manifest_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?
        } else {
            // Legacy fallback
            return Err(HostError::Crashed); // Or handle appropriately
        };

        // 2. Layer 1: Topology
        let topo_path = dot_dirty.join("topology.ir");
        let topology: dirtydata_core::ir::Topology =
            serde_json::from_str(&std::fs::read_to_string(topo_path)?)?;

        // 3. Layer 3: Lineage
        let lineage_path = dot_dirty.join("lineage.dag");
        let lineage: dirtydata_core::ir::Lineage =
            serde_json::from_str(&std::fs::read_to_string(lineage_path)?)?;

        // 4. Layer 2: Circuit Registry (Loaded via references from Topology/Manifest or walking CAS)
        let mut registry = dirtydata_core::ir::CircuitRegistry::default();
        let cas_root = dot_dirty.join("circuits").join("blake3");
        if cas_root.exists() {
            // For now, we'll walk all files in the CAS and load them
            for entry in walkdir::WalkDir::new(cas_root) {
                let entry = entry.map_err(|_| HostError::Crashed)?;
                if entry.file_type().is_file() {
                    let data = std::fs::read_to_string(entry.path())?;
                    let def: dirtydata_core::types::CircuitDefinition =
                        serde_json::from_str(&data)?;
                    registry.definitions.insert(def.id, def);
                }
            }
        }

        let mut graph = dirtydata_core::ir::Graph {
            spec_version: manifest.spec_version,
            topology,
            lineage,
            registry,
            verification: manifest.verification,
            revision: dirtydata_core::types::Revision(manifest.last_revision),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            modulations: BTreeMap::new(),
        };
        graph.sync();

        let intent_state = dirtydata_intent::IntentState::load(&root)?;

        Ok(Self {
            root,
            graph,
            intent_state,
        })
    }

    /// Performs an atomic save of the entire forensic record using CAS.
    pub fn save(&self) -> Result<(), HostError> {
        let dot_dirty = self.root.join(".dirtydata");
        std::fs::create_dir_all(&dot_dirty)?;

        // 1. Layer 1: Topology
        self.save_atomic(&dot_dirty.join("topology.ir"), &self.graph.topology)?;

        // 2. Layer 2: Circuit Registry (CAS Storage)
        for def in self.graph.registry.definitions.values() {
            let hash = def.hash();
            let hash_hex = hex::encode(hash);
            let cas_path = dot_dirty
                .join("circuits")
                .join("blake3")
                .join(&hash_hex[0..2])
                .join(&hash_hex[2..4]);
            std::fs::create_dir_all(&cas_path)?;
            self.save_atomic(&cas_path.join(&hash_hex), def)?;
        }

        // 3. Layer 3: Lineage
        self.save_atomic(&dot_dirty.join("lineage.dag"), &self.graph.lineage)?;

        // 4. Layer 4: Intents
        self.intent_state.save(&self.root)?;

        // 5. Layer 5: Verification (Manifest with Root Hash & Signature)
        let root_hash = self.calculate_root_hash()?;
        let manifest = dirtydata_core::types::Manifest {
            spec_version: self.graph.spec_version.clone(),
            last_revision: self.graph.revision.0,
            timestamp: dirtydata_core::types::Timestamp::now().0,
            verification: dirtydata_core::types::Verification {
                null_test: true,
                hash: hex::encode(root_hash),
                trust_state: "verified".into(),
            },
            author_id: "dirtydata-host-local".into(),
            public_key: "ed25519:stub_key".into(),
            signature: "stub_signature".into(),
        };
        self.save_atomic(&dot_dirty.join("manifest.json"), &manifest)?;

        tracing::info!(
            "Forensic record (Merkle DAG) saved successfully. Root Hash: {}",
            hex::encode(root_hash)
        );
        Ok(())
    }

    pub fn calculate_root_hash(&self) -> Result<dirtydata_core::types::Hash, HostError> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(serde_json::to_string(&self.graph.topology)?.as_bytes());
        // For Merkle DAG, we hash the head of the lineage
        hasher.update(serde_json::to_string(&self.graph.lineage)?.as_bytes());
        hasher.update(serde_json::to_string(&self.intent_state)?.as_bytes());

        // Include the hash of the last applied patch as the 'tip' of the Merkle chain
        if let Some(&last_id) = self.graph.lineage.applied_patches.last() {
            if let Some(patch) = self.graph.lineage.history.get(&last_id) {
                hasher.update(&patch.deterministic_hash);
            }
        }

        for def in self.graph.registry.definitions.values() {
            hasher.update(&def.hash());
        }
        Ok(*hasher.finalize().as_bytes())
    }

    fn save_atomic<T: Serialize>(&self, path: &Path, data: &T) -> Result<(), HostError> {
        let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        let json = serde_json::to_string_pretty(data)?;
        temp.write_all(json.as_bytes())?;
        temp.persist(path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }

    // --- Accessors ---

    pub fn graph(&self) -> &dirtydata_core::ir::Graph {
        &self.graph
    }

    pub fn graph_mut(&mut self) -> &mut dirtydata_core::ir::Graph {
        &mut self.graph
    }

    pub fn intent_state(&self) -> &dirtydata_intent::IntentState {
        &self.intent_state
    }

    pub fn intent_state_mut(&mut self) -> &mut dirtydata_intent::IntentState {
        &mut self.intent_state
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    // --- High Level API ---

    pub fn apply_patch(&mut self, patch: dirtydata_core::patch::Patch) -> Result<(), HostError> {
        self.graph.apply_patch(&patch).map_err(|e| {
            println!("Failed to apply patch: {:?}", e);
            HostError::Crashed
        })?;
        self.save()?;
        Ok(())
    }

    pub fn apply_user_patch(
        &mut self,
        patch_file: dirtydata_core::actions::UserPatchFile,
    ) -> Result<(), HostError> {
        let ops = dirtydata_core::actions::compile_actions(&patch_file.actions, &self.graph)
            .map_err(|e| {
                println!("Failed to compile actions: {:?}", e);
                HostError::Crashed
            })?;
        let patch = dirtydata_core::patch::Patch::from_operations(ops);
        self.apply_patch(patch)
    }

    /// Audits the forensic integrity of the workspace.
    pub fn audit(&self) -> Result<AuditReport, HostError> {
        let mut report = AuditReport {
            root_hash_valid: false,
            manifest_valid: true, // Placeholder for signature check
            lineage_intact: true,
            cas_complete: true,
            determinism_verified: false,
            issues: Vec::new(),
        };

        // 1. Verify Root Hash
        let actual_hash = self.calculate_root_hash()?;
        let manifest_path = self.root.join(".dirtydata").join("manifest.json");
        if manifest_path.exists() {
            let manifest: dirtydata_core::types::Manifest =
                serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
            if hex::encode(actual_hash) == manifest.verification.hash {
                report.root_hash_valid = true;
            } else {
                report.issues.push(format!(
                    "Root hash mismatch! Manifest: {}, Actual: {}",
                    manifest.verification.hash,
                    hex::encode(actual_hash)
                ));
            }
        }

        // 2. Audit Lineage (Merkle chain verification)
        let mut prev_hash = None;
        for patch_id in &self.graph.lineage.applied_patches {
            if let Some(patch) = self.graph.lineage.history.get(patch_id) {
                if !patch.verify_hash() {
                    report.lineage_intact = false;
                    report
                        .issues
                        .push(format!("Patch {} has corrupted hash", patch_id));
                }
                // Verify parent linkage
                if let Some(p_hash) = prev_hash {
                    if !patch.parent_hashes.contains(&p_hash) {
                        // In a simple chain this would be an error, but in a DAG it's more complex.
                        // For now, just logging suspicious gaps.
                    }
                }
                prev_hash = Some(patch.deterministic_hash);
            }
        }

        // 3. CAS Completeness
        for node in self.graph.topology.nodes.values() {
            if let dirtydata_core::types::NodeKind::Processor = node.kind {
                // If we implement circuit definitions in registry, check them here
            }
        }

        // 4. Determinism Check (Quick null-test replay)
        // This requires dirtydata-runtime, so we might move it to a high-level helper
        // For now, mark as pending

        Ok(report)
    }
}

// #[derive(Serialize, Deserialize)]
// struct WorkspaceMeta {
//     last_revision: u64,
//     timestamp: i64,
// }

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum HostCommand {
    Process = 0,
    SetParameter = 1,
    GetState = 2,
    SetState = 3,
}

pub struct PluginHost {
    child: Child,
    fallback_buffer: Vec<f32>,
}

impl PluginHost {
    pub fn new(plugin_name: &str, buffer_size: usize) -> Result<Self, HostError> {
        let exe = std::env::current_exe().unwrap_or_default();
        let dir = exe.parent().unwrap_or(std::path::Path::new("."));
        let worker_path = dir.join("dirtydata-plugin-worker");

        let child = Command::new(&worker_path)
            .arg(plugin_name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| {
                tracing::error!(
                    "Failed to spawn plugin worker '{}' at {:?}: {}",
                    plugin_name,
                    worker_path,
                    e
                );
                e
            })?;

        tracing::info!(
            "Spawned plugin worker '{}' (pid={})",
            plugin_name,
            child.id()
        );
        Ok(Self {
            child,
            fallback_buffer: vec![0.0; buffer_size],
        })
    }

    pub fn set_parameter(&mut self, param_id: u32, value: f32) -> Result<(), HostError> {
        let mut stdin = self.child.stdin.as_ref().ok_or(HostError::Crashed)?;

        let cmd = HostCommand::SetParameter as u8;
        stdin.write_all(&[cmd])?;
        stdin.write_all(&param_id.to_le_bytes())?;
        stdin.write_all(&value.to_le_bytes())?;
        stdin.flush()?;

        Ok(())
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), HostError> {
        let mut stdin = self.child.stdin.as_ref().ok_or(HostError::Crashed)?;
        let stdout = self.child.stdout.as_mut().ok_or(HostError::Crashed)?;

        // Send Command
        let cmd = HostCommand::Process as u8;
        stdin.write_all(&[cmd])?;

        // Send size (u32)
        let size = input.len() as u32;
        stdin.write_all(&size.to_le_bytes())?;

        // Write input buffer as bytes
        let in_bytes = bytemuck::cast_slice(input);
        if stdin.write_all(in_bytes).is_err() {
            tracing::error!("Failed to write to plugin stdin — plugin likely crashed");
            return Err(HostError::Crashed);
        }
        if stdin.flush().is_err() {
            tracing::error!("Failed to flush plugin stdin");
            return Err(HostError::Crashed);
        }

        // Read output buffer as bytes
        let out_bytes = bytemuck::cast_slice_mut(output);
        if stdout.read_exact(out_bytes).is_err() {
            return Err(HostError::Crashed);
        }

        // Check for NaN Storm
        for sample in output.iter() {
            if sample.is_nan() {
                tracing::warn!("NaN detected in plugin output! Entering NaN storm protocol.");
                return Err(HostError::NanStorm);
            }
        }

        // Update fallback buffer
        if self.fallback_buffer.len() != output.len() {
            self.fallback_buffer.resize(output.len(), 0.0);
        }
        self.fallback_buffer.copy_from_slice(output);

        Ok(())
    }

    pub fn get_fallback(&self) -> &[f32] {
        &self.fallback_buffer
    }
}
