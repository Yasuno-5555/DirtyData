use dirtydata_core::patch::Patch;
use dirtydata_core::types::{IntentId, PatchId, StableId, Timestamp};
use serde::{Deserialize, Serialize};

/// 音の「犯人」を追跡する
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundTrace {
    pub sample_index: u64,
    pub node_id: StableId,
    pub parameter_name: String,
    pub value: f32,
    pub attribution: Attribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    pub intent_id: Option<IntentId>,
    pub intent_description: Option<String>,
    pub patch_id: PatchId,
    pub source: String,
    pub timestamp: Timestamp,
    pub commit_hash: Option<String>, // if available
}

pub struct Attributor;

impl Attributor {
    /// 特定のノードの特定のパラメータに最後に影響を与えたパッチを探す
    pub fn trace_parameter(
        node_id: StableId,
        param_name: &str,
        patches: &[Patch],
    ) -> Option<Attribution> {
        // 後ろから（最新から）探す
        for patch in patches.iter().rev() {
            for op in &patch.operations {
                match op {
                    dirtydata_core::patch::Operation::ModifyConfig { node_id: id, delta }
                        if *id == node_id =>
                    {
                        if delta.contains_key(param_name) {
                            return Some(Attribution {
                                intent_id: patch.intent_ref,
                                intent_description: None, // Need IntentState to look this up
                                patch_id: patch.identity,
                                source: format!("{:?}", patch.source),
                                timestamp: patch.timestamp,
                                commit_hash: None, // Could be stored in Patch metadata
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }
}
