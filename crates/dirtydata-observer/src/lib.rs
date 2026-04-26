//! Observer Engine — 自動化された疑いのシステム
//!
//! "Observe before Control"
//!
//! DirtyData において、GUI やプラグインは真実ではない。
//! Observer が物理ファイルシステムや外部状態を観測し、
//! その結果を `Observation` として記録する。
//!
//! これは `dirty doctor` や `status` の基礎データとなる。

pub mod divergence;

pub use divergence::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use dirtydata_core::actions::node_name;
use dirtydata_core::hash;
use dirtydata_core::ir::Graph;
use dirtydata_core::types::{ConfidenceScore, ConfigValue, Hash, NodeKind, StableId, Timestamp};

/// 観測結果。特定のエンティティ（ノードやファイル）に対する信用度。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub target: StableId,
    pub target_name: String,
    pub confidence: ConfidenceScore,
    pub evidence: Evidence,
    pub timestamp: Timestamp,
}

/// なぜその Confidence なのか？の証拠。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Evidence {
    /// 外部ファイルが期待するハッシュと完全に一致した。
    FileHashMatch { path: PathBuf, hash: Hash },
    /// ファイルサイズや mtime は取得できたが、ハッシュ計算は重いためスキップした。
    FileStatOnly {
        path: PathBuf,
        size: u64,
        mtime: u64,
    },
    /// 期待するハッシュと実際のハッシュが食い違っている。危険。
    FileHashMismatch {
        path: PathBuf,
        expected: Hash,
        actual: Hash,
    },
    /// 拡張子が未知、またはサポート対象外。
    ExtensionUnknown { path: PathBuf, ext: String },
    /// 外部プラグインなど、構造上非決定的なもの。
    InherentNondeterminism { plugin_name: String },
    /// 状況から推論した。
    InferredFromContext(String),
    /// 観測不能。
    Unobservable(String),
}

/// Observer の永続状態。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObserverState {
    /// 最後に実行された観測結果。対象の StableId をキーにする。
    pub observations: HashMap<StableId, Observation>,
}

pub struct Observer;

impl Observer {
    /// グラフ全体を観測する
    pub fn observe_graph(graph: &Graph, project_root: &Path) -> ObserverState {
        let mut state = ObserverState::default();

        for (id, node) in &graph.nodes {
            let name = node_name(node);

            match &node.kind {
                // Source: ファイル参照があるならハッシュを検証する
                NodeKind::Source => {
                    if let Some(ConfigValue::String(file_path)) = node.config.get("file") {
                        let path = project_root.join(file_path);

                        // 期待されるハッシュが config にあれば検証、なければ Inferred
                        let expected_hash_str = node.config.get("expected_hash").and_then(|v| {
                            if let ConfigValue::String(s) = v {
                                Some(s)
                            } else {
                                None
                            }
                        });

                        let obs = Self::observe_file(
                            *id,
                            name.clone(),
                            &path,
                            expected_hash_str.map(|s| s.as_str()),
                        );
                        state.observations.insert(*id, obs);
                    } else {
                        // ファイル参照がない Source は外部入力（マイク等）とみなす
                        state.observations.insert(
                            *id,
                            Observation {
                                target: *id,
                                target_name: name,
                                confidence: ConfidenceScore::Unknown,
                                evidence: Evidence::Unobservable("Live audio input".into()),
                                timestamp: Timestamp(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as i64,
                                ),
                            },
                        );
                    }
                }

                // Foreign: プラグインなどはデフォルトでSuspicious
                NodeKind::Foreign(plugin_name) => {
                    state.observations.insert(
                        *id,
                        Observation {
                            target: *id,
                            target_name: name,
                            confidence: ConfidenceScore::Suspicious,
                            evidence: Evidence::InherentNondeterminism {
                                plugin_name: plugin_name.clone(),
                            },
                            timestamp: Timestamp(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as i64,
                            ),
                        },
                    );
                }

                // デフォルトのプロセッサ等は Inferred
                _ => {
                    state.observations.insert(
                        *id,
                        Observation {
                            target: *id,
                            target_name: name,
                            confidence: ConfidenceScore::Inferred,
                            evidence: Evidence::InferredFromContext(
                                "Internal deterministic node".into(),
                            ),
                            timestamp: Timestamp(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as i64,
                            ),
                        },
                    );
                }
            }
        }

        state
    }

    /// 単一のファイルを観測する
    pub fn observe_file(
        target: StableId,
        target_name: String,
        path: &Path,
        expected_hash_hex: Option<&str>,
    ) -> Observation {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        if !path.exists() {
            return Observation {
                target,
                target_name,
                confidence: ConfidenceScore::Unknown,
                evidence: Evidence::Unobservable(format!("File not found: {}", path.display())),
                timestamp: Timestamp(ts),
            };
        }

        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                return Observation {
                    target,
                    target_name,
                    confidence: ConfidenceScore::Unknown,
                    evidence: Evidence::Unobservable(format!("Cannot read metadata: {}", e)),
                    timestamp: Timestamp(ts),
                }
            }
        };

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "wav" && ext != "flac" && ext != "json" {
            return Observation {
                target,
                target_name,
                confidence: ConfidenceScore::Suspicious,
                evidence: Evidence::ExtensionUnknown {
                    path: path.to_path_buf(),
                    ext: ext.to_string(),
                },
                timestamp: Timestamp(ts),
            };
        }

        // expected_hash がある場合は厳密な検証を行う
        if let Some(expected_hex) = expected_hash_hex {
            let file_bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(_) => {
                    return Observation {
                        target,
                        target_name,
                        confidence: ConfidenceScore::Unknown,
                        evidence: Evidence::Unobservable("Failed to read file contents".into()),
                        timestamp: Timestamp(ts),
                    }
                }
            };

            let actual_hash = hash::hash_bytes(&file_bytes);
            let mut expected_hash = [0u8; 32];

            // Hex decode
            let valid_hex =
                expected_hex.len() == 64 && expected_hex.chars().all(|c| c.is_ascii_hexdigit());
            if valid_hex {
                for i in 0..32 {
                    if let Ok(b) = u8::from_str_radix(&expected_hex[i * 2..i * 2 + 2], 16) {
                        expected_hash[i] = b;
                    }
                }

                if expected_hash == actual_hash {
                    return Observation {
                        target,
                        target_name,
                        confidence: ConfidenceScore::Verified,
                        evidence: Evidence::FileHashMatch {
                            path: path.to_path_buf(),
                            hash: actual_hash,
                        },
                        timestamp: Timestamp(ts),
                    };
                } else {
                    return Observation {
                        target,
                        target_name,
                        confidence: ConfidenceScore::Unknown,
                        evidence: Evidence::FileHashMismatch {
                            path: path.to_path_buf(),
                            expected: expected_hash,
                            actual: actual_hash,
                        },
                        timestamp: Timestamp(ts),
                    };
                }
            }
        }

        // ハッシュ指定がなければ Stat のみ
        let mtime = meta
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Observation {
            target,
            target_name,
            confidence: ConfidenceScore::Inferred,
            evidence: Evidence::FileStatOnly {
                path: path.to_path_buf(),
                size: meta.len(),
                mtime,
            },
            timestamp: Timestamp(ts),
        }
    }
}

impl ObserverState {
    pub fn save(&self, project_root: &Path) -> Result<(), std::io::Error> {
        let path = project_root.join(".dirtydata").join("observations.json");
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)
    }

    pub fn load(project_root: &Path) -> Result<Self, std::io::Error> {
        let path = project_root.join(".dirtydata").join("observations.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        let state = serde_json::from_str(&data)?;
        Ok(state)
    }
}
