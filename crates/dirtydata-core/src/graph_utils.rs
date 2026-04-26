<<<<<<< Updated upstream
use crate::ir::{EdgeKind, Graph};
=======
use crate::ir::{Graph, EdgeKind};
>>>>>>> Stashed changes
use crate::types::StableId;
use std::collections::{HashMap, HashSet, VecDeque};

/// Sorts the graph nodes topologically.
/// If cycles are detected, they are returned as well.
pub fn topological_sort(graph: &Graph) -> (Vec<StableId>, Vec<Vec<StableId>>) {
    let mut in_degree = HashMap::new();
    let mut adj = HashMap::new();
    let mut all_nodes = HashSet::new();

    for id in graph.nodes.keys() {
        all_nodes.insert(*id);
        in_degree.insert(*id, 0);
        adj.insert(*id, Vec::new());
    }

    for edge in graph.edges.values() {
        if edge.kind == EdgeKind::Normal {
<<<<<<< Updated upstream
            adj.get_mut(&edge.source.node_id)
                .unwrap()
                .push(edge.target.node_id);
=======
            adj.get_mut(&edge.source.node_id).unwrap().push(edge.target.node_id);
>>>>>>> Stashed changes
            *in_degree.get_mut(&edge.target.node_id).unwrap() += 1;
        }
    }

    let mut queue = VecDeque::new();
    for (id, degree) in &in_degree {
        if *degree == 0 {
            queue.push_back(*id);
        }
    }

    let mut sorted = Vec::new();
    while let Some(u) = queue.pop_front() {
        sorted.push(u);
        if let Some(neighbors) = adj.get(&u) {
            for &v in neighbors {
                let degree = in_degree.get_mut(&v).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(v);
                }
            }
        }
    }

    // Detect cycles
    let mut cycles = Vec::new();
    if sorted.len() < all_nodes.len() {
        // Simple cycle detection: nodes with remaining in-degree are part of cycles
        let remaining: HashSet<_> = all_nodes
            .into_iter()
            .filter(|id| !sorted.contains(id))
            .collect();
        // For MVP, we just return them as a single group of "cyclic nodes"
        // In a real system, we'd use Tarjan's or similar to find SCCs.
        cycles.push(remaining.into_iter().collect());
    }

    (sorted, cycles)
}

/// Returns all nodes that are upstream (ancestors) of the target node, including the target itself.
/// Only considers Normal edges (not Feedback) to avoid capturing entire feedback loops if not necessary.
pub fn get_upstream_nodes(graph: &Graph, target_node: StableId) -> HashSet<StableId> {
    let mut upstream = HashSet::new();
    let mut stack = vec![target_node];

    while let Some(current) = stack.pop() {
        if upstream.insert(current) {
            for edge in graph.edges.values() {
                if edge.target.node_id == current && edge.kind == EdgeKind::Normal {
                    stack.push(edge.source.node_id);
                }
            }
        }
    }
    upstream
}

/// Creates a new minimal Graph containing only the specified nodes and the edges connecting them.
pub fn clone_subgraph(graph: &Graph, node_ids: &HashSet<StableId>) -> Graph {
    let mut new_graph = Graph::new();
    
    for &id in node_ids {
        if let Some(node) = graph.nodes.get(&id) {
            new_graph.nodes.insert(id, node.clone());
        }
    }
    
    for edge in graph.edges.values() {
        if node_ids.contains(&edge.source.node_id) && node_ids.contains(&edge.target.node_id) {
            new_graph.edges.insert(edge.id, edge.clone());
        }
    }
    
    // Copy modulations if both source and target are in the subgraph
    for m in graph.modulations.values() {
        if node_ids.contains(&m.source.node_id) && node_ids.contains(&m.target_node) {
            new_graph.modulations.insert(m.id, m.clone());
        }
    }
    
    new_graph
}
