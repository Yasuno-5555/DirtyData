//! Filesystem-based storage for DirtyData.
//!
//! boring は美徳。FS で行く。
//!
//! Layout:
//! .dirtydata/
//! ├── HEAD                 # ref: refs/heads/main
//! ├── refs/
//! │   └── heads/
//! │       └── main         # Latest PatchId for branch 'main'
//! ├── ir/
//! │   └── current.json       # Current Graph snapshot
//! ├── patches/
//! │   ├── {patch_id}.json    # Individual patches
//! │   └── index.json         # Patch DAG metadata
//! ├── intents/
//! │   └── {intent_id}.json   # Intent metadata
//! └── config.json            # Project config
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::ir::Graph;
use crate::patch::Patch;
use crate::types::PatchId;

/// Root storage directory name.
const DIRTYDATA_DIR: &str = ".dirtydata";

/// Errors during storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("project not initialized — run `dirtydata init`")]
    NotInitialized,

    #[error("patch {0} not found")]
    PatchNotFound(PatchId),
}

/// Filesystem-based storage backend.
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    /// Open storage at the given project root.
    pub fn open(project_root: &Path) -> Result<Self, StorageError> {
        let root = project_root.join(DIRTYDATA_DIR);
        if !root.exists() {
            return Err(StorageError::NotInitialized);
        }
        Ok(Self { root })
    }

    /// Initialize a new DirtyData project.
    pub fn init(project_root: &Path) -> Result<Self, StorageError> {
        let root = project_root.join(DIRTYDATA_DIR);
        fs::create_dir_all(root.join("ir"))?;
        fs::create_dir_all(root.join("patches"))?;
        fs::create_dir_all(root.join("intents"))?;
        fs::create_dir_all(root.join("refs").join("heads"))?;

        // Initialize HEAD and main branch
        fs::write(root.join("HEAD"), "ref: refs/heads/main")?;

        // Write default config
        let config = serde_json::json!({
            "version": "0.1.0",
            "hash_algorithm": "blake3",
            "id_scheme": "ulid"
        });
        fs::write(
            root.join("config.json"),
            serde_json::to_string_pretty(&config)?,
        )?;

        // Write empty patch index
        let index = PatchIndex {
            patches: Vec::new(),
        };
        fs::write(
            root.join("patches").join("index.json"),
            serde_json::to_string_pretty(&index)?,
        )?;

        // Write empty graph
        let graph = Graph::new();
        fs::write(
            root.join("ir").join("current.json"),
            serde_json::to_string_pretty(&graph)?,
        )?;

        Ok(Self { root })
    }

    // ── Graph ─────────────────────────────────

    /// Load the current graph.
    pub fn load_graph(&self) -> Result<Graph, StorageError> {
        let path = self.root.join("ir").join("current.json");
        let data = fs::read_to_string(&path)?;
        let mut graph: Graph = serde_json::from_str(&data)?;
        graph.sync();
        Ok(graph)
    }

    /// Save the current graph.
    pub fn save_graph(&self, graph: &Graph) -> Result<(), StorageError> {
        let path = self.root.join("ir").join("current.json");
        let data = serde_json::to_string_pretty(graph)?;
        fs::write(path, data)?;
        Ok(())
    }

    // ── Branches (Timeline) ───────────────────

    /// Read the current branch name from HEAD
    pub fn read_head(&self) -> Result<String, StorageError> {
        let head_path = self.root.join("HEAD");
        if !head_path.exists() {
            return Ok("main".to_string());
        }
        let content = fs::read_to_string(head_path)?;
        let content = content.trim();
        if content.starts_with("ref: refs/heads/") {
            Ok(content.replace("ref: refs/heads/", ""))
        } else {
            Ok(content.to_string()) // detached HEAD support later if needed
        }
    }

    /// Update HEAD to point to a branch
    pub fn write_head(&self, branch: &str) -> Result<(), StorageError> {
        let head_path = self.root.join("HEAD");
        fs::write(head_path, format!("ref: refs/heads/{}", branch))?;
        Ok(())
    }

    /// Get the PatchId a branch points to
    pub fn read_branch(&self, branch: &str) -> Result<Option<PatchId>, StorageError> {
        let path = self.root.join("refs").join("heads").join(branch);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        let content = content.trim();
        if content.is_empty() {
            return Ok(None);
        }
        Ok(content.parse::<PatchId>().ok())
    }

    /// Update a branch to point to a PatchId
    pub fn write_branch(&self, branch: &str, patch_id: PatchId) -> Result<(), StorageError> {
        let path = self.root.join("refs").join("heads").join(branch);
        fs::write(path, patch_id.to_string())?;
        Ok(())
    }

    /// List all local branches
    pub fn list_branches(&self) -> Result<Vec<String>, StorageError> {
        let mut branches = Vec::new();
        let heads_dir = self.root.join("refs").join("heads");
        if heads_dir.exists() {
            for entry in fs::read_dir(heads_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        branches.push(name.to_string());
                    }
                }
            }
        }
        branches.sort();
        Ok(branches)
    }

    // ── Patches ───────────────────────────────

    /// Save a patch.
    pub fn save_patch(&self, patch: &Patch) -> Result<(), StorageError> {
        let filename = format!("{}.json", patch.identity);
        let path = self.root.join("patches").join(&filename);
        let data = serde_json::to_string_pretty(patch)?;
        fs::write(path, data)?;

        // Update index
        let mut index = self.load_patch_index()?;
        let entry = PatchIndexEntry {
            id: patch.identity,
            parents: patch.parents.clone(),
            timestamp: patch.timestamp,
            hash: patch.deterministic_hash,
        };
        // Avoid duplicates
        if !index.patches.iter().any(|e| e.id == patch.identity) {
            index.patches.push(entry);
            self.save_patch_index(&index)?;
        }

        // Auto-update current branch pointer if applying a new patch
        let current_branch = self.read_head()?;
        self.write_branch(&current_branch, patch.identity)?;

        Ok(())
    }

    /// Load a patch by ID.
    pub fn load_patch(&self, id: &PatchId) -> Result<Patch, StorageError> {
        let filename = format!("{}.json", id);
        let path = self.root.join("patches").join(&filename);
        if !path.exists() {
            return Err(StorageError::PatchNotFound(*id));
        }
        let data = fs::read_to_string(path)?;
        let patch = serde_json::from_str(&data)?;
        Ok(patch)
    }

    /// Load all patches in order.
    pub fn load_all_patches(&self) -> Result<Vec<Patch>, StorageError> {
        let index = self.load_patch_index()?;
        let mut patches = Vec::new();
        for entry in &index.patches {
            patches.push(self.load_patch(&entry.id)?);
        }
        Ok(patches)
    }

    /// Load the patch index.
    fn load_patch_index(&self) -> Result<PatchIndex, StorageError> {
        let path = self.root.join("patches").join("index.json");
        let data = fs::read_to_string(path)?;
        let index = serde_json::from_str(&data)?;
        Ok(index)
    }

    /// Load patches starting from a tip, following parents backwards, then return in chronological order.
    pub fn load_patch_ancestry(&self, tip: PatchId) -> Result<Vec<Patch>, StorageError> {
        let mut ancestry = Vec::new();
        let mut current = Some(tip);

        while let Some(id) = current {
            let patch = self.load_patch(&id)?;
            // Simple linear history assumption for now (take first parent)
            current = patch.parents.first().copied();
            ancestry.push(patch);
        }

        // Reverse to get chronological order
        ancestry.reverse();
        Ok(ancestry)
    }

    /// Save the patch index.
    fn save_patch_index(&self, index: &PatchIndex) -> Result<(), StorageError> {
        let path = self.root.join("patches").join("index.json");
        let data = serde_json::to_string_pretty(index)?;
        fs::write(path, data)?;
        Ok(())
    }

    /// Get the storage root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Metadata index for the patch DAG.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PatchIndex {
    pub patches: Vec<PatchIndexEntry>,
}

/// An entry in the patch index.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PatchIndexEntry {
    pub id: PatchId,
    pub parents: Vec<PatchId>,
    pub timestamp: crate::types::Timestamp,
    pub hash: crate::types::Hash,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Node;
    use crate::patch::{Operation, Patch};

    #[test]
    fn test_init_and_load() {
        let tmp = std::env::temp_dir().join(format!("dirtydata_test_{}", ulid::Ulid::new()));
        fs::create_dir_all(&tmp).unwrap();

        let storage = Storage::init(&tmp).unwrap();
        let graph = storage.load_graph().unwrap();
        assert!(graph.nodes.is_empty());
        assert_eq!(graph.revision.0, 0);

        // Cleanup
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_save_and_load_patch() {
        let tmp = std::env::temp_dir().join(format!("dirtydata_test_{}", ulid::Ulid::new()));
        fs::create_dir_all(&tmp).unwrap();

        let storage = Storage::init(&tmp).unwrap();

        let node = Node::new_source("Sine");
        let patch = Patch::from_operations(vec![Operation::AddNode(node)]);
        storage.save_patch(&patch).unwrap();

        let loaded = storage.load_patch(&patch.identity).unwrap();
        assert_eq!(loaded.identity, patch.identity);
        assert_eq!(loaded.operations.len(), 1);

        // Cleanup
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_full_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("dirtydata_test_{}", ulid::Ulid::new()));
        fs::create_dir_all(&tmp).unwrap();

        let storage = Storage::init(&tmp).unwrap();

        // Build a graph
        let mut graph = storage.load_graph().unwrap();
        let node = Node::new_processor("Gain");
        let patch = Patch::from_operations(vec![Operation::AddNode(node.clone())]);
        graph.apply_patch(&patch).unwrap();

        // Save everything
        storage.save_graph(&graph).unwrap();
        storage.save_patch(&patch).unwrap();

        // Reload and verify
        let reloaded = storage.load_graph().unwrap();
        assert_eq!(reloaded.nodes.len(), 1);
        assert!(reloaded.nodes.contains_key(&node.id));

        // Replay from patches
        let patches = storage.load_all_patches().unwrap();
        let replayed = Graph::replay(&patches).unwrap();
        assert_eq!(replayed.nodes.len(), reloaded.nodes.len());

        // Cleanup
        fs::remove_dir_all(&tmp).ok();
    }
}
