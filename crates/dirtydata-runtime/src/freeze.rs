use hound::{WavSpec, WavWriter};
use std::path::Path;

use crate::offline::OfflineRenderer;
use dirtydata_core::graph_utils;
use dirtydata_core::ir::{Edge, Graph, Node};
use dirtydata_core::patch::{Operation, Patch};
use dirtydata_core::types::{ConfigValue, PortRef, StableId};

#[derive(Debug, thiserror::Error)]
pub enum FreezeError {
    #[error("Node not found: {0}")]
    NodeNotFound(StableId),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Hound error: {0}")]
    Hound(#[from] hound::Error),
    #[error("Patch error: {0}")]
    Patch(#[from] dirtydata_core::patch::PatchError),
}

/// Freezes a node and its upstream dependencies into a WAV asset and returns a Patch
/// to replace the subgraph with an AssetReaderNode.
pub fn freeze_node(
    graph: &Graph,
    target_node_id: StableId,
    duration_secs: f32,
    sample_rate: f32,
    asset_path: &Path,
) -> Result<Patch, FreezeError> {
    // 1. Identify all nodes that need to be frozen (target + its ancestors)
    let ancestors = graph_utils::get_upstream_nodes(graph, target_node_id);
    if ancestors.is_empty() {
        return Err(FreezeError::NodeNotFound(target_node_id));
    }

    // 2. Clone the minimal subgraph for rendering
    let mut render_graph = graph_utils::clone_subgraph(graph, &ancestors);

    // 3. Add a temporary Sink node to capture target_node's output
    let sink_id = StableId::new();
    let sink_node = Node::new_sink("FreezeCaptureSink");
    render_graph.nodes.insert(sink_id, sink_node);

    // 4. Connect target_node to the capture Sink
    // Assume port "out" exists on target (standard for processors/sources)
    let edge = Edge::new(
        PortRef {
            node_id: target_node_id,
            port_name: "out".into(),
        },
        PortRef {
            node_id: sink_id,
            port_name: "in".into(),
        },
    );
    render_graph.edges.insert(edge.id, edge);

    // 5. Perform offline rendering
    let mut renderer = OfflineRenderer::new(render_graph, sample_rate);
    let audio_data = renderer.render(duration_secs);

    // 6. Save results to WAV file
    if let Some(parent) = asset_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let spec = WavSpec {
        channels: 2,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = WavWriter::create(asset_path, spec)?;
    for &sample in &audio_data {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;

    // 7. Construct Patch to transform the original graph
    let mut operations = Vec::new();

    // A. Remove all frozen nodes (upstream cascade will handle most edges)
    for &id in &ancestors {
        operations.push(Operation::RemoveNode(id));
    }

    // B. Add the replacement AssetReaderNode
    let asset_node_id = StableId::new();
    let mut asset_node = Node::new_source("FrozenAsset");
    asset_node.config.insert(
        "path".into(),
        ConfigValue::String(asset_path.to_string_lossy().into()),
    );
    asset_node.config.insert(
        "name".into(),
        ConfigValue::String(format!(
            "Frozen_{}",
            target_node_id.to_string()[..4].to_string()
        )),
    );
    operations.push(Operation::AddNode(asset_node));

    // C. Reconnect downstream consumers
    // We need to find edges in the original graph that were consuming target_node's output
    for edge in graph.edges.values() {
        // If the source was the target_node or one of its ancestors being removed...
        // AND the target is NOT one of the ancestors being removed...
        if ancestors.contains(&edge.source.node_id) && !ancestors.contains(&edge.target.node_id) {
            // Re-route this connection to come from our new FrozenAsset node
            let mut redirected_edge = edge.clone();
            redirected_edge.id = StableId::new(); // New ID for the new edge
            redirected_edge.source = PortRef {
                node_id: asset_node_id,
                port_name: "out".into(),
            };
            operations.push(Operation::AddEdge(redirected_edge));
        }
    }

    Ok(Patch::from_operations(operations))
}

pub struct DifferentialCache {
    /// Maps subgraph hash (blake3) to WAV asset path
    entries: std::collections::HashMap<[u8; 32], std::path::PathBuf>,
}

impl DifferentialCache {
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    pub fn get_cached_asset(&self, graph: &Graph) -> Option<std::path::PathBuf> {
        let hash = self.compute_graph_hash(graph);
        self.entries.get(&hash).cloned()
    }

    pub fn insert(&mut self, graph: &Graph, path: std::path::PathBuf) {
        let hash = self.compute_graph_hash(graph);
        self.entries.insert(hash, path);
    }

    fn compute_graph_hash(&self, graph: &Graph) -> [u8; 32] {
        // In a real implementation, this would walk the graph and hash nodes/edges
        let mut hasher = blake3::Hasher::new();
        hasher.update(&graph.revision.0.to_le_bytes());
        *hasher.finalize().as_bytes()
    }
}
