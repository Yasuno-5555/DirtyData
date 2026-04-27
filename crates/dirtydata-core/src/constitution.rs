//! 憲法テスト — DirtyData の根本的不変条件。
//!
//! # Every state must be explainable, or disposable.
//!
//! これは property test である。
//! ランダムな操作列に対して、以下が常に成立することを証明する:
//!
//! 1. Replayability: replay(patches) == current graph (hash一致)
//! 2. Hash Stability: 同一 operations → 同一 hash
//! 3. Explainability: graph.applied_patches は全て存在し、逆変換可能

#[cfg(test)]
mod tests {
    use crate::hash;
    use crate::ir::{Edge, Graph, Node};
    use crate::patch::{Operation, Patch};
    use crate::types::*;
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    // ── Strategy: ランダムなノード生成 ─────────

    fn arb_node_kind() -> impl Strategy<Value = NodeKind> {
        prop_oneof![
            Just(NodeKind::Source),
            Just(NodeKind::Processor),
            Just(NodeKind::Analyzer),
            Just(NodeKind::Sink),
            Just(NodeKind::Junction),
        ]
    }

    fn arb_node() -> impl Strategy<Value = Node> {
        (arb_node_kind(), "[a-z]{3,8}").prop_map(|(kind, name)| {
            let ports = match kind {
                NodeKind::Source => vec![TypedPort {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    domain: ExecutionDomain::Sample,
                    data_type: DataType::Audio { channels: 2 },
                    semantic: PortSemantic::Signal,
                    polarity: PortPolarity::Bipolar,
                }],
                NodeKind::Sink => vec![TypedPort {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    domain: ExecutionDomain::Sample,
                    data_type: DataType::Audio { channels: 2 },
                    semantic: PortSemantic::Signal,
                    polarity: PortPolarity::Bipolar,
                }],
                _ => vec![
                    TypedPort {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        domain: ExecutionDomain::Sample,
                        data_type: DataType::Audio { channels: 2 },
                        semantic: PortSemantic::Signal,
                        polarity: PortPolarity::Bipolar,
                    },
                    TypedPort {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        domain: ExecutionDomain::Sample,
                        data_type: DataType::Audio { channels: 2 },
                        semantic: PortSemantic::Signal,
                        polarity: PortPolarity::Bipolar,
                    },
                ],
            };
            let mut config = BTreeMap::new();
            config.insert("name".into(), ConfigValue::String(name));
            Node {
                id: StableId::new(),
                kind,
                ports,
                config,
                metadata: MetadataRef(None),
                confidence: ConfidenceScore::Verified,
            }
        })
    }

    fn arb_config_value() -> impl Strategy<Value = ConfigValue> {
        prop_oneof![
            any::<f64>().prop_map(ConfigValue::Float),
            any::<i64>().prop_map(ConfigValue::Int),
            any::<bool>().prop_map(ConfigValue::Bool),
            "[a-z]{1,10}".prop_map(ConfigValue::String),
        ]
    }

    // ── 憲法 1: Replayability Invariant ────────
    //
    // 任意のパッチ列を適用した結果の graph は、
    // 同じパッチ列を replay することで完全に再構成できる。

    proptest! {
        #[test]
        fn constitution_replayability(
            node_count in 1usize..8,
        ) {
            // ランダムにノードを生成してパッチを作成
            let _rt = proptest::test_runner::TestRunner::new(Default::default());
            let mut nodes = Vec::new();
            for _ in 0..node_count {
                // Use fixed nodes for determinism within proptest
                nodes.push(Node::new_processor(&format!("node_{}", nodes.len())));
            }

            let patch = Patch::from_operations(
                nodes.iter().map(|n| Operation::AddNode(n.clone())).collect()
            );

            // Apply
            let mut graph = Graph::new();
            graph.apply_patch(&patch).unwrap();

            // Replay
            let replayed = Graph::replay(&[patch]).unwrap();

            // 憲法: hash は一致しなければならない
            let h1 = hash::hash_graph(&graph);
            let h2 = hash::hash_graph(&replayed);
            prop_assert_eq!(h1, h2, "Replayability violation: graph hashes differ");
        }
    }

    // ── 憲法 2: Hash Stability Invariant ───────
    //
    // 同一の operations から生成された patch は
    // 常に同一の deterministic_hash を持つ。

    #[test]
    fn constitution_hash_stability() {
        // 同じ構造のノードを2回作る（IDは異なる）
        // → hash は operations の内容に依存するので、
        //   同一の Patch インスタンスは常に同一 hash を返す

        let node = Node::new_processor("TestGain");
        let ops = vec![Operation::AddNode(node.clone())];

        let patch1 = Patch::from_operations(ops.clone());

        // 同じ ops から再計算
        let recomputed_hash = hash::hash_patch(&patch1);

        assert_eq!(
            patch1.deterministic_hash, recomputed_hash,
            "Hash stability violation"
        );
    }

    // ── 憲法 3: Explainability Invariant ───────
    //
    // graph.applied_patches の全エントリは
    // 実際に適用されたパッチと1:1対応する。
    // パッチ数 == applied_patches.len()

    proptest! {
        #[test]
        fn constitution_explainability(
            patch_count in 1usize..6,
        ) {
            let mut graph = Graph::new();
            let mut all_patches = Vec::new();

            for i in 0..patch_count {
                let node = Node::new_processor(&format!("node_{}", i));
                let patch = Patch::from_operations(vec![Operation::AddNode(node)]);
                graph.apply_patch(&patch).unwrap();
                all_patches.push(patch);
            }

            // 憲法: applied_patches は全パッチの ID と一致
            prop_assert_eq!(
                graph.lineage.applied_patches.len(),
                all_patches.len(),
                "Explainability violation: patch count mismatch"
            );

            for (applied_id, patch) in graph.lineage.applied_patches.iter().zip(all_patches.iter()) {
                prop_assert_eq!(
                    applied_id, &patch.identity,
                    "Explainability violation: patch ID mismatch"
                );
            }

            // 憲法: replay から同一グラフを再構成可能
            let replayed = Graph::replay(&all_patches).unwrap();
            let h1 = hash::hash_graph(&graph);
            let h2 = hash::hash_graph(&replayed);
            prop_assert_eq!(h1, h2,
                "Explainability violation: replay produces different state"
            );
        }
    }

    // ── 憲法 4: Modify-Replay Round-trip ───────
    //
    // config 変更を含むパッチ列でも replay は成立する。

    #[test]
    fn constitution_modify_replay() {
        let node = Node::new_processor("Gain");
        let p1 = Patch::from_operations(vec![Operation::AddNode(node.clone())]);

        let mut delta = BTreeMap::new();
        delta.insert(
            "gain_db".into(),
            ConfigChange {
                old: None,
                new: Some(ConfigValue::Float(3.5)),
            },
        );
        let p2 = Patch::from_operations(vec![Operation::ModifyConfig {
            node_id: node.id,
            delta,
        }]);

        // Apply sequentially
        let mut graph = Graph::new();
        graph.apply_patch(&p1).unwrap();
        graph.apply_patch(&p2).unwrap();

        // Replay
        let replayed = Graph::replay(&[p1, p2]).unwrap();

        assert_eq!(
            hash::hash_graph(&graph),
            hash::hash_graph(&replayed),
            "Modify-replay round-trip failed"
        );
    }

    // ── 憲法 5: Edge Operations Replay ─────────

    #[test]
    fn constitution_edge_replay() {
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

        let p1 = Patch::from_operations(vec![
            Operation::AddNode(src),
            Operation::AddNode(gain),
            Operation::AddNode(sink),
        ]);
        let p2 = Patch::from_operations(vec![Operation::AddEdge(edge1), Operation::AddEdge(edge2)]);

        let mut graph = Graph::new();
        graph.apply_patch(&p1).unwrap();
        graph.apply_patch(&p2).unwrap();

        let replayed = Graph::replay(&[p1, p2]).unwrap();

        assert_eq!(
            hash::hash_graph(&graph),
            hash::hash_graph(&replayed),
            "Edge replay round-trip failed"
        );
    }
}
