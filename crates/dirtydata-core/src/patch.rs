//! Patch Engine — the heart of DirtyData.
//!
//! MVP Requirements:
//! - patch apply
//! - patch diff
//! - patch merge
//! - patch replay
//! - deterministic hash
//! - undo as branch

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::hash;
use crate::ir::{Edge, Graph, Node};
use crate::types::*;

// ──────────────────────────────────────────────
// §5.3 — Patch
// ──────────────────────────────────────────────

/// A single atomic patch — the unit of change.
///
/// `parents` is `Vec<PatchId>`, not `Option<PatchId>`.
/// Because merge exists. A DAG with single parent is a tree.
///
/// §SSS+: Merkle DAG
/// Every patch now carries `parent_hashes` to ensure a cryptographically 
/// verifiable chain of causality.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub identity: PatchId,
    pub operations: Vec<Operation>,
    pub intent_ref: Option<IntentId>,
    pub deterministic_hash: Hash,
    /// DAG parents — identifiers.
    pub parents: Vec<PatchId>,
    /// DAG parents — content hashes for Merkle verification.
    pub parent_hashes: Vec<Hash>,
    pub timestamp: Timestamp,
    pub source: PatchSource,
    pub trust: TrustLevel,
}

/// Atomic operations on the Canonical IR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Operation {
    AddNode(Node),
    RemoveNode(StableId),
    ReplaceNode(Node),
    ModifyConfig {
        node_id: StableId,
        delta: ConfigDelta,
    },
    AddEdge(Edge),
    RemoveEdge(StableId),
    ModifyEdge {
        edge_id: StableId,
        delta: EdgeDelta,
    },
    AddModulation(crate::ir::Modulation),
    RemoveModulation(StableId),
}

/// An ordered collection of patches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchSet {
    pub patches: Vec<Patch>,
}

// ──────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────

#[derive(Debug, Clone, thiserror::Error)]
pub enum PatchError {
    #[error("node {0} not found")]
    NodeNotFound(StableId),

    #[error("edge {0} not found")]
    EdgeNotFound(StableId),

    #[error("node {0} already exists")]
    NodeAlreadyExists(StableId),

    #[error("edge {0} already exists")]
    EdgeAlreadyExists(StableId),

    #[error("port '{port}' not found on node {node}")]
    PortNotFound { node: StableId, port: String },

    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("merge conflict: {0}")]
    MergeConflict(String),
}

// ──────────────────────────────────────────────
// Patch construction
// ──────────────────────────────────────────────

impl Patch {
    /// Create a patch from operations. Hash is computed automatically.
    pub fn from_operations(operations: Vec<Operation>) -> Self {
        let mut patch = Self {
            identity: PatchId::new(),
            operations,
            intent_ref: None,
            deterministic_hash: [0u8; 32],
            parents: Vec::new(),
            parent_hashes: Vec::new(),
            timestamp: Timestamp::now(),
            source: PatchSource::System,
            trust: TrustLevel::Trusted,
        };
        patch.deterministic_hash = hash::hash_patch(&patch);
        patch
    }

    /// Create a patch with explicit source and trust level.
    pub fn from_operations_with_provenance(
        operations: Vec<Operation>,
        source: PatchSource,
        trust: TrustLevel,
    ) -> Self {
        let mut patch = Self {
            identity: PatchId::new(),
            operations,
            intent_ref: None,
            deterministic_hash: [0u8; 32],
            parents: Vec::new(),
            parent_hashes: Vec::new(),
            timestamp: Timestamp::now(),
            source,
            trust,
        };
        patch.deterministic_hash = hash::hash_patch(&patch);
        patch
    }

    /// Attach an intent to this patch.
    pub fn with_intent(mut self, intent_id: IntentId) -> Self {
        self.intent_ref = Some(intent_id);
        // Rehash — intent changes content identity
        self.deterministic_hash = hash::hash_patch(&self);
        self
    }

    /// Set parent patches (for Merkle DAG lineage).
    pub fn with_parents(mut self, parents: Vec<(PatchId, Hash)>) -> Self {
        self.parents = parents.iter().map(|(id, _)| *id).collect();
        self.parent_hashes = parents.iter().map(|(_, h)| *h).collect();
        // Rehash — parents change content identity
        self.deterministic_hash = hash::hash_patch(&self);
        self
    }

    /// Verify the patch's hash integrity.
    pub fn verify_hash(&self) -> bool {
        let computed = hash::hash_patch(self);
        computed == self.deterministic_hash
    }
}

// ──────────────────────────────────────────────
// Graph::apply — patch application
// ──────────────────────────────────────────────

impl Graph {
    /// Apply a single patch to this graph.
    #[tracing::instrument(skip(self, patch))]
    pub fn apply(&mut self, patch: &Patch) -> Result<(), PatchError> {
        tracing::debug!("Applying patch {} with {} operations", patch.identity, patch.operations.len());
        for op in &patch.operations {
            self.apply_operation(op)?;
        }
        self.revision = self.revision.next();
        self.lineage.applied_patches.push(patch.identity);
        self.lineage.history.insert(patch.identity, patch.clone());
        Ok(())
    }

    #[tracing::instrument(skip(self, op))]
    fn apply_operation(&mut self, op: &Operation) -> Result<(), PatchError> {
        tracing::trace!("Applying operation: {:?}", op);
        match op {
            Operation::AddNode(node) => {
                if self.nodes.contains_key(&node.id) {
                    return Err(PatchError::NodeAlreadyExists(node.id));
                }
                self.nodes.insert(node.id, node.clone());
            }

            Operation::RemoveNode(id) => {
                if self.nodes.remove(id).is_none() {
                    return Err(PatchError::NodeNotFound(*id));
                }
                // Remove all edges connected to this node.
                self.edges
                    .retain(|_, e| e.source.node_id != *id && e.target.node_id != *id);
            }

            Operation::ReplaceNode(node) => {
                if !self.nodes.contains_key(&node.id) {
                    return Err(PatchError::NodeNotFound(node.id));
                }
                self.nodes.insert(node.id, node.clone());
                // Note: We do NOT remove edges. It's up to the user to ensure 
                // ports are still compatible, otherwise validation will catch it.
            }

            Operation::ModifyConfig { node_id, delta } => {
                let node = self
                    .nodes
                    .get_mut(node_id)
                    .ok_or(PatchError::NodeNotFound(*node_id))?;
                for (key, change) in delta {
                    match &change.new {
                        Some(val) => {
                            node.config.insert(key.clone(), val.clone());
                        }
                        None => {
                            node.config.remove(key);
                        }
                    }
                }
            }

            Operation::AddEdge(edge) => {
                if self.edges.contains_key(&edge.id) {
                    return Err(PatchError::EdgeAlreadyExists(edge.id));
                }
                self.require_port(&edge.source)?;
                self.require_port(&edge.target)?;
                self.edges.insert(edge.id, edge.clone());
            }

            Operation::RemoveEdge(id) => {
                if self.edges.remove(id).is_none() {
                    return Err(PatchError::EdgeNotFound(*id));
                }
            }

            Operation::ModifyEdge { edge_id, delta } => {
                // Validate ports before borrowing edge mutably
                if let Some(ref src) = delta.source {
                    self.require_port(src)?;
                }
                if let Some(ref tgt) = delta.target {
                    self.require_port(tgt)?;
                }
                let edge = self
                    .edges
                    .get_mut(edge_id)
                    .ok_or(PatchError::EdgeNotFound(*edge_id))?;
                if let Some(ref src) = delta.source {
                    edge.source = src.clone();
                }
                if let Some(ref tgt) = delta.target {
                    edge.target = tgt.clone();
                }
                if let Some(k) = delta.kind {
                    edge.kind = k;
                }
            }

            Operation::AddModulation(m) => {
                if self.modulations.contains_key(&m.id) {
                    return Err(PatchError::EdgeAlreadyExists(m.id)); // Use EdgeAlreadyExists as proxy
                }
                self.require_port(&m.source)?;
                if !self.nodes.contains_key(&m.target_node) {
                    return Err(PatchError::NodeNotFound(m.target_node));
                }
                self.modulations.insert(m.id, m.clone());
            }

            Operation::RemoveModulation(id) => {
                if self.modulations.remove(id).is_none() {
                    return Err(PatchError::EdgeNotFound(*id));
                }
            }
        }
        Ok(())
    }

    /// Validate a port reference exists in the current graph.
    fn require_port(&self, port_ref: &PortRef) -> Result<(), PatchError> {
        let node = self
            .nodes
            .get(&port_ref.node_id)
            .ok_or(PatchError::NodeNotFound(port_ref.node_id))?;
        if !node.ports.iter().any(|p| p.name == port_ref.port_name) {
            return Err(PatchError::PortNotFound {
                node: port_ref.node_id,
                port: port_ref.port_name.clone(),
            });
        }
        Ok(())
    }
}

// ──────────────────────────────────────────────
// Graph::diff — compute difference between graphs
// ──────────────────────────────────────────────

impl Graph {
    /// Compute the diff from `self` to `other` as a PatchSet.
    /// Applying the result to `self` should produce `other`.
    pub fn diff(&self, other: &Graph) -> PatchSet {
        let mut operations = Vec::new();

        // Nodes removed in `other`
        for id in self.topology.nodes.keys() {
            if !other.topology.nodes.contains_key(id) {
                operations.push(Operation::RemoveNode(*id));
            }
        }

        // Nodes added or modified in `other`
        for (id, node) in &other.topology.nodes {
            match self.topology.nodes.get(id) {
                None => operations.push(Operation::AddNode(node.clone())),
                Some(old) => {
                    if old.config != node.config {
                        let delta = config_diff(&old.config, &node.config);
                        if !delta.is_empty() {
                            operations.push(Operation::ModifyConfig {
                                node_id: *id,
                                delta,
                            });
                        }
                    }
                }
            }
        }

        // Edges removed in `other`
        for id in self.topology.edges.keys() {
            if !other.topology.edges.contains_key(id) {
                operations.push(Operation::RemoveEdge(*id));
            }
        }

        // Edges added or modified in `other`
        for (id, edge) in &other.topology.edges {
            match self.topology.edges.get(id) {
                None => operations.push(Operation::AddEdge(edge.clone())),
                Some(old) => {
                    if old != edge {
                        let delta = EdgeDelta {
                            source: if old.source != edge.source {
                                Some(edge.source.clone())
                            } else {
                                None
                            },
                            target: if old.target != edge.target {
                                Some(edge.target.clone())
                            } else {
                                None
                            },
                            kind: if old.kind != edge.kind {
                                Some(edge.kind)
                            } else {
                                None
                            },
                        };
                        operations.push(Operation::ModifyEdge {
                            edge_id: *id,
                            delta,
                        });
                    }
                }
            }
        }

        let patch = Patch::from_operations(operations);
        PatchSet {
            patches: vec![patch],
        }
    }
}

// ──────────────────────────────────────────────
// Graph::replay — deterministic reconstruction
// ──────────────────────────────────────────────

impl Graph {
    /// Replay a sequence of patches from an empty graph.
    /// This is the core of deterministic replayability — §6.
    pub fn replay(patches: &[Patch]) -> Result<Self, PatchError> {
        let mut graph = Graph::new();
        for patch in patches {
            graph.apply(patch)?;
        }
        Ok(graph)
    }

    /// Replay and verify: replay patches, then check that the
    /// resulting graph hash matches the expected hash.
    pub fn replay_and_verify(patches: &[Patch], expected_hash: &Hash) -> Result<Self, PatchError> {
        let graph = Self::replay(patches)?;
        let actual_hash = hash::hash_graph(&graph);
        if &actual_hash != expected_hash {
            return Err(PatchError::HashMismatch {
                expected: hex::encode(expected_hash),
                actual: hex::encode(&actual_hash),
            });
        }
        Ok(graph)
    }
}

// ──────────────────────────────────────────────
// PatchSet operations
// ──────────────────────────────────────────────

impl PatchSet {
    pub fn new() -> Self {
        Self {
            patches: Vec::new(),
        }
    }

    pub fn single(patch: Patch) -> Self {
        Self {
            patches: vec![patch],
        }
    }

    /// MVP merge: sequential composition.
    /// True three-way merge is Phase 2.
    pub fn merge(&self, other: &PatchSet) -> Result<PatchSet, PatchError> {
        let mut merged = self.patches.clone();
        merged.extend(other.patches.iter().cloned());
        Ok(PatchSet { patches: merged })
    }

    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }

    pub fn len(&self) -> usize {
        self.patches.len()
    }
}

impl Default for PatchSet {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Config diff utility
// ──────────────────────────────────────────────

/// Compute the delta between two configurations.
pub fn config_diff(old: &ConfigSnapshot, new: &ConfigSnapshot) -> ConfigDelta {
    let mut delta = BTreeMap::new();

    // Changed or removed keys
    for (key, old_val) in old {
        match new.get(key) {
            Some(new_val) if old_val != new_val => {
                delta.insert(
                    key.clone(),
                    ConfigChange {
                        old: Some(old_val.clone()),
                        new: Some(new_val.clone()),
                    },
                );
            }
            None => {
                delta.insert(
                    key.clone(),
                    ConfigChange {
                        old: Some(old_val.clone()),
                        new: None,
                    },
                );
            }
            _ => {}
        }
    }

    // Added keys
    for (key, new_val) in new {
        if !old.contains_key(key) {
            delta.insert(
                key.clone(),
                ConfigChange {
                    old: None,
                    new: Some(new_val.clone()),
                },
            );
        }
    }

    delta
}

// ──────────────────────────────────────────────
// Hex encoding utility (no external dep for MVP)
// ──────────────────────────────────────────────

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Node;

    fn make_source() -> Node {
        Node::new_source("Sine")
    }

    fn make_gain() -> Node {
        Node::new_processor("Gain")
    }

    fn make_sink() -> Node {
        Node::new_sink("Output")
    }

    #[test]
    fn test_apply_add_node() {
        let mut graph = Graph::new();
        let node = make_source();
        let patch = Patch::from_operations(vec![Operation::AddNode(node.clone())]);

        graph.apply(&patch).unwrap();

        assert_eq!(graph.topology.nodes.len(), 1);
        assert_eq!(graph.topology.nodes.get(&node.id).unwrap().kind, NodeKind::Source);
        assert_eq!(graph.revision, Revision(1));
        assert_eq!(graph.lineage.applied_patches.len(), 1);
    }

    #[test]
    fn test_apply_remove_node() {
        let mut graph = Graph::new();
        let node = make_source();

        // Add then remove
        let add = Patch::from_operations(vec![Operation::AddNode(node.clone())]);
        let remove = Patch::from_operations(vec![Operation::RemoveNode(node.id)]);

        graph.apply(&add).unwrap();
        graph.apply(&remove).unwrap();

        assert!(graph.topology.nodes.is_empty());
        assert_eq!(graph.revision, Revision(2));
    }

    #[test]
    fn test_apply_modify_config() {
        let mut graph = Graph::new();
        let node = make_gain();

        let add = Patch::from_operations(vec![Operation::AddNode(node.clone())]);
        graph.apply(&add).unwrap();

        let delta = {
            let mut d = BTreeMap::new();
            d.insert(
                "gain_db".into(),
                ConfigChange {
                    old: None,
                    new: Some(ConfigValue::Float(2.0)),
                },
            );
            d
        };
        let modify = Patch::from_operations(vec![Operation::ModifyConfig {
            node_id: node.id,
            delta,
        }]);
        graph.apply(&modify).unwrap();

        let n = graph.node(&node.id).unwrap();
        assert_eq!(n.config.get("gain_db"), Some(&ConfigValue::Float(2.0)));
    }

    #[test]
    fn test_apply_add_edge() {
        let mut graph = Graph::new();
        let src = make_source();
        let sink = make_sink();

        let edge = Edge::new(
            PortRef {
                node_id: src.id,
                port_name: "out".into(),
            },
            PortRef {
                node_id: sink.id,
                port_name: "in".into(),
            },
        );

        let patch = Patch::from_operations(vec![
            Operation::AddNode(src.clone()),
            Operation::AddNode(sink.clone()),
            Operation::AddEdge(edge.clone()),
        ]);

        graph.apply(&patch).unwrap();

        assert_eq!(graph.topology.nodes.len(), 2);
        assert_eq!(graph.topology.edges.len(), 1);
    }

    #[test]
    fn test_remove_node_cascades_edges() {
        let mut graph = Graph::new();
        let src = make_source();
        let sink = make_sink();
        let edge = Edge::new(
            PortRef {
                node_id: src.id,
                port_name: "out".into(),
            },
            PortRef {
                node_id: sink.id,
                port_name: "in".into(),
            },
        );

        graph
            .apply(&Patch::from_operations(vec![
                Operation::AddNode(src.clone()),
                Operation::AddNode(sink.clone()),
                Operation::AddEdge(edge),
            ]))
            .unwrap();

        // Remove source — edge should cascade
        graph
            .apply(&Patch::from_operations(vec![Operation::RemoveNode(src.id)]))
            .unwrap();

        assert_eq!(graph.topology.nodes.len(), 1);
        assert!(graph.topology.edges.is_empty());
    }

    #[test]
    fn test_duplicate_node_error() {
        let mut graph = Graph::new();
        let node = make_source();

        graph
            .apply(&Patch::from_operations(vec![Operation::AddNode(
                node.clone(),
            )]))
            .unwrap();

        let result = graph.apply(&Patch::from_operations(vec![Operation::AddNode(node)]));
        assert!(result.is_err());
    }

    #[test]
    fn test_diff_and_apply() {
        let mut g1 = Graph::new();
        let src = make_source();
        g1.apply(&Patch::from_operations(vec![Operation::AddNode(
            src.clone(),
        )]))
        .unwrap();

        let mut g2 = g1.clone();
        let gain = make_gain();
        g2.apply(&Patch::from_operations(vec![Operation::AddNode(
            gain.clone(),
        )]))
        .unwrap();

        // Diff should capture the added gain node
        let diff = g1.diff(&g2);
        assert_eq!(diff.patches.len(), 1);

        // Applying diff to g1 should yield equivalent nodes
        let mut g1_patched = g1.clone();
        for p in &diff.patches {
            g1_patched.apply(p).unwrap();
        }
        assert!(g1_patched.topology.nodes.contains_key(&gain.id));
    }

    #[test]
    fn test_deterministic_replay() {
        let src = make_source();
        let gain = make_gain();
        let sink = make_sink();

        let p1 = Patch::from_operations(vec![
            Operation::AddNode(src.clone()),
            Operation::AddNode(gain.clone()),
            Operation::AddNode(sink.clone()),
        ]);
        let edge = Edge::new(
            PortRef {
                node_id: src.id,
                port_name: "out".into(),
            },
            PortRef {
                node_id: gain.id,
                port_name: "in".into(),
            },
        );
        let p2 = Patch::from_operations(vec![Operation::AddEdge(edge)]);

        // Replay twice — must produce identical graphs
        let g1 = Graph::replay(&[p1.clone(), p2.clone()]).unwrap();
        let g2 = Graph::replay(&[p1, p2]).unwrap();

        let h1 = hash::hash_graph(&g1);
        let h2 = hash::hash_graph(&g2);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_patch_hash_integrity() {
        let node = make_source();
        let patch = Patch::from_operations(vec![Operation::AddNode(node)]);
        assert!(patch.verify_hash());
    }

    #[test]
    fn test_patch_merge() {
        let p1 = PatchSet::single(Patch::from_operations(vec![Operation::AddNode(
            make_source(),
        )]));
        let p2 = PatchSet::single(Patch::from_operations(vec![
            Operation::AddNode(make_gain()),
        ]));

        let merged = p1.merge(&p2).unwrap();
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_provenance_tracking() {
        let patch = Patch::from_operations_with_provenance(
            vec![Operation::AddNode(make_gain())],
            PatchSource::AiGenerated("gpt-4".into()),
            TrustLevel::ReviewRequired,
        );
        assert_eq!(patch.source, PatchSource::AiGenerated("gpt-4".into()));
        assert_eq!(patch.trust, TrustLevel::ReviewRequired);
    }
}
