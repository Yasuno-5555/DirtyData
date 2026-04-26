//! Commit Validation — §6.
//!
//! "Fail/Pass の二元論は現場を壊す。人類はグレーで生きてる。"
//!
//! Commit は以下を全通過しなければならない:
//! - Topology Check (cycle detection, isolated side effects)
//! - Type & Domain Safety (domain crossing validation)
//! - Dependency Closure (asset existence, hash recording)
//! - Deterministic Replayability

use std::collections::{HashMap, HashSet, VecDeque};

use crate::hash;
use crate::ir::Graph;
use crate::patch::Patch;
use crate::types::*;

// ──────────────────────────────────────────────
// §6 ValidationReport — not binary pass/fail
// ──────────────────────────────────────────────

/// Commit validation result.
/// Errors block commit. Warnings don't. Confidence debt is tracked.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub confidence_debt: Vec<ConfidenceDebt>,
    pub replay_proof: Option<ReplayProof>,
}

impl ValidationReport {
    /// Can this graph be committed?
    /// Only errors block — warnings and debt are informational.
    pub fn is_committable(&self) -> bool {
        self.errors.is_empty()
    }

    /// Total confidence debt score.
    pub fn total_debt(&self) -> u32 {
        self.confidence_debt.iter().map(|d| d.weight).sum()
    }
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub code: &'static str,
    pub message: String,
    pub node: Option<StableId>,
}

#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub code: &'static str,
    pub message: String,
    pub node: Option<StableId>,
}

/// Confidence debt — things we can't fully verify but won't block.
#[derive(Debug, Clone)]
pub struct ConfidenceDebt {
    pub source: StableId,
    pub reason: String,
    pub confidence: ConfidenceScore,
    pub weight: u32,
}

/// Proof that the graph can be deterministically replayed.
#[derive(Debug, Clone)]
pub struct ReplayProof {
    pub graph_hash: Hash,
    pub patch_count: usize,
    pub replayed_hash: Hash,
    pub matches: bool,
}

// ──────────────────────────────────────────────
// Main validation entry point
// ──────────────────────────────────────────────

/// Validate a graph for commit readiness.
pub fn validate_commit(graph: &Graph, patches: &[Patch]) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut confidence_debt = Vec::new();

    // §6.1 Topology Check
    check_topology(graph, &mut errors, &mut warnings);

    // §6.2 Type & Domain Safety
    check_domain_safety(graph, &mut errors, &mut warnings);

    // §6.3 Dependency Closure
    check_dependencies(graph, &mut errors, &mut warnings, &mut confidence_debt);

    // §6.4 Deterministic Replayability
    let replay_proof = check_determinism(graph, patches);

    if let Some(ref proof) = replay_proof {
        if !proof.matches {
            errors.push(ValidationError {
                code: "REPLAY_MISMATCH",
                message: format!(
                    "replay produced different hash: expected {}, got {}",
                    hex_encode(&proof.graph_hash),
                    hex_encode(&proof.replayed_hash)
                ),
                node: None,
            });
        }
    }

    ValidationReport {
        errors,
        warnings,
        confidence_debt,
        replay_proof,
    }
}

// ──────────────────────────────────────────────
// §6.1 Topology Check
// ──────────────────────────────────────────────

fn check_topology(
    graph: &Graph,
    errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<ValidationWarning>,
) {
    // Cycle detection via topological sort (Kahn's algorithm)
    if !graph.edges.is_empty() {
        let mut in_degree: HashMap<StableId, usize> = HashMap::new();
        let mut adjacency: HashMap<StableId, Vec<StableId>> = HashMap::new();

        // Initialize all nodes
        for id in graph.nodes.keys() {
            in_degree.entry(*id).or_insert(0);
            adjacency.entry(*id).or_default();
        }

        // Build adjacency from normal edges only (Feedback edges don't carry causal dependency)
        for edge in graph.edges.values() {
            if edge.kind == crate::ir::EdgeKind::Normal {
                adjacency
                    .entry(edge.source.node_id)
                    .or_default()
                    .push(edge.target.node_id);
                *in_degree.entry(edge.target.node_id).or_insert(0) += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<StableId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut visited = 0;
        while let Some(node) = queue.pop_front() {
            visited += 1;
            if let Some(neighbors) = adjacency.get(&node) {
                for &next in neighbors {
                    if let Some(deg) = in_degree.get_mut(&next) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(next);
                        }
                    }
                }
            }
        }

        if visited < graph.nodes.len() {
            errors.push(ValidationError {
                code: "CYCLE_DETECTED",
                message: format!(
                    "causal cycle detected: {} nodes unreachable in topological sort",
                    graph.nodes.len() - visited
                ),
                node: None,
            });
        }
    }

    // Isolated nodes warning (nodes with no edges)
    let connected: HashSet<StableId> = graph
        .edges
        .values()
        .flat_map(|e| [e.source.node_id, e.target.node_id])
        .collect();

    for id in graph.nodes.keys() {
        if !connected.contains(id) && graph.nodes.len() > 1 {
            warnings.push(ValidationWarning {
                code: "ISOLATED_NODE",
                message: format!("node {} has no connections", id),
                node: Some(*id),
            });
        }
    }
}

// ──────────────────────────────────────────────
// §6.2 Type & Domain Safety
// ──────────────────────────────────────────────

fn check_domain_safety(
    graph: &Graph,
    errors: &mut Vec<ValidationError>,
    _warnings: &mut Vec<ValidationWarning>,
) {
    for edge in graph.edges.values() {
        let src_port = graph
            .nodes
            .get(&edge.source.node_id)
            .and_then(|n| n.ports.iter().find(|p| p.name == edge.source.port_name));

        let tgt_port = graph
            .nodes
            .get(&edge.target.node_id)
            .and_then(|n| n.ports.iter().find(|p| p.name == edge.target.port_name));

        if let (Some(src), Some(tgt)) = (src_port, tgt_port) {
            // Domain crossing check
            if src.domain != tgt.domain {
                errors.push(ValidationError {
                    code: "DOMAIN_CROSSING",
                    message: format!(
                        "edge {} crosses domains: {:?} -> {:?} (requires explicit bridge)",
                        edge.id, src.domain, tgt.domain
                    ),
                    node: None,
                });
            }

            // Direction check — source must be Output, target must be Input
            if src.direction != PortDirection::Output {
                errors.push(ValidationError {
                    code: "PORT_DIRECTION",
                    message: format!(
                        "edge {} source port '{}' is not an output",
                        edge.id, edge.source.port_name
                    ),
                    node: Some(edge.source.node_id),
                });
            }
            if tgt.direction != PortDirection::Input {
                errors.push(ValidationError {
                    code: "PORT_DIRECTION",
                    message: format!(
                        "edge {} target port '{}' is not an input",
                        edge.id, edge.target.port_name
                    ),
                    node: Some(edge.target.node_id),
                });
            }
        }
    }
}

// ──────────────────────────────────────────────
// §6.3 Dependency Closure
// ──────────────────────────────────────────────

fn check_dependencies(
    graph: &Graph,
    _errors: &mut Vec<ValidationError>,
    _warnings: &mut Vec<ValidationWarning>,
    confidence_debt: &mut Vec<ConfidenceDebt>,
) {
    // Check Foreign nodes — they carry inherent confidence debt
    for (id, node) in &graph.nodes {
        if let NodeKind::Foreign(plugin_name) = &node.kind {
            confidence_debt.push(ConfidenceDebt {
                source: *id,
                reason: format!(
                    "foreign plugin '{}' is nondeterministic by default",
                    plugin_name
                ),
                confidence: ConfidenceScore::Suspicious,
                weight: 30,
            });
        }
    }

    // Check dangling edge references
    for edge in graph.edges.values() {
        if !graph.nodes.contains_key(&edge.source.node_id) {
            confidence_debt.push(ConfidenceDebt {
                source: edge.id,
                reason: format!("edge source node {} missing", edge.source.node_id),
                confidence: ConfidenceScore::Unknown,
                weight: 100,
            });
        }
        if !graph.nodes.contains_key(&edge.target.node_id) {
            confidence_debt.push(ConfidenceDebt {
                source: edge.id,
                reason: format!("edge target node {} missing", edge.target.node_id),
                confidence: ConfidenceScore::Unknown,
                weight: 100,
            });
        }
    }
}

// ──────────────────────────────────────────────
// §6.4 Deterministic Replayability
// ──────────────────────────────────────────────

fn check_determinism(graph: &Graph, patches: &[Patch]) -> Option<ReplayProof> {
    if patches.is_empty() {
        return None;
    }

    let graph_hash = hash::hash_graph(graph);

    match Graph::replay(patches) {
        Ok(replayed) => {
            let replayed_hash = hash::hash_graph(&replayed);
            Some(ReplayProof {
                graph_hash,
                patch_count: patches.len(),
                replayed_hash,
                matches: graph_hash == replayed_hash,
            })
        }
        Err(_) => {
            // Replay itself failed — still produce a proof
            Some(ReplayProof {
                graph_hash,
                patch_count: patches.len(),
                replayed_hash: [0u8; 32],
                matches: false,
            })
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Edge, Node};
    use crate::patch::{Operation, Patch};

    #[test]
    fn test_valid_linear_graph() {
        let src = Node::new_source("Sine");
        let gain = Node::new_processor("Gain");
        let sink = Node::new_sink("Output");

        let e1 = Edge::new(
            PortRef {
                node_id: src.id,
                port_name: "out".into(),
            },
            PortRef {
                node_id: gain.id,
                port_name: "in".into(),
            },
        );
        let e2 = Edge::new(
            PortRef {
                node_id: gain.id,
                port_name: "out".into(),
            },
            PortRef {
                node_id: sink.id,
                port_name: "in".into(),
            },
        );

        let patch = Patch::from_operations(vec![
            Operation::AddNode(src),
            Operation::AddNode(gain),
            Operation::AddNode(sink),
            Operation::AddEdge(e1),
            Operation::AddEdge(e2),
        ]);

        let mut graph = Graph::new();
        graph.apply(&patch).unwrap();

        let report = validate_commit(&graph, &[patch]);
        assert!(report.is_committable(), "errors: {:?}", report.errors);
        assert!(report.replay_proof.is_some());
        assert!(report.replay_proof.unwrap().matches);
    }

    #[test]
    fn test_isolated_node_warning() {
        let src = Node::new_source("Sine");
        let orphan = Node::new_processor("Orphan");

        let patch =
            Patch::from_operations(vec![Operation::AddNode(src), Operation::AddNode(orphan)]);

        let mut graph = Graph::new();
        graph.apply(&patch).unwrap();

        let report = validate_commit(&graph, &[patch]);
        assert!(report.is_committable()); // warnings don't block
        assert!(!report.warnings.is_empty());
        assert!(report.warnings.iter().any(|w| w.code == "ISOLATED_NODE"));
    }

    #[test]
    fn test_foreign_node_confidence_debt() {
        let foreign = Node {
            id: StableId::new(),
            kind: NodeKind::Foreign("SomeVST".into()),
            ports: vec![],
            config: Default::default(),
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        };

        let patch = Patch::from_operations(vec![Operation::AddNode(foreign)]);
        let mut graph = Graph::new();
        graph.apply(&patch).unwrap();

        let report = validate_commit(&graph, &[patch]);
        assert!(!report.confidence_debt.is_empty());
    }

    #[test]
    fn test_domain_crossing_error() {
        // Create nodes with different domains
        let src = Node {
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
            config: Default::default(),
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        };

        let analyzer = Node {
            id: StableId::new(),
            kind: NodeKind::Analyzer,
            ports: vec![TypedPort {
                name: "in".into(),
                direction: PortDirection::Input,
                domain: ExecutionDomain::Block, // Different domain!
                data_type: DataType::Audio { channels: 2 },
                semantic: PortSemantic::Signal,
                polarity: PortPolarity::Bipolar,
            }],
            config: Default::default(),
            metadata: MetadataRef(None),
            confidence: ConfidenceScore::Verified,
        };

        let edge = Edge::new(
            PortRef {
                node_id: src.id,
                port_name: "out".into(),
            },
            PortRef {
                node_id: analyzer.id,
                port_name: "in".into(),
            },
        );

        let patch = Patch::from_operations(vec![
            Operation::AddNode(src),
            Operation::AddNode(analyzer),
            Operation::AddEdge(edge),
        ]);

        let mut graph = Graph::new();
        graph.apply(&patch).unwrap();

        let report = validate_commit(&graph, &[patch]);
        assert!(!report.is_committable());
        assert!(report.errors.iter().any(|e| e.code == "DOMAIN_CROSSING"));
    }
}
