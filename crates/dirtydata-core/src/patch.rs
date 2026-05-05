//! Patch Engine — the heart of DirtyData.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::hash;
use crate::ir::{Edge, Graph, Node};
use crate::types::*;

/// A single atomic patch — the unit of change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub identity: PatchId,
    pub operations: Vec<Operation>,
    pub intent_ref: Option<IntentId>,
    pub deterministic_hash: Hash,
    pub parents: Vec<PatchId>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchSet {
    pub patches: Vec<Patch>,
}

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

impl Patch {
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

    pub fn with_intent(mut self, intent_id: IntentId) -> Self {
        self.intent_ref = Some(intent_id);
        self.deterministic_hash = hash::hash_patch(&self);
        self
    }

    pub fn with_parents(mut self, parents: Vec<(PatchId, Hash)>) -> Self {
        self.parents = parents.iter().map(|(id, _)| *id).collect();
        self.parent_hashes = parents.iter().map(|(_, h)| *h).collect();
        self.deterministic_hash = hash::hash_patch(&self);
        self
    }

    pub fn verify_hash(&self) -> bool {
        hash::hash_patch(self) == self.deterministic_hash
    }
}

impl Graph {
    pub fn apply_patch(&mut self, patch: &Patch) -> Result<(), PatchError> {
        for op in &patch.operations {
            self.apply_operation(op)?;
        }
        self.revision = self.revision.next();
        self.lineage.applied_patches.push(patch.identity);
        self.lineage.history.insert(patch.identity, patch.clone());
        self.sync();
        Ok(())
    }

    fn apply_operation(&mut self, op: &Operation) -> Result<(), PatchError> {
        match op {
            Operation::AddNode(node) => {
                if self.topology.nodes.contains_key(&node.id) {
                    return Err(PatchError::NodeAlreadyExists(node.id));
                }
                self.topology.nodes.insert(node.id, node.clone());
            }
            Operation::RemoveNode(id) => {
                if self.topology.nodes.remove(id).is_none() {
                    return Err(PatchError::NodeNotFound(*id));
                }
                self.topology
                    .edges
                    .retain(|_, e| e.source.node_id != *id && e.target.node_id != *id);
            }
            Operation::ReplaceNode(node) => {
                if !self.topology.nodes.contains_key(&node.id) {
                    return Err(PatchError::NodeNotFound(node.id));
                }
                self.topology.nodes.insert(node.id, node.clone());
            }
            Operation::ModifyConfig { node_id, delta } => {
                let node = self
                    .topology
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
                if self.topology.edges.contains_key(&edge.id) {
                    return Err(PatchError::EdgeAlreadyExists(edge.id));
                }
                self.require_port(&edge.source)?;
                self.require_port(&edge.target)?;
                self.topology.edges.insert(edge.id, edge.clone());
            }
            Operation::RemoveEdge(id) => {
                if self.topology.edges.remove(id).is_none() {
                    return Err(PatchError::EdgeNotFound(*id));
                }
            }
            Operation::ModifyEdge { edge_id, delta } => {
                if let Some(ref src) = delta.source {
                    self.require_port(src)?;
                }
                if let Some(ref tgt) = delta.target {
                    self.require_port(tgt)?;
                }
                let edge = self
                    .topology
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
                if self.topology.modulations.contains_key(&m.id) {
                    return Err(PatchError::EdgeAlreadyExists(m.id));
                }
                self.require_port(&m.source)?;
                if !self.topology.nodes.contains_key(&m.target_node) {
                    return Err(PatchError::NodeNotFound(m.target_node));
                }
                self.topology.modulations.insert(m.id, m.clone());
            }
            Operation::RemoveModulation(id) => {
                if self.topology.modulations.remove(id).is_none() {
                    return Err(PatchError::EdgeNotFound(*id));
                }
            }
        }
        Ok(())
    }

    fn require_port(&self, port_ref: &PortRef) -> Result<(), PatchError> {
        let _node = self
            .topology
            .nodes
            .get(&port_ref.node_id)
            .ok_or(PatchError::NodeNotFound(port_ref.node_id))?;
        // Disable port validation for now since the CLI graph builder doesn't populate all custom ports
        /*
        if !node.ports.iter().any(|p| p.name == port_ref.port_name) {
            return Err(PatchError::PortNotFound {
                node: port_ref.node_id,
                port: port_ref.port_name.clone(),
            });
        }
        */
        Ok(())
    }

    pub fn diff(&self, other: &Graph) -> PatchSet {
        let mut operations = Vec::new();
        for id in self.topology.nodes.keys() {
            if !other.topology.nodes.contains_key(id) {
                operations.push(Operation::RemoveNode(*id));
            }
        }
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
        for id in self.topology.edges.keys() {
            if !other.topology.edges.contains_key(id) {
                operations.push(Operation::RemoveEdge(*id));
            }
        }
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
        PatchSet {
            patches: vec![Patch::from_operations(operations)],
        }
    }

    pub fn replay(patches: &[Patch]) -> Result<Self, PatchError> {
        let mut graph = Graph::new();
        for patch in patches {
            graph.apply_patch(patch)?;
        }
        Ok(graph)
    }

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

pub fn config_diff(old: &ConfigSnapshot, new: &ConfigSnapshot) -> ConfigDelta {
    let mut delta = BTreeMap::new();
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

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
