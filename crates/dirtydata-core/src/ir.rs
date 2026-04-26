//! Canonical IR — Layer 1: Machine Truth.
//!
//! The single Source of Truth.
//! Git manages it. The compiler interprets it. The runtime depends on it.
//!
//! GUI や DSL による直接上書きを禁止。
//! すべての変更は PatchSet を経由して適用される。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::types::*;

// ──────────────────────────────────────────────
// §5.1 — Node
// ──────────────────────────────────────────────

/// A node in the Canonical IR graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: StableId,
    pub kind: NodeKind,
    pub ports: Vec<TypedPort>,
    pub config: ConfigSnapshot,
    pub metadata: MetadataRef,
    pub confidence: ConfidenceScore,
}

impl Node {
    /// Create a minimal node with standard audio I/O ports.
    pub fn new_processor(name: &str) -> Self {
        Self {
            id: StableId::new(),
            kind: NodeKind::Processor,
            ports: vec![
                TypedPort {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    domain: ExecutionDomain::Sample,
                    data_type: DataType::Audio { channels: 2 },
                    semantic: PortSemantic::Signal,
                    polarity: PortPolarity::Bipolar,
                },
                TypedPort {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    domain: ExecutionDomain::Sample,
                    data_type: DataType::Audio { channels: 2 },
                    semantic: PortSemantic::Signal,
                    polarity: PortPolarity::Bipolar,
                },
            ],
            config: {
                let mut c = BTreeMap::new();
                c.insert("name".into(), ConfigValue::String(name.into()));
                c
            },
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        }
    }

    /// Create a source node (audio file, input device).
    pub fn new_source(name: &str) -> Self {
        Self {
            id: StableId::new(),
            kind: NodeKind::Source,
            ports: vec![TypedPort {
                name: "out".into(),
                direction: PortDirection::Output,
                domain: ExecutionDomain::Sample,
                data_type: DataType::Audio { channels: 2 },
                semantic: PortSemantic::Signal,
                polarity: PortPolarity::Bipolar,
            }],
            config: {
                let mut c = BTreeMap::new();
                c.insert("name".into(), ConfigValue::String(name.into()));
                c
            },
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        }
    }

    /// Create a sink node (output, export target).
    pub fn new_sink(name: &str) -> Self {
        Self {
            id: StableId::new(),
            kind: NodeKind::Sink,
            ports: vec![TypedPort {
                name: "in".into(),
                direction: PortDirection::Input,
                domain: ExecutionDomain::Sample,
                data_type: DataType::Audio { channels: 2 },
                semantic: PortSemantic::Signal,
                polarity: PortPolarity::Bipolar,
            }],
            config: {
                let mut c = BTreeMap::new();
                c.insert("name".into(), ConfigValue::String(name.into()));
                c
            },
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        }
    }

    pub fn new_subgraph(name: &str) -> Self {
        Self {
            id: StableId::new(),
            kind: NodeKind::SubGraph,
            ports: vec![
<<<<<<< Updated upstream
                TypedPort {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    domain: ExecutionDomain::Sample,
                    data_type: DataType::Audio { channels: 2 },
                },
                TypedPort {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    domain: ExecutionDomain::Sample,
                    data_type: DataType::Audio { channels: 2 },
                },
=======
                TypedPort { name: "in".into(), direction: PortDirection::Input, domain: ExecutionDomain::Sample, data_type: DataType::Audio { channels: 2 }, semantic: PortSemantic::Signal, polarity: PortPolarity::Bipolar },
                TypedPort { name: "out".into(), direction: PortDirection::Output, domain: ExecutionDomain::Sample, data_type: DataType::Audio { channels: 2 }, semantic: PortSemantic::Signal, polarity: PortPolarity::Bipolar },
>>>>>>> Stashed changes
            ],
            config: {
                let mut c = BTreeMap::new();
                c.insert("name".into(), ConfigValue::String(name.into()));
                c.insert("graph_json".into(), ConfigValue::String("{}".into()));
                c
            },
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        }
    }

    pub fn new_input_proxy(name: &str) -> Self {
        Self {
            id: StableId::new(),
            kind: NodeKind::InputProxy,
<<<<<<< Updated upstream
            ports: vec![TypedPort {
                name: "out".into(),
                direction: PortDirection::Output,
                domain: ExecutionDomain::Sample,
                data_type: DataType::Audio { channels: 2 },
            }],
=======
            ports: vec![TypedPort { name: "out".into(), direction: PortDirection::Output, domain: ExecutionDomain::Sample, data_type: DataType::Audio { channels: 2 }, semantic: PortSemantic::Signal, polarity: PortPolarity::Bipolar }],
>>>>>>> Stashed changes
            config: {
                let mut c = BTreeMap::new();
                c.insert("name".into(), ConfigValue::String(name.into()));
                c
            },
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        }
    }

    pub fn new_output_proxy(name: &str) -> Self {
        Self {
            id: StableId::new(),
            kind: NodeKind::OutputProxy,
<<<<<<< Updated upstream
            ports: vec![TypedPort {
                name: "in".into(),
                direction: PortDirection::Input,
                domain: ExecutionDomain::Sample,
                data_type: DataType::Audio { channels: 2 },
            }],
=======
            ports: vec![TypedPort { name: "in".into(), direction: PortDirection::Input, domain: ExecutionDomain::Sample, data_type: DataType::Audio { channels: 2 }, semantic: PortSemantic::Signal, polarity: PortPolarity::Bipolar }],
>>>>>>> Stashed changes
            config: {
                let mut c = BTreeMap::new();
                c.insert("name".into(), ConfigValue::String(name.into()));
                c
            },
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        }
    }
<<<<<<< Updated upstream
=======
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.config.get("name").and_then(|v| v.as_string()).map(|s| s.as_str()).unwrap_or("Unknown");
        write!(f, "Node({}: {:?}, id={})", name, self.kind, self.id)
    }
>>>>>>> Stashed changes
}

// ──────────────────────────────────────────────
// §5.2 — Edge
// ──────────────────────────────────────────────

/// The type of connection between ports.
<<<<<<< Updated upstream
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Normal feed-forward connection (causal dependency).
=======
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EdgeKind {
    /// Normal feed-forward connection (causal dependency).
    #[default]
>>>>>>> Stashed changes
    Normal,
    /// Feedback connection (1-sample delay, breaks DAG constraint).
    Feedback,
}

/// An edge connecting two ports in the Canonical IR graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: StableId,
    pub source: PortRef,
    pub target: PortRef,
<<<<<<< Updated upstream
=======
    #[serde(default)]
>>>>>>> Stashed changes
    pub kind: EdgeKind,
}

impl Edge {
    /// Create a causal edge between two ports.
    pub fn new(source: PortRef, target: PortRef) -> Self {
        Self {
            id: StableId::new(),
            source,
            target,
            kind: EdgeKind::Normal,
        }
    }

    /// Create a feedback edge (1-sample delay).
    pub fn new_feedback(source: PortRef, target: PortRef) -> Self {
        Self {
            id: StableId::new(),
            source,
            target,
            kind: EdgeKind::Feedback,
        }
    }
}

// ──────────────────────────────────────────────
// §5.3 — Modulation
// ──────────────────────────────────────────────

/// A modulation assignment between a source and a parameter.
/// This is "cable-less" modulation — Bitwig style.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modulation {
    pub id: StableId,
    pub source: PortRef,
    pub target_node: StableId,
    pub target_param: String,
    pub amount: f32,
}

impl Modulation {
    pub fn new(source: PortRef, target_node: StableId, target_param: String, amount: f32) -> Self {
        Self {
            id: StableId::new(),
            source,
            target_node,
            target_param,
            amount,
        }
    }
}

// ──────────────────────────────────────────────
// Canonical IR Graph
// ──────────────────────────────────────────────

/// Layer 1: Topology (Machine Truth - Current Space)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Topology {
    pub nodes: BTreeMap<StableId, Node>,
    pub edges: BTreeMap<StableId, Edge>,
<<<<<<< Updated upstream
    pub modulations: BTreeMap<StableId, Modulation>,
    pub revision: Revision,
    /// Ordered history of applied patches — explainability.
    /// "今この状態は何からできたか" が常に答えられること。
=======
    #[serde(default)]
    pub modulations: BTreeMap<StableId, Modulation>,
}

/// Layer 3: Lineage (Time's Genealogy - History DAG)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Lineage {
>>>>>>> Stashed changes
    pub applied_patches: Vec<PatchId>,
    pub history: BTreeMap<PatchId, crate::patch::Patch>,
    pub snapshots: BTreeMap<String, PatchId>,
}

/// Layer 2: Circuit Registry (The Mythic Blueprints)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CircuitRegistry {
    pub definitions: BTreeMap<StableId, crate::types::CircuitDefinition>,
}


/// The Canonical IR Graph — The Forensic Record of Sound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub spec_version: String,
    pub topology: Topology,
    pub lineage: Lineage,
    pub registry: CircuitRegistry,
    pub verification: Verification,
    pub revision: Revision,
    
    // Compatibility fields for immediate legacy support
    #[serde(skip)]
    pub nodes: BTreeMap<StableId, Node>,
    #[serde(skip)]
    pub edges: BTreeMap<StableId, Edge>,
    #[serde(skip)]
    pub modulations: BTreeMap<StableId, Modulation>,
}

impl Graph {
    /// Create an empty graph at revision zero.
    pub fn new() -> Self {
        Self {
            spec_version: "1.0.0".into(),
            topology: Topology::default(),
            lineage: Lineage::default(),
            registry: CircuitRegistry::default(),
            verification: Verification {
                null_test: true,
                hash: String::new(),
                trust_state: "verified".into(),
            },
            revision: Revision::zero(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            modulations: BTreeMap::new(),
<<<<<<< Updated upstream
            revision: Revision::zero(),
            applied_patches: Vec::new(),
=======
        }
    }

    /// Synchronizes legacy compatibility fields from the layered structure.
    /// Call this after deserializing or modifying layers.
    pub fn sync(&mut self) {
        self.nodes = self.topology.nodes.clone();
        self.edges = self.topology.edges.clone();
        self.modulations = self.topology.modulations.clone();
    }

    pub fn squash_history(&mut self) {
        let empty = Graph::new();
        let patch_set = empty.diff(self);
        
        if let Some(baseline) = patch_set.patches.first() {
            self.lineage.applied_patches = vec![baseline.identity];
            self.lineage.history.clear();
            self.lineage.history.insert(baseline.identity, baseline.clone());
        }
    }

    pub fn create_snapshot(&mut self, name: &str) {
        if let Some(&last_patch_id) = self.lineage.applied_patches.last() {
            self.lineage.snapshots.insert(name.to_string(), last_patch_id);
>>>>>>> Stashed changes
        }
    }

    pub fn node(&self, id: &StableId) -> Option<&Node> {
        self.topology.nodes.get(id)
    }

    pub fn edge(&self, id: &StableId) -> Option<&Edge> {
        self.topology.edges.get(id)
    }

    pub fn validate_port_ref(&self, port_ref: &PortRef) -> bool {
        self.topology.nodes
            .get(&port_ref.node_id)
            .map(|n| n.ports.iter().any(|p| p.name == port_ref.port_name))
            .unwrap_or(false)
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph(rev={}, nodes={}, edges={})", self.revision.0, self.nodes.len(), self.edges.len())
    }
}
