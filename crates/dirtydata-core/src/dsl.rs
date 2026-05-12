//! Surface DSL — Layer 2: Human Review Language.
//!
//! Authoring Language ではなく Review Language。
//! 吸うな。その薬は強い。export-only。

use std::fmt::Write;

use crate::actions::node_name;
use crate::hash;
use crate::ir::Graph;
use crate::types::*;

/// Render the graph as Surface DSL text.
pub fn render_dsl(graph: &Graph) -> String {
    let mut out = String::new();

    // Header
    let _ = writeln!(
        out,
        "# DirtyData Surface DSL — revision {}",
        graph.revision.0
    );
    let _ = writeln!(
        out,
        "# Hash: blake3:{}",
        hex_short(&hash::hash_graph(graph))
    );
    let _ = writeln!(out, "# Patches: {}", graph.lineage.applied_patches.len());
    let _ = writeln!(out);

    // Build name lookup for connections
    let name_of = |id: &StableId| -> String {
        graph
            .topology
            .nodes
            .get(id)
            .map(node_name)
            .unwrap_or_else(|| id.to_string())
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
                let _ = writeln!(out, "foreign \"{}\" \"{}\" {{", node_name(node), name);
                render_node_body(&mut out, node);
                let _ = writeln!(out, "}}");
                let _ = writeln!(out);
                continue;
            }
            NodeKind::Intent => "intent",
            NodeKind::Metadata => "metadata",
            NodeKind::Boundary => "boundary",
            NodeKind::SubGraph => "subgraph",
            NodeKind::InputProxy => "input_proxy",
            NodeKind::OutputProxy => "output_proxy",
            NodeKind::CircuitModule { .. } => "circuit_module",
        };

        let _ = writeln!(out, "{} \"{}\" {{ # id: {}", kind_str, node_name(node), node.id);
        render_node_body(&mut out, node);
        let _ = writeln!(out, "}}");
        let _ = writeln!(out);
    }

    // Edges
    if !graph.topology.edges.is_empty() {
        let _ = writeln!(out, "# Connections");
        for edge in graph.topology.edges.values() {
            let src_name = name_of(&edge.source.node_id);
            let tgt_name = name_of(&edge.target.node_id);

            let kind_tag = "# normal";

            let _ = writeln!(
                out,
                "{}.{} -> {}.{}  {}",
                src_name, edge.source.port_name, tgt_name, edge.target.port_name, kind_tag
            );
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
        if port.name == dir {
            let _ = writeln!(out, "  {}: {} {}", dir, dtype, domain);
        } else {
            let _ = writeln!(out, "  {} \"{}\": {} {}", dir, port.name, dtype, domain);
        }
    }

    // Config
    let config_entries: Vec<_> = node
        .config
        .iter()
        .filter(|(k, _)| k.as_str() != "name")
        .collect();

    if !config_entries.is_empty() {
        let _ = writeln!(out, "  config {{");
        for (key, value) in config_entries {
            let _ = writeln!(out, "    {}: {}", key, format_config_value(value));
        }
        let _ = writeln!(out, "  }}");
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
