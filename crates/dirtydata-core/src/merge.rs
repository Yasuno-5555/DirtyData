use crate::patch::{Operation, Patch, PatchSet};
use crate::types::{ConfigDelta, StableId, EdgeDelta};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum MergeError {
    #[error("Conflict on node {node_id}: key '{key}' modified by both sides")]
    ConfigConflict { node_id: StableId, key: String },
    #[error("Conflict on edge {edge_id}: both sides modified this edge")]
    EdgeConflict { edge_id: StableId },
    #[error("Conflict on node {node_id}: one side removed, other side modified")]
    RemoveModifyConflict { node_id: StableId },
    #[error("Conflict: both sides added node {node_id} with different content")]
    AddNodeConflict { node_id: StableId },
    #[error("Conflict: both sides added edge {edge_id} with different content")]
    AddEdgeConflict { edge_id: StableId },
    #[error("Conflict on node {node_id}: both sides replaced this node")]
    ReplaceConflict { node_id: StableId },
}

pub fn merge_three_way(
    _base: &PatchSet,
    left: &PatchSet,
    right: &PatchSet,
) -> Result<PatchSet, MergeError> {
    let mut merged_ops = Vec::new();

    // Track what each side is doing
    let left_ops = collect_op_targets(&left.patches);
    let right_ops = collect_op_targets(&right.patches);

    // ──────────────────────────────────────────────
    // 1. Merge Node Operations
    // ──────────────────────────────────────────────
    let all_nodes: HashSet<_> = left_ops
        .nodes
        .keys()
        .chain(right_ops.nodes.keys())
        .cloned()
        .collect();

    for node_id in all_nodes {
        let l_mod = left_ops.nodes.get(&node_id);
        let r_mod = right_ops.nodes.get(&node_id);

        match (l_mod, r_mod) {
            (Some(l), Some(r)) => {
                if l.removed || r.removed {
                    return Err(MergeError::RemoveModifyConflict { node_id });
                }

                // AddNode Conflict Check
                if let (Some(l_add), Some(r_add)) = (&l.added, &r.added) {
                    if l_add != r_add {
                        return Err(MergeError::AddNodeConflict { node_id });
                    }
                    merged_ops.push(Operation::AddNode(l_add.clone()));
                }

                // ReplaceNode Conflict Check
                if let (Some(l_rep), Some(r_rep)) = (&l.replaced, &r.replaced) {
                    if l_rep != r_rep {
                        return Err(MergeError::ReplaceConflict { node_id });
                    }
                    merged_ops.push(Operation::ReplaceNode(l_rep.clone()));
                } else if let Some(l_rep) = &l.replaced {
                    merged_ops.push(Operation::ReplaceNode(l_rep.clone()));
                } else if let Some(r_rep) = &r.replaced {
                    merged_ops.push(Operation::ReplaceNode(r_rep.clone()));
                }

                // ModifyConfig Merge
                let mut merged_delta = l.config_delta.clone();
                for (key, r_change) in &r.config_delta {
                    if let Some(l_change) = l.config_delta.get(key) {
                        if l_change != r_change {
                            return Err(MergeError::ConfigConflict {
                                node_id,
                                key: key.clone(),
                            });
                        }
                    } else {
                        merged_delta.insert(key.clone(), r_change.clone());
                    }
                }

                if !merged_delta.is_empty() {
                    merged_ops.push(Operation::ModifyConfig {
                        node_id,
                        delta: merged_delta,
                    });
                }
            }
            (Some(l), None) => {
                if l.removed {
                    merged_ops.push(Operation::RemoveNode(node_id));
                } else {
                    if let Some(node) = &l.added {
                        merged_ops.push(Operation::AddNode(node.clone()));
                    }
                    if let Some(node) = &l.replaced {
                        merged_ops.push(Operation::ReplaceNode(node.clone()));
                    }
                    if !l.config_delta.is_empty() {
                        merged_ops.push(Operation::ModifyConfig {
                            node_id,
                            delta: l.config_delta.clone(),
                        });
                    }
                }
            }
            (None, Some(r)) => {
                if r.removed {
                    merged_ops.push(Operation::RemoveNode(node_id));
                } else {
                    if let Some(node) = &r.added {
                        merged_ops.push(Operation::AddNode(node.clone()));
                    }
                    if let Some(node) = &r.replaced {
                        merged_ops.push(Operation::ReplaceNode(node.clone()));
                    }
                    if !r.config_delta.is_empty() {
                        merged_ops.push(Operation::ModifyConfig {
                            node_id,
                            delta: r.config_delta.clone(),
                        });
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    // ──────────────────────────────────────────────
    // 2. Merge Edge Operations
    // ──────────────────────────────────────────────
    let all_edges: HashSet<_> = left_ops
        .edges
        .keys()
        .chain(right_ops.edges.keys())
        .cloned()
        .collect();

    for edge_id in all_edges {
        let l_mod = left_ops.edges.get(&edge_id);
        let r_mod = right_ops.edges.get(&edge_id);

        match (l_mod, r_mod) {
            (Some(l), Some(r)) => {
                if l.removed || r.removed {
                    return Err(MergeError::RemoveModifyConflict { node_id: edge_id });
                }

                // AddEdge Conflict Check
                if let (Some(l_add), Some(r_add)) = (&l.added, &r.added) {
                    if l_add != r_add {
                        return Err(MergeError::AddEdgeConflict { edge_id });
                    }
                    merged_ops.push(Operation::AddEdge(l_add.clone()));
                }

                // ModifyEdge Merge/Conflict
                if l.modified.is_some() || r.modified.is_some() {
                    if l.modified != r.modified {
                        return Err(MergeError::EdgeConflict { edge_id });
                    }
                    if let Some(delta) = &l.modified {
                        merged_ops.push(Operation::ModifyEdge { edge_id, delta: delta.clone() });
                    }
                }
            }
            (Some(l), None) => {
                if l.removed {
                    merged_ops.push(Operation::RemoveEdge(edge_id));
                } else {
                    if let Some(edge) = &l.added {
                        merged_ops.push(Operation::AddEdge(edge.clone()));
                    }
                    if let Some(delta) = &l.modified {
                        merged_ops.push(Operation::ModifyEdge { edge_id, delta: delta.clone() });
                    }
                }
            }
            (None, Some(r)) => {
                if r.removed {
                    merged_ops.push(Operation::RemoveEdge(edge_id));
                } else {
                    if let Some(edge) = &r.added {
                        merged_ops.push(Operation::AddEdge(edge.clone()));
                    }
                    if let Some(delta) = &r.modified {
                        merged_ops.push(Operation::ModifyEdge { edge_id, delta: delta.clone() });
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(PatchSet {
        patches: vec![Patch::from_operations_with_provenance(
            merged_ops,
            crate::types::PatchSource::System,
            crate::types::TrustLevel::Trusted,
        )],
    })
}

struct OpTargets {
    nodes: HashMap<StableId, NodeOpSummary>,
    edges: HashMap<StableId, EdgeOpSummary>,
}

struct NodeOpSummary {
    added: Option<crate::ir::Node>,
    removed: bool,
    replaced: Option<crate::ir::Node>,
    config_delta: ConfigDelta,
}

struct EdgeOpSummary {
    added: Option<crate::ir::Edge>,
    removed: bool,
    modified: Option<EdgeDelta>,
}

fn collect_op_targets(patches: &[Patch]) -> OpTargets {
    let mut nodes = HashMap::new();
    let mut edges = HashMap::new();

    for patch in patches {
        for op in &patch.operations {
            match op {
                Operation::AddNode(node) => {
                    nodes.entry(node.id).or_insert_with(|| NodeOpSummary {
                        added: Some(node.clone()),
                        removed: false,
                        replaced: None,
                        config_delta: BTreeMap::new(),
                    }).added = Some(node.clone());
                }
                Operation::RemoveNode(id) => {
                    nodes
                        .entry(*id)
                        .or_insert_with(|| NodeOpSummary {
                            added: None,
                            removed: true,
                            replaced: None,
                            config_delta: BTreeMap::new(),
                        })
                        .removed = true;
                }
                Operation::ReplaceNode(node) => {
                    nodes.entry(node.id).or_insert_with(|| NodeOpSummary {
                        added: None,
                        removed: false,
                        replaced: Some(node.clone()),
                        config_delta: BTreeMap::new(),
                    }).replaced = Some(node.clone());
                }
                Operation::ModifyConfig { node_id, delta } => {
                    let summary = nodes.entry(*node_id).or_insert_with(|| NodeOpSummary {
                        added: None,
                        removed: false,
                        replaced: None,
                        config_delta: BTreeMap::new(),
                    });
                    for (k, v) in delta {
                        summary.config_delta.insert(k.clone(), v.clone());
                    }
                }
                Operation::AddEdge(edge) => {
                    edges.entry(edge.id).or_insert_with(|| EdgeOpSummary {
                        added: Some(edge.clone()),
                        removed: false,
                        modified: None,
                    }).added = Some(edge.clone());
                }
                Operation::RemoveEdge(id) => {
                    edges.entry(*id).or_insert_with(|| EdgeOpSummary {
                        added: None,
                        removed: true,
                        modified: None,
                    }).removed = true;
                }
                Operation::ModifyEdge { edge_id, delta } => {
                    edges.entry(*edge_id).or_insert_with(|| EdgeOpSummary {
                        added: None,
                        removed: false,
                        modified: Some(delta.clone()),
                    }).modified = Some(delta.clone());
                }
                _ => {}
            }
        }
    }

    OpTargets { nodes, edges }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Node, Edge};
    use crate::types::PortRef;

    #[test]
    fn test_merge_add_node_conflict() {
        let node_id = StableId::new();
        let left_node = Node {
            id: node_id,
            kind: crate::types::NodeKind::Source,
            ports: Vec::new(),
            config: BTreeMap::new(),
            metadata: crate::types::MetadataRef(None),
            confidence: crate::types::ConfidenceScore::Verified,
        };
        let right_node = Node {
            id: node_id,
            kind: crate::types::NodeKind::Processor, // Different kind
            ports: Vec::new(),
            config: BTreeMap::new(),
            metadata: crate::types::MetadataRef(None),
            confidence: crate::types::ConfidenceScore::Verified,
        };

        let base = PatchSet { patches: Vec::new() };
        let left = PatchSet {
            patches: vec![Patch::from_operations(vec![Operation::AddNode(left_node)])],
        };
        let right = PatchSet {
            patches: vec![Patch::from_operations(vec![Operation::AddNode(right_node)])],
        };

        let result = merge_three_way(&base, &left, &right);
        assert_eq!(result.unwrap_err(), MergeError::AddNodeConflict { node_id });
    }

    #[test]
    fn test_merge_add_edge_conflict() {
        let edge_id = StableId::new();
        let src1 = PortRef { node_id: StableId::new(), port_name: "out".into() };
        let src2 = PortRef { node_id: StableId::new(), port_name: "out".into() };
        let tgt = PortRef { node_id: StableId::new(), port_name: "in".into() };

        let left_edge = Edge {
            id: edge_id,
            source: src1,
            target: tgt.clone(),
            kind: crate::ir::EdgeKind::Normal,
            modulations: BTreeMap::new(),
        };
        let right_edge = Edge {
            id: edge_id,
            source: src2, // Different source
            target: tgt,
            kind: crate::ir::EdgeKind::Normal,
            modulations: BTreeMap::new(),
        };

        let base = PatchSet { patches: Vec::new() };
        let left = PatchSet {
            patches: vec![Patch::from_operations(vec![Operation::AddEdge(left_edge)])],
        };
        let right = PatchSet {
            patches: vec![Patch::from_operations(vec![Operation::AddEdge(right_edge)])],
        };

        let result = merge_three_way(&base, &left, &right);
        assert_eq!(result.unwrap_err(), MergeError::AddEdgeConflict { edge_id });
    }

    #[test]
    fn test_merge_remove_edge_success() {
        let edge_id = StableId::new();
        let base = PatchSet { patches: Vec::new() };
        let left = PatchSet {
            patches: vec![Patch::from_operations(vec![Operation::RemoveEdge(edge_id)])],
        };
        let right = PatchSet { patches: Vec::new() };

        let result = merge_three_way(&base, &left, &right).unwrap();
        assert_eq!(result.patches[0].operations.len(), 1);
        assert_eq!(result.patches[0].operations[0], Operation::RemoveEdge(edge_id));
    }
}
