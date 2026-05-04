//! Visual Data Constitution — The Shadow of Audio
//!
//! オーディオスレッドからGUIスレッドへ投影される「影（スナップショット）」。
//! ロックフリーな Triple-Buffer を通じて転送される。

use std::collections::BTreeMap;

/// モジュールごとの視覚的状態
#[derive(Clone, Default)]
pub struct ModuleVisualState {
    /// 現在のパラメータ値（アニメーション、LED輝度、ノブ角度用）
    pub params: BTreeMap<String, f32>,
    /// 出力ポートの現在の電圧（ケーブルの発光強度用）
    pub outputs: Vec<f32>,
    /// 入力ポートの現在の電圧
    pub inputs: Vec<f32>,
    /// オシロスコープ用バッファ (直近 N サンプル)
    pub scope_data: Vec<f32>,
    /// 鑑識データ (Drift Inspector 用)
    pub forensic: Option<dirtyrack_sdk::ForensicData>,
    /// 変調後のパラメータ値 (Forensic View用)
    pub modulated_params: Option<Vec<f32>>,
}

/// ラック全体の視覚的スナップショット
#[derive(Clone, Default)]
pub struct VisualSnapshot {
    /// キーは StableNodeId
    pub modules: BTreeMap<u64, ModuleVisualState>,
}

impl VisualSnapshot {
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }
}
