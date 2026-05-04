//! User-facing Action Schema.
//!
//! Authoring Language ではなく Review Language。
//! 吸うな。その薬は強い。export-only。

use crate::ir::{Edge, Graph, Node};
use crate::patch::Operation;
use crate::types::*;
use serde::{Deserialize, Serialize};

/// User-facing action — what humans write.
/// Internal operations are derived from these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UserAction {
    AddSource {
        name: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },
    AddProcessor {
        name: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },
    AddAnalyzer {
        name: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },
    AddSink {
        name: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },
    AddForeign {
        name: String,
        plugin: String,
        #[serde(default = "default_channels")]
        channels: u32,
    },
    Connect {
        from: String,
        from_port: Option<String>,
        to: String,
        to_port: Option<String>,
    },
    Disconnect {
        from: String,
        from_port: Option<String>,
        to: String,
        to_port: Option<String>,
    },
    RemoveNode {
        name: String,
    },
    FreezeNode {
        name: String,
        length_secs: f32,
    },
    SetConfig {
        node: String,
        key: String,
        value: serde_json::Value,
    },
    AddModulation {
        source_node: String,
        source_port: String,
        target_node: String,
        target_param: String,
        amount: f32,
    },
    RemoveModulation {
        id: StableId,
    },
    AddSubGraph {
        name: String,
    },
    ReplaceNode {
        name: String,
        new_kind_name: String,
    },
    DuplicateNode {
        node_id: StableId,
    },
    CheckoutRevision {
        revision: PatchId,
    },
    SquashHistory,
    CreateSnapshot {
        name: String,
    },
    RunMonteCarlo {
        node_name: String,
        count: usize,
    },
    RunSensitivity {
        node_name: String,
    },
    RunStabilityMap {
        node_name: String,
        param: String,
        range_start: f32,
        range_end: f32,
    },
}

fn default_channels() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPatchFile {
    pub description: Option<String>,
    pub intent: Option<String>,
    pub constraints: Vec<UserConstraint>,
    pub actions: Vec<UserAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConstraint {
    #[serde(rename = "type")]
    pub kind: String,
    pub description: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("node '{0}' not found")]
    NodeNotFound(String),
    #[error("ambiguous node name '{0}'")]
    AmbiguousName(String),
    #[error("invalid config for '{0}': {1}")]
    InvalidConfig(String, String),
}

pub fn compile_actions(
    actions: &[UserAction],
    graph: &Graph,
) -> Result<Vec<Operation>, ActionError> {
    let mut ops = Vec::new();
    let mut created: std::collections::HashMap<String, StableId> = std::collections::HashMap::new();

    for action in actions {
        match action {
            UserAction::AddSource { name, channels } => {
                let node = make_node("source", name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }
            UserAction::AddProcessor { name, channels } => {
                let node = make_node("processor", name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }
            UserAction::AddAnalyzer { name, channels } => {
                let node = make_node("analyzer", name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }
            UserAction::AddSink { name, channels } => {
                let node = make_node("sink", name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }
<<<<<<< HEAD
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
=======
            UserAction::AddForeign { name, plugin, channels } => {
                let node = make_node(&format!("foreign:{}", plugin), name, *channels);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
            }
            UserAction::Connect { from, from_port, to, to_port } => {
                let src_id = resolve_name(from, graph, &created).map_err(|e| { eprintln!("Connect Error: src {} not found", from); e })?;
                let tgt_id = resolve_name(to, graph, &created).map_err(|e| { eprintln!("Connect Error: tgt {} not found", to); e })?;
>>>>>>> fe9c97d (feat: enhance modular synthesis architecture, add circuit simulation modules, and update GUI/SDK)
                let edge = Edge::new(
                    PortRef {
                        node_id: src_id,
                        port_name: from_port.clone().unwrap_or_else(|| "out".into()),
                    },
                    PortRef {
                        node_id: tgt_id,
                        port_name: to_port.clone().unwrap_or_else(|| "in".into()),
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
            UserAction::SetConfig { node, key, value } => {
<<<<<<< HEAD
                let id = resolve_name(node, graph, &created)?;
                let config_val = json_to_config_value(value)
                    .map_err(|e| ActionError::InvalidConfig(key.clone(), e))?;
=======
                let id = resolve_name(node, graph, &created).map_err(|e| { eprintln!("SetConfig Error: node {} not found", node); e })?;
                let config_val = json_to_config_value(value).map_err(|e| ActionError::InvalidConfig(key.clone(), e))?;
>>>>>>> fe9c97d (feat: enhance modular synthesis architecture, add circuit simulation modules, and update GUI/SDK)
                let mut delta = std::collections::BTreeMap::new();
                delta.insert(
                    key.clone(),
                    ConfigChange {
                        old: None,
                        new: Some(config_val),
                    },
                );
                ops.push(Operation::ModifyConfig { node_id: id, delta });
            }
<<<<<<< HEAD
            UserAction::AddModulation {
                source_node,
                source_port,
                target_node,
                target_param,
                amount,
            } => {
                let src_id = resolve_name(source_node, graph, &created)?;
                let tgt_id = resolve_name(target_node, graph, &created)?;
=======
            UserAction::AddModulation { source_node, source_port, target_node, target_param, amount } => {
                let src_id = resolve_name(source_node, graph, &created).map_err(|e| { eprintln!("AddMod Error: src {} not found", source_node); e })?;
                let tgt_id = resolve_name(target_node, graph, &created).map_err(|e| { eprintln!("AddMod Error: tgt {} not found", target_node); e })?;
>>>>>>> fe9c97d (feat: enhance modular synthesis architecture, add circuit simulation modules, and update GUI/SDK)
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
            UserAction::RemoveModulation { id } => {
                ops.push(Operation::RemoveModulation(*id));
            }
            UserAction::AddSubGraph { name } => {
                let node = crate::ir::Node::new_subgraph(name);
                created.insert(name.clone(), node.id);
                ops.push(Operation::AddNode(node));
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
            UserAction::DuplicateNode { .. }
            | UserAction::FreezeNode { .. }
            | UserAction::CheckoutRevision { .. }
            | UserAction::SquashHistory
            | UserAction::CreateSnapshot { .. } => {}
            UserAction::RunMonteCarlo { node_name, count } => {
                let id = resolve_name(node_name, graph, &created)?;
                if let Some(node) = graph.nodes.get(&id) {
                    for _ in 0..*count {
                        for (key, value) in &node.config {
                            if let ConfigValue::Float(f) = value {
                                let noise = (rand::random::<f64>() - 0.5) * 0.1;
                                let mut delta = std::collections::BTreeMap::new();
                                delta.insert(
                                    key.clone(),
                                    ConfigChange {
                                        old: None,
                                        new: Some(ConfigValue::Float(*f + noise)),
                                    },
                                );
                                ops.push(Operation::ModifyConfig { node_id: id, delta });
                            }
                        }
                    }
                }
            }
            UserAction::RunSensitivity { .. } | UserAction::RunStabilityMap { .. } => {}
        }
    }
    Ok(ops)
}

pub fn node_name(node: &Node) -> String {
    node.config
        .get("name")
        .and_then(|v| v.as_string())
        .cloned()
        .unwrap_or_else(|| node.id.to_string())
}

fn make_node(kind_str: &str, name: &str, _channels: u32) -> Node {
    let mut n = Node::new_processor(name);
    // Store the requested type in config for the runtime to pick up
    n.config.insert("type".to_string(), ConfigValue::String(kind_str.to_string()));
    n
}

pub fn resolve_name(
    name: &str,
    graph: &Graph,
    created: &std::collections::HashMap<String, StableId>,
) -> Result<StableId, ActionError> {
    if let Some(&id) = created.get(name) {
        return Ok(id);
    }
    if let Ok(id) = name.parse::<StableId>() {
        if graph.nodes.contains_key(&id) {
            return Ok(id);
        }
    }
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

fn json_to_config_value(v: &serde_json::Value) -> Result<ConfigValue, String> {
    match v {
        serde_json::Value::Number(n) => Ok(ConfigValue::Float(n.as_f64().unwrap_or(0.0))),
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
        serde_json::Value::Null => Err("null is not allowed".into()),
    }
}
