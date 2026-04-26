//! Surface DSL — Layer 2: Human Review Language.
//!
//! Authoring Language ではなく Review Language。
//! 吸うな。その薬は強い。export-only。
//!
//! 出力例:
//! ```text
//! source "Sine" {
//!   out: audio(2ch) @sample
//! }
//! processor "Gain" {
//!   in: audio(2ch) @sample
//!   out: audio(2ch) @sample
//!   config {
//!     gain_db: 2.0
//!   }
//! }
//! Sine.out -> Gain.in  # causal
//! ```

use std::fmt::Write;

use crate::actions::node_name;
use crate::hash;
use crate::ir::Graph;
use crate::types::*;

/// Render the graph as Surface DSL text.
pub fn render_dsl(graph: &Graph) -> String {
    let mut out = String::new();

    // Header
    writeln!(out, "# DirtyData Surface DSL — revision {}", graph.revision.0).unwrap();
    writeln!(out, "# Hash: blake3:{}", hex_short(&hash::hash_graph(graph))).unwrap();
    writeln!(out, "# Patches: {}", graph.lineage.applied_patches.len()).unwrap();
    writeln!(out).unwrap();

    // Build name lookup for connections
    let name_of = |id: &StableId| -> String {
        graph.topology.nodes.get(id).map(node_name).unwrap_or_else(|| id.to_string())
    };

    // Nodes
    for node in graph.topology.nodes.values() {
        let kind_str = match &node.kind {
            NodeKind::Source => "source",
            NodeKind::Processor => "processor",
            NodeKind::Analyzer => "analyzer",
            NodeKind::Sink => "sink",
            NodeKind::Junction => "junction",
            NodeKind::Foreign(name) => {
                writeln!(out, "foreign \"{}\" \"{}\" {{", node_name(node), name).unwrap();
                render_node_body(&mut out, node);
                writeln!(out, "}}").unwrap();
                writeln!(out).unwrap();
                continue;
            }
            NodeKind::Intent => "intent",
            NodeKind::Metadata => "metadata",
            NodeKind::Boundary => "boundary",
            NodeKind::SubGraph => "subgraph",
            NodeKind::InputProxy => "input_proxy",
            NodeKind::OutputProxy => "output_proxy",
                src_name, edge.source.port_name,
                tgt_name, edge.target.port_name,
                kind_tag
            )
            .unwrap();
        }
    }

    out
}

fn render_node_body(out: &mut String, node: &crate::ir::Node) {
    // Ports
    for port in &node.ports {
        let dir = match port.direction {
            PortDirection::Input => "in",
            PortDirection::Output => "out",
        };
        let domain = match port.domain {
            ExecutionDomain::Sample => "@sample",
            ExecutionDomain::Block => "@block",
            ExecutionDomain::Timeline => "@timeline",
            ExecutionDomain::Background => "@background",
        };
        let dtype = format_data_type(&port.data_type);
        // Only show port name if it differs from direction
        if port.name == dir {
            writeln!(out, "  {}: {} {}", dir, dtype, domain).unwrap();
        } else {
            writeln!(out, "  {} \"{}\": {} {}", dir, port.name, dtype, domain).unwrap();
        }
    }

    // Config (excluding "name" since it's in the header)
    let config_entries: Vec<_> = node
        .config
        .iter()
        .filter(|(k, _)| k.as_str() != "name")
        .collect();

    if !config_entries.is_empty() {
        writeln!(out, "  config {{").unwrap();
        for (key, value) in config_entries {
            writeln!(out, "    {}: {}", key, format_config_value(value)).unwrap();
        }
        writeln!(out, "  }}").unwrap();
    }
}

fn format_data_type(dt: &DataType) -> String {
    match dt {
        DataType::Audio { channels } => format!("audio({}ch)", channels),
        DataType::Control => "control".into(),
        DataType::Midi => "midi".into(),
        DataType::Spectral { bins } => format!("spectral({})", bins),
        DataType::Blob => "blob".into(),
        DataType::Meta => "meta".into(),
    }
}

fn format_config_value(v: &ConfigValue) -> String {
    match v {
        ConfigValue::Float(f) => format!("{}", f),
        ConfigValue::Int(i) => format!("{}", i),
        ConfigValue::Bool(b) => format!("{}", b),
        ConfigValue::String(s) => format!("\"{}\"", s),
        ConfigValue::List(items) => {
            let inner: Vec<_> = items.iter().map(format_config_value).collect();
            format!("[{}]", inner.join(", "))
        }
        ConfigValue::Map(map) => {
            let inner: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_config_value(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

fn hex_short(bytes: &[u8]) -> String {
    bytes[..8].iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Edge, Node};
    use crate::patch::{Operation, Patch};

    #[test]
    fn test_render_basic_chain() {
        let src = Node::new_source("Sine");
        let gain = Node::new_processor("Gain");
        let sink = Node::new_sink("Output");

        let edge1 = Edge::new(
            PortRef {
                node_id: src.id,
                port_name: "out".into(),
            },
            PortRef {
                node_id: gain.id,
                port_name: "in".into(),
            },
        );
        let edge2 = Edge::new(
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
            Operation::AddEdge(edge1),
            Operation::AddEdge(edge2),
        ]);

        let mut graph = Graph::new();
        graph.apply(&patch).unwrap();

        let dsl = render_dsl(&graph);

        assert!(dsl.contains("source \"Sine\""));
        assert!(dsl.contains("processor \"Gain\""));
        assert!(dsl.contains("sink \"Output\""));
        assert!(dsl.contains("Sine.out -> Gain.in"));
        assert!(dsl.contains("Gain.out -> Output.in"));
        assert!(dsl.contains("@sample"));
        assert!(dsl.contains("# normal"));
    }

    #[test]
    fn test_render_with_config() {
        let mut gain = Node::new_processor("Gain");
        gain.config
            .insert("gain_db".into(), ConfigValue::Float(2.5));

        let patch = Patch::from_operations(vec![Operation::AddNode(gain)]);
        let mut graph = Graph::new();
        graph.apply(&patch).unwrap();

        let dsl = render_dsl(&graph);
        assert!(dsl.contains("gain_db: 2.5"));
    }
}
