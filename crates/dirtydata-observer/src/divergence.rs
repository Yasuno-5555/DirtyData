use dirtydata_core::types::{Hash, StableId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 世界がズレた瞬間を記録する
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergencePoint {
    pub sample_index: u64,
    pub node_id: StableId,
    pub node_name: String,
    pub port_idx: usize,
    pub expected_value: [f32; 2],
    pub actual_value: [f32; 2],
    pub diff_magnitude: f32,
}

/// Divergence Map — オーディオのリアリティにおける分岐の地図
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DivergenceMap {
    pub points: Vec<DivergencePoint>,
    pub golden_hash: Option<Hash>,
    pub actual_hash: Option<Hash>,
    pub first_divergence_index: Option<u64>,
    pub timestamp: Timestamp,
    pub metadata: HashMap<String, String>,
}

impl DivergenceMap {
    pub fn new() -> Self {
        Self {
            timestamp: Timestamp::now(),
            ..Default::default()
        }
    }

    pub fn add_point(&mut self, point: DivergencePoint) {
        if self.first_divergence_index.is_none() {
            self.first_divergence_index = Some(point.sample_index);
        }
        self.points.push(point);
    }

    pub fn is_diverged(&self) -> bool {
        !self.points.is_empty()
    }
}

/// 因果比較 — A/B 比較を超えた、変更の影響分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAnalysis {
    pub parameter_delta: Vec<(StableId, String, f32, f32)>, // node, param, val_a, val_b
    pub divergence_magnitude_db: f32,
    pub peak_divergence_sample: u64,
    pub transient_impact: f32, // Simplified for now
}

impl CausalAnalysis {
    pub fn from_divergence(map: &DivergenceMap) -> Self {
        let max_mag = map
            .points
            .iter()
            .map(|p| p.diff_magnitude)
            .fold(0.0, f32::max);
        let peak_sample = map
            .points
            .iter()
            .find(|p| p.diff_magnitude >= max_mag)
            .map(|p| p.sample_index)
            .unwrap_or(0);

        Self {
            parameter_delta: Vec::new(), // Populate this from Graph diff
            divergence_magnitude_db: 20.0 * max_mag.log10().max(-100.0),
            peak_divergence_sample: peak_sample,
            transient_impact: max_mag * 1.5, // Dummy heuristic
        }
    }
}
