//! Deterministic hashing using BLAKE3.
//!
//! Git に人生相談してはいけない。
//! DirtyData の因果の鎖は BLAKE3 で繋ぐ。

use blake3::Hasher;

use crate::ir::{Edge, Graph, Node};
use crate::patch::{Operation, Patch};

/// Hash arbitrary bytes.
pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

/// Hash a patch deterministically.
/// The hash covers operations, intent, and parents — but NOT the identity or timestamp.
/// This means the same logical change always produces the same hash.
pub fn hash_patch(patch: &Patch) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"dirtydata:patch:v2:");

    // Hash operations in order
    for op in &patch.operations {
        hash_operation(&mut hasher, op);
    }

    // Hash intent reference
    if let Some(ref intent_id) = patch.intent_ref {
        hasher.update(b"intent:");
        hasher.update(intent_id.0.to_string().as_bytes());
    }

    // Hash parents (Merkle DAG lineage)
    for (id, hash) in patch.parents.iter().zip(patch.parent_hashes.iter()) {
        hasher.update(b"parent:");
        hasher.update(id.0.to_string().as_bytes());
        hasher.update(b":hash:");
        hasher.update(hash);
    }

    *hasher.finalize().as_bytes()
}

fn hash_operation(hasher: &mut Hasher, op: &Operation) {
    match op {
        Operation::AddNode(node) => {
            hasher.update(b"op:add_node:");
            hash_node(hasher, node);
        }
        Operation::RemoveNode(id) => {
            hasher.update(b"op:remove_node:");
            hasher.update(id.0.to_string().as_bytes());
        }
        Operation::ReplaceNode(node) => {
            hasher.update(b"op:replace_node:");
            hash_node(hasher, node);
        }
        Operation::ModifyConfig { node_id, delta } => {
            hasher.update(b"op:modify_config:");
            hasher.update(node_id.0.to_string().as_bytes());
            // BTreeMap guarantees deterministic key ordering.
            // Serialize to JSON for canonical byte representation.
            let json = serde_json::to_string(delta).unwrap_or_default();
            hasher.update(json.as_bytes());
        }
        Operation::AddEdge(edge) => {
            hasher.update(b"op:add_edge:");
            hash_edge(hasher, edge);
        }
        Operation::RemoveEdge(id) => {
            hasher.update(b"op:remove_edge:");
            hasher.update(id.0.to_string().as_bytes());
        }
        Operation::ModifyEdge { edge_id, delta } => {
            hasher.update(b"op:modify_edge:");
            hasher.update(edge_id.0.to_string().as_bytes());
            let json = serde_json::to_string(delta).unwrap_or_default();
            hasher.update(json.as_bytes());
        }
        Operation::AddModulation(m) => {
            hasher.update(b"op:add_modulation:");
            hash_modulation(hasher, m);
        }
        Operation::RemoveModulation(id) => {
            hasher.update(b"op:remove_modulation:");
            hasher.update(id.0.to_string().as_bytes());
        }
    }
}

fn hash_node(hasher: &mut Hasher, node: &Node) {
    hasher.update(b"node:");
    hasher.update(node.id.0.to_string().as_bytes());

    let kind_json = serde_json::to_string(&node.kind).unwrap_or_default();
    hasher.update(kind_json.as_bytes());

    // Ports — order matters
    for port in &node.ports {
        hasher.update(b"port:");
        hasher.update(port.name.as_bytes());
        let dir = serde_json::to_string(&port.direction).unwrap_or_default();
        hasher.update(dir.as_bytes());
        let domain = serde_json::to_string(&port.domain).unwrap_or_default();
        hasher.update(domain.as_bytes());
        let dtype = serde_json::to_string(&port.data_type).unwrap_or_default();
        hasher.update(dtype.as_bytes());
    }

    // Config — BTreeMap ordering is deterministic
    let config_json = serde_json::to_string(&node.config).unwrap_or_default();
    hasher.update(config_json.as_bytes());

    // Metadata ref
    let meta = serde_json::to_string(&node.metadata).unwrap_or_default();
    hasher.update(meta.as_bytes());
}

fn hash_edge(hasher: &mut Hasher, edge: &Edge) {
    hasher.update(b"edge:");
    hasher.update(edge.id.0.to_string().as_bytes());
    hasher.update(edge.source.node_id.0.to_string().as_bytes());
    hasher.update(b":");
    hasher.update(edge.source.port_name.as_bytes());
    hasher.update(b"->");
    hasher.update(edge.target.node_id.0.to_string().as_bytes());
    hasher.update(b":");
    hasher.update(edge.target.port_name.as_bytes());
    hasher.update(&[edge.kind as u8]);
}

fn hash_modulation(hasher: &mut Hasher, m: &crate::ir::Modulation) {
    hasher.update(b"modulation:");
    hasher.update(m.id.0.to_string().as_bytes());
    hasher.update(m.source.node_id.0.to_string().as_bytes());
    hasher.update(b":");
    hasher.update(m.source.port_name.as_bytes());
    hasher.update(b"->param:");
    hasher.update(m.target_node.0.to_string().as_bytes());
    hasher.update(b":");
    hasher.update(m.target_param.as_bytes());
    hasher.update(&m.amount.to_le_bytes());
}

/// Hash an entire graph for integrity verification.
/// BTreeMap iteration gives deterministic key order.
pub fn hash_graph(graph: &Graph) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"dirtydata:graph:v1:");

    for node in graph.nodes.values() {
        hash_node(&mut hasher, node);
    }
    for edge in graph.edges.values() {
        hash_edge(&mut hasher, edge);
    }
    for m in graph.modulations.values() {
        hash_modulation(&mut hasher, m);
    }

    hasher.update(&graph.revision.0.to_le_bytes());

    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Node;
    use crate::patch::Operation;

    #[test]
    fn test_graph_hash_deterministic() {
        let mut g = Graph::new();
        let node = Node::new_source("Sine");
        let patch = Patch::from_operations(vec![Operation::AddNode(node)]);
        g.apply_patch(&patch).unwrap();

        let h1 = hash_graph(&g);
        let h2 = hash_graph(&g);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_graphs_different_hashes() {
        let mut g1 = Graph::new();
        g1.apply_patch(&Patch::from_operations(vec![Operation::AddNode(
            Node::new_source("Sine"),
        )]))
        .unwrap();

        let mut g2 = Graph::new();
        g2.apply_patch(&Patch::from_operations(vec![Operation::AddNode(
            Node::new_source("Noise"),
        )]))
        .unwrap();

        assert_ne!(hash_graph(&g1), hash_graph(&g2));
    }

    #[test]
    fn test_patch_hash_stable() {
        let node = Node::new_processor("EQ");
        let patch = Patch::from_operations(vec![Operation::AddNode(node.clone())]);
        let h1 = hash_patch(&patch);
        let h2 = hash_patch(&patch);
        assert_eq!(h1, h2);
    }
}
