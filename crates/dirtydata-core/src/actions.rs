//! User-facing Action Schema.
//!
//! 内部臓器を見せるな。
//!
//! ユーザーは `Operation::AddNode(Node { id, kind, ports, config, metadata })`
//! なんて書かない。人間は書かない。
//!
//! ユーザーは以下を書く:
//! ```json
//! { "action": "add_processor", "name": "MainGain" }
//! ```
//!
//! このモジュールが UserAction → Vec<Operation> に変換する。

use serde::{Deserialize, Serialize};

use crate::types::{StableId};
use crate::ir::{Edge, Graph, Node};
use crate::patch::Operation;
use crate::types::*;
use crate::types::{ConfigDelta, StableId};

/// User-facing action — what humans write.
/// Internal operations are derived from these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UserAction {
    /// Add a source node (audio file, input).
    AddSource {
        name: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },

    /// Add a processor node (EQ, gain, compressor, etc).
    AddProcessor {
        name: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },

    /// Add an analyzer node (meter, spectrum).
    AddAnalyzer {
        name: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },

    /// Add a sink node (output, export target).
    AddSink {
        name: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },

    /// Add an external plugin node.
    AddForeign {
        name: String,
        plugin: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },

    /// Connect two nodes: source_name.port -> target_name.port
    Connect {
        from: String,
        from_port: Option<String>,
        to: String,
        to_port: Option<String>,
    },

    /// Disconnect two nodes.
    Disconnect {
        from: String,
        from_port: Option<String>,
        to: String,
        to_port: Option<String>,
    },

    /// Remove a node by name.
    RemoveNode { name: String },

    /// Freeze a node into an asset.
    FreezeNode { name: String, length_secs: f32 },

    /// Freeze a node into an asset.
    FreezeNode {
        name: String,
        length_secs: f32,
    },

    /// Set a config value on a node.
    SetConfig {
        node: String,
        key: String,
        value: serde_json::Value,
    },

    /// Add a modulation assignment (§5.3).
    AddModulation {
        source_node: String,
        source_port: String,
        target_node: String,
        target_param: String,
        amount: f32,
    },

<<<<<<< Updated upstream
    /// Replace a node with another kind, preserving connections if possible.
    ReplaceNode { name: String, new_kind_name: String },

    /// Add a container node.
    AddSubGraph { name: String },

    /// Remove a modulation assignment.
    RemoveModulation { id: StableId },

    /// Duplicate a node (Placeholder for GUI).
    DuplicateNode { node_id: StableId },
=======
    // --- §SSS: Engineering & Exploration ---
    
    /// Engineering: Run Monte Carlo manufacturing simulation.
    RunMonteCarlo { node_name: String, count: usize },
    /// Engineering: Run sensitivity analysis on circuit components.
    RunSensitivity { node_name: String },
    /// Engineering: Map the stability region of a parameter.
    RunStabilityMap { node_name: String, param: String, range_start: f32, range_end: f32 },
    /// Squash history into a single baseline patch.
    SquashHistory,
    /// Create a named snapshot (Safe Haven).
    CreateSnapshot {
        name: String,
    },

    /// Replace a node with another kind, preserving connections if possible.
    ReplaceNode {
        name: String,
        new_kind_name: String,
    },

    /// Add a container node.
    AddSubGraph {
        name: String,
    },

    /// Remove a modulation assignment.
    RemoveModulation {
        id: StableId,
    },

    /// Duplicate a node (Placeholder for GUI).
    DuplicateNode {
        node_id: StableId,
    },

    /// Checkout a specific revision.
    CheckoutRevision {
        revision: crate::types::PatchId,
    },
>>>>>>> Stashed changes
}

fn default_channels() -> u32 {
    2
}

/// A user-facing patch file — what the CLI reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPatchFile {
    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Intent description (optional).
    #[serde(default)]
    pub intent: Option<String>,

    /// Intent constraints (optional).
    #[serde(default)]
    pub constraints: Vec<UserConstraint>,

    /// The actions to perform.
    pub actions: Vec<UserAction>,
}

/// User-facing constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConstraint {
    #[serde(rename = "type")]
    pub kind: String, // "must", "prefer", "avoid", "never"
    pub description: String,
}

/// Errors during action compilation.
#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("node '{0}' not found in current graph")]
    NodeNotFound(String),

    #[error("ambiguous node name '{0}': multiple nodes match")]
    AmbiguousName(String),

    #[error("invalid config value for key '{0}': {1}")]
    InvalidConfig(String, String),
}

/// Compile user actions into internal operations.
///
/// This is the boundary between human intent and machine execution.
/// The graph is needed to resolve names → StableIds.
pub fn compile_actions(
    actions: &[UserAction],
    graph: &Graph,
) -> Result<Vec<Operation>, ActionError> {
    let mut ops = Vec::new();
    // Track nodes created in this batch (name → StableId)
    let mut created: std::collections::HashMap<String, StableId> = std::collections::HashMap::new();

    for action in actions {
        match action {
            UserAction::AddSource { name, channels } => {
                let node = make_node(NodeKind::Source, name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }

            UserAction::AddProcessor { name, channels } => {
                let node = make_node(NodeKind::Processor, name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }

            UserAction::AddAnalyzer { name, channels } => {
                let node = make_node(NodeKind::Analyzer, name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }

            UserAction::AddSink { name, channels } => {
                let node = make_node(NodeKind::Sink, name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }

            UserAction::AddForeign {
                name,
                plugin,
                channels,
            } => {
                let node = make_node(NodeKind::Foreign(plugin.clone()), name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }

            UserAction::Connect {
                from,
                from_port,
                to,
                to_port,
            } => {
                let src_id = resolve_name(from, graph, &created)?;
                let tgt_id = resolve_name(to, graph, &created)?;

                let src_port = from_port.clone().unwrap_or_else(|| "out".into());
                let tgt_port = to_port.clone().unwrap_or_else(|| "in".into());

                let edge = Edge::new(
                    PortRef {
                        node_id: src_id,
                        port_name: src_port,
                    },
                    PortRef {
                        node_id: tgt_id,
                        port_name: tgt_port,
                    },
                );
                ops.push(Operation::AddEdge(edge));
            }

            UserAction::Disconnect {
                from,
                from_port,
                to,
                to_port,
            } => {
                let src_id = resolve_name(from, graph, &created)?;
                let tgt_id = resolve_name(to, graph, &created)?;
                let src_port = from_port.clone().unwrap_or_else(|| "out".into());
                let tgt_port = to_port.clone().unwrap_or_else(|| "in".into());

                // Find matching edge
                if let Some(edge_id) = graph.edges.values().find_map(|e| {
                    if e.source.node_id == src_id
                        && e.source.port_name == src_port
                        && e.target.node_id == tgt_id
                        && e.target.port_name == tgt_port
                    {
                        Some(e.id)
                    } else {
                        None
                    }
                }) {
                    ops.push(Operation::RemoveEdge(edge_id));
                }
            }

            UserAction::RemoveNode { name } => {
                let id = resolve_name(name, graph, &created)?;
                ops.push(Operation::RemoveNode(id));
            }

            UserAction::FreezeNode { name, .. } => {
                let _id = resolve_name(name, graph, &created)?;
                // NOTE: Freezing requires offline rendering which isn't available here.
                // In a full implementation, this might return a placeholder op or
                // be handled by the caller (CLI/GUI) which has access to the renderer.
            }

            UserAction::SetConfig { node, key, value } => {
                let id = resolve_name(node, graph, &created)?;
                let config_val = json_to_config_value(value)
                    .map_err(|e| ActionError::InvalidConfig(key.clone(), e))?;

                let mut delta = std::collections::BTreeMap::new();
                delta.insert(
                    key.clone(),
                    ConfigChange {
                        old: None, // We don't track old value at action level
                        new: Some(config_val),
                    },
                );
                ops.push(Operation::ModifyConfig { node_id: id, delta });
            }

            UserAction::AddModulation {
                source_node,
                source_port,
                target_node,
                target_param,
                amount,
            } => {
                let src_id = resolve_name(source_node, graph, &created)?;
                let tgt_id = resolve_name(target_node, graph, &created)?;
                let mod_ir = crate::ir::Modulation::new(
                    PortRef {
                        node_id: src_id,
                        port_name: source_port.clone(),
                    },
                    tgt_id,
                    target_param.clone(),
                    *amount,
                );
                ops.push(Operation::AddModulation(mod_ir));
            }

            UserAction::ReplaceNode {
                name,
                new_kind_name,
            } => {
                let id = resolve_name(name, graph, &created)?;
                let mut delta = std::collections::BTreeMap::new();
                delta.insert(
                    "name".to_string(),
                    ConfigChange {
                        old: None,
                        new: Some(ConfigValue::String(new_kind_name.clone())),
                    },
                );
                ops.push(Operation::ModifyConfig { node_id: id, delta });
            }

            UserAction::AddSubGraph { name } => {
                let node = crate::ir::Node::new_subgraph(name);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }

            UserAction::RemoveModulation { id } => {
                ops.push(Operation::RemoveModulation(*id));
            }

            UserAction::DuplicateNode { .. } => {
                // Placeholder
            }

            UserAction::AddModulation { source_node, source_port, target_node, target_param, amount } => {
                let src_id = resolve_name(source_node, graph, &created)?;
                let tgt_id = resolve_name(target_node, graph, &created)?;
                let mod_ir = crate::ir::Modulation::new(
                    PortRef { node_id: src_id, port_name: source_port.clone() },
                    tgt_id,
                    target_param.clone(),
                    *amount
                );
                ops.push(Operation::AddModulation(mod_ir));
            }

            UserAction::ReplaceNode { name, new_kind_name } => {
                let id = resolve_name(name, graph, &created)?;
                let mut delta = std::collections::BTreeMap::new();
                delta.insert(
                    "name".to_string(),
                    ConfigChange {
                        old: None,
                        new: Some(ConfigValue::String(new_kind_name.clone())),
                    },
                );
                ops.push(Operation::ModifyConfig {
                    node_id: id,
                    delta,
                });
            }

            UserAction::AddSubGraph { name } => {
                let node = crate::ir::Node::new_subgraph(name);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }

            UserAction::RemoveModulation { id } => {
                ops.push(Operation::RemoveModulation(*id));
            }

            UserAction::DuplicateNode { .. } => {
                // Placeholder
            }
            UserAction::RunMonteCarlo { node_name, count } => {
                let id = resolve_name(node_name, graph, &created)?;
                if let Some(node) = graph.nodes.get(&id) {
                    for _ in 0..*count {
                        for (key, value) in &node.config {
                            if let ConfigValue::Float(f) = value {
                                // Random nudge +/- 10%
                                let noise = (rand::random::<f32>() - 0.5) * 0.2;
                                let new_val = (*f + noise as f64).clamp(0.0, 1.0);
                                
                                let mut delta = std::collections::BTreeMap::new();
                                delta.insert(key.clone(), ConfigChange { old: None, new: Some(ConfigValue::Float(new_val)) });
                                ops.push(Operation::ModifyConfig { node_id: id, delta });
                            }
                        }
                    }
                }
            }
            UserAction::RunSensitivity { .. } | UserAction::RunStabilityMap { .. } | UserAction::SquashHistory | UserAction::CreateSnapshot { .. } | UserAction::CheckoutRevision { .. } => {
                // Background or meta-operations handled by higher-level services.
            }
        }
    }

    Ok(ops)
}

/// Find a node's name from config.
pub fn node_name(node: &Node) -> String {
    node.config
        .get("name")
        .and_then(|v| match v {
            ConfigValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_else(|| node.id.to_string())
}

// ── Internal helpers ─────────────────────────

fn make_node(kind: NodeKind, name: &str, channels: u32) -> Node {
    let ports = match kind {
        NodeKind::Source => vec![TypedPort {
            name: "out".into(),
            direction: PortDirection::Output,
            domain: ExecutionDomain::Sample,
            data_type: DataType::Audio { channels },
            semantic: PortSemantic::Signal,
            polarity: PortPolarity::Bipolar,
        }],
        NodeKind::Sink => vec![TypedPort {
            name: "in".into(),
            direction: PortDirection::Input,
            domain: ExecutionDomain::Sample,
            data_type: DataType::Audio { channels },
            semantic: PortSemantic::Signal,
            polarity: PortPolarity::Bipolar,
        }],
        _ => vec![
            TypedPort {
                name: "in".into(),
                direction: PortDirection::Input,
                domain: ExecutionDomain::Sample,
                data_type: DataType::Audio { channels },
                semantic: PortSemantic::Signal,
                polarity: PortPolarity::Bipolar,
            },
            TypedPort {
                name: "out".into(),
                direction: PortDirection::Output,
                domain: ExecutionDomain::Sample,
                data_type: DataType::Audio { channels },
                semantic: PortSemantic::Signal,
                polarity: PortPolarity::Bipolar,
            },
        ],
    };

    let mut config = std::collections::BTreeMap::new();
    config.insert("name".into(), ConfigValue::String(name.into()));

    Node {
        id: StableId::new(),
        kind,
        ports,
        config,
        metadata: MetadataRef(None),
        confidence: ConfidenceScore::Verified,
    }
}

/// Resolve a human-readable name to a StableId.
/// Checks newly created nodes first, then existing graph.
pub fn resolve_name(
    name: &str,
    graph: &Graph,
    created: &std::collections::HashMap<String, StableId>,
) -> Result<StableId, ActionError> {
    // 1. Check batch-created nodes first
    if let Some(&id) = created.get(name) {
        return Ok(id);
    }

    // 2. Check if it's a direct StableId (highest priority to avoid ambiguity)
    if let Ok(id) = name.parse::<StableId>() {
        if graph.nodes.contains_key(&id) {
            return Ok(id);
        }
    }

    // 3. Search existing graph by config "name" field
    let matches: Vec<StableId> = graph
        .nodes
        .iter()
        .filter(|(_, n)| node_name(n) == name)
        .map(|(&id, _)| id)
        .collect();

    match matches.len() {
        0 => Err(ActionError::NodeNotFound(name.into())),
        1 => Ok(matches[0]),
        _ => Err(ActionError::AmbiguousName(name.into())),
    }
}

/// Convert serde_json::Value to ConfigValue.
fn json_to_config_value(v: &serde_json::Value) -> Result<ConfigValue, String> {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(ConfigValue::Float(f))
            } else if let Some(i) = n.as_i64() {
                Ok(ConfigValue::Int(i))
            } else {
                Err("unsupported number type".into())
            }
        }
        serde_json::Value::Bool(b) => Ok(ConfigValue::Bool(*b)),
        serde_json::Value::String(s) => Ok(ConfigValue::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.iter().map(json_to_config_value).collect();
            Ok(ConfigValue::List(items?))
        }
        serde_json::Value::Object(map) => {
            let mut bmap = std::collections::BTreeMap::new();
            for (k, v) in map {
                bmap.insert(k.clone(), json_to_config_value(v)?);
            }
            Ok(ConfigValue::Map(bmap))
        }
        serde_json::Value::Null => Err("null is not a valid config value".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_add_source() {
        let actions = vec![UserAction::AddSource {
            name: "Sine".into(),
            channels: 2,
        }];
        let graph = Graph::new();
        let ops = compile_actions(&actions, &graph).unwrap();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            Operation::AddNode(n) => {
                assert_eq!(n.kind, NodeKind::Source);
                assert_eq!(node_name(n), "Sine");
            }
            _ => panic!("expected AddNode"),
        }
    }

    #[test]
    fn test_compile_connect() {
        let actions = vec![
            UserAction::AddSource {
                name: "Sine".into(),
                channels: 2,
            },
            UserAction::AddSink {
                name: "Output".into(),
                channels: 2,
            },
            UserAction::Connect {
                from: "Sine".into(),
                from_port: None,
                to: "Output".into(),
                to_port: None,
            },
        ];
        let graph = Graph::new();
        let ops = compile_actions(&actions, &graph).unwrap();
        assert_eq!(ops.len(), 3);
        assert!(matches!(&ops[2], Operation::AddEdge(_)));
    }

    #[test]
    fn test_compile_set_config() {
        let actions = vec![
            UserAction::AddProcessor {
                name: "Gain".into(),
                channels: 2,
            },
            UserAction::SetConfig {
                node: "Gain".into(),
                key: "gain_db".into(),
                value: serde_json::json!(2.5),
            },
        ];
        let graph = Graph::new();
        let ops = compile_actions(&actions, &graph).unwrap();
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[1], Operation::ModifyConfig { .. }));
    }

    #[test]
    fn test_user_patch_file_deserialize() {
        let json = r#"{
            "description": "Basic signal chain",
            "intent": "Clean monitoring path",
            "constraints": [
                { "type": "must", "description": "preserve transients" },
                { "type": "avoid", "description": "harsh sibilance" }
            ],
            "actions": [
                { "action": "add_source", "name": "Sine" },
                { "action": "add_processor", "name": "Gain" },
                { "action": "add_sink", "name": "Output" },
                { "action": "connect", "from": "Sine", "to": "Gain" },
                { "action": "connect", "from": "Gain", "to": "Output" },
                { "action": "set_config", "node": "Gain", "key": "gain_db", "value": 2.0 }
            ]
        }"#;

        let patch_file: UserPatchFile = serde_json::from_str(json).unwrap();
        assert_eq!(patch_file.actions.len(), 6);
        assert_eq!(patch_file.intent, Some("Clean monitoring path".into()));
        assert_eq!(patch_file.constraints.len(), 2);
    }
}
