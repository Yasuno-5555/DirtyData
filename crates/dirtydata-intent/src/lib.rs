//! Intent Engine — Structured Meaning & Constraints
//!
//! "音楽は最適化問題じゃない。制約付き妥協問題です。"
//!
//! DirtyData において、パッチは単なる状態変更の羅列ではない。
//! Intent（意図）という上位概念があり、パッチは「それを実現するための Strategy の結果」である。

pub mod attribution;

pub use attribution::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use dirtydata_core::types::*;

/// Intent の実現方法。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum IntentStrategy {
    /// 手動。ユーザーがパッチを適用して紐付ける。
    #[default]
    Manual,
    /// 自動。特定のノードを挿入する。
    InsertNode {
        kind: NodeKind,
        name: String,
        config: ConfigSnapshot,
    },
    /// 自動。既存のノードを接続する。
    Bridge { from_node: String, to_node: String },
    /// 自動。安全な Frozen Asset に置換する。
    Freeze { target_node: String },
}

impl Default for IntentStrategy {
    fn default() -> Self {
        Self::Manual
    }
}

/// Intent 本体。何を実現したいか。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentNode {
    pub id: IntentId,
    pub description: String,
    pub constraints: Vec<IntentConstraint>,
    pub status: IntentStatus,
    pub strategy: IntentStrategy,
    pub attached_patches: Vec<PatchId>,
}

/// IntentEngine の永続状態。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentState {
    pub intents: HashMap<IntentId, IntentNode>,
}

impl IntentState {
    pub fn save(&self, project_root: &Path) -> Result<(), std::io::Error> {
        let path = project_root.join(".dirtydata").join("intents.json");
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)
    }

    pub fn load(project_root: &Path) -> Result<Self, std::io::Error> {
        let path = project_root.join(".dirtydata").join("intents.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        let state = serde_json::from_str(&data)?;
        Ok(state)
    }

    pub fn add(&mut self, description: String, constraints: Vec<IntentConstraint>) -> IntentId {
        let id = IntentId::new();
        self.intents.insert(
            id,
            IntentNode {
                id,
                description,
                constraints,
                status: IntentStatus::Proposal,
                strategy: IntentStrategy::Manual,
                attached_patches: Vec::new(),
            },
        );
        id
    }

    pub fn attach(&mut self, id: IntentId, patch_id: PatchId) -> Result<(), String> {
        let intent = self
            .intents
            .get_mut(&id)
            .ok_or_else(|| format!("Intent {} not found", id))?;
        if !intent.attached_patches.contains(&patch_id) {
            intent.attached_patches.push(patch_id);
        }
        if intent.status == IntentStatus::Proposal {
            intent.status = IntentStatus::Attached;
        }
        Ok(())
    }

    /// 制約を評価し、違反内容を返す。
    /// これが Semantic Timeline で「なぜ壊れたか」を表示する基盤となる。
    pub fn evaluate_constraints(
        &self,
        id: IntentId,
        graph: &dirtydata_core::ir::Graph,
    ) -> Vec<String> {
        let intent = match self.intents.get(&id) {
            Some(i) => i,
            None => return vec![format!("Intent {} not found", id)],
        };
        let mut violations = Vec::new();

        for constraint in &intent.constraints {
            match constraint {
                IntentConstraint::Must(bound) => {
                    if let Some(val) = self.get_param_value(graph, &bound.target) {
                        if val < bound.range_start || val > bound.range_end {
                            violations.push(format!("Constraint Violation [Must]: {} is {}, but must be in range {}..={}", bound.target, val, bound.range_start, bound.range_end));
                        }
                    }
                }
                IntentConstraint::Never(bound) => {
                    if let Some(val) = self.get_param_value(graph, &bound.target) {
                        if val >= bound.range_start && val <= bound.range_end {
                            violations.push(format!("Constraint Violation [Never]: {} is {}, which is forbidden in range {}..={}", bound.target, val, bound.range_start, bound.range_end));
                        }
                    }
                }
                _ => {} // Prefer/Avoid are soft
            }
        }
        violations
    }

    fn get_param_value(&self, graph: &dirtydata_core::ir::Graph, path: &str) -> Option<f32> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() != 2 {
            return None;
        }

        let node_name = parts[0];
        let param_key = parts[1];

        for node in graph.nodes.values() {
            if dirtydata_core::actions::node_name(node) == node_name {
                if let Some(ConfigValue::Float(f)) = node.config.get(param_key) {
                    return Some(*f as f32);
                }
            }
        }
        None
    }
}
