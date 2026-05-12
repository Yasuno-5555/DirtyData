# DirtyData ROADMAP

監査結果（2026-05-12）に基づく実装優先順位。

## Priority 0 — 即時対応

### P0.1 マージコンフリクトの解消

actions.rs に残っている `<<<<<<< HEAD` / `>>>>>>> fe9c97d` マーカーを解決する。

- **対象**: `crates/dirtydata-core/src/actions.rs`（L166-193, L235-242, L254-267）
- **内容**: `AddForeign`, `SetConfig`, `AddModulation` の3箇所で競合状態。適切な実装を選定しマーカーを除去する
- **確認**: `cargo build` が通り、関連テストがパスすること

### P0.2 PluginHost の最小サンドボックス

PluginHost が子プロセスを起動する際、ファイルシステムアクセスを制限する。

- **対象**: `crates/dirtydata-host/src/lib.rs`
- **内容**:
  1. ワークスペースルート以外のファイルシステムへの書き込みを防止
  2. プラグイン実行前にワークスペースの manifest hash を検証するオプションを追加
- **非目標**: 完全なコンテナ分離（Linux namespace / seccomp）は将来の拡張とする

## Priority 1 — 短期（1-2週間）

### P1.1 core クレートからの `.unwrap()` 撲滅

DSP ホットパス以外の `unwrap()` を適切なエラー伝搬に置き換える。

- **対象**:
  - `crates/dirtydata-core/src/hash.rs`: `serde_json::to_string().unwrap()` → `?` またはデフォルト値
  - `crates/dirtydata-core/src/dsl.rs`: `writeln!(...).unwrap()` → エラーを無視してよい旨を明示（`let _ = writeln!(...)`）
  - `crates/dirtydata-core/src/storage.rs`: `serde_json::to_string_pretty().unwrap()` → `?`
- **確認**: 既存テストがすべてパスすること

### P1.2 `.gitignore` のワイルドカード修正

`*.txt` / `*.log` がリポジトリルート全体にマッチする問題を修正する。

- **対象**: `.gitignore`
- **内容**: `/build_errors.txt` など個別ファイルを明示するか、`/` プレフィックスで限定する

### P1.3 ConfigValue f32 ラウンドトリップ保証

`ConfigValue::Float(f64)` → DSP f32 変換の正確性を保証するテストとアクセサを追加する。

- **対象**: `crates/dirtydata-core/src/types.rs`
- **内容**: `as_f32()` アクセサ追加、ラウンドトリップの等価性テスト
- **非目標**: 内部表現を f64 から f32 に変更すること（精度維持のため）

## Priority 2 — 中期（2-4週間）

### P2.1 DspRunner エッジ探索の事前インデックス化

サンプル単位のホットパスで毎回 `graph.edges.values()` を線形探索している問題を修正する。

- **対象**: `crates/dirtydata-runtime/src/lib.rs`
- **内容**: `DspRunner::new()` でノードごとの入力エッジインデックス `HashMap<StableId, Vec<PortRef>>` を事前構築し、`process_sample()` ではそれを参照する
- **ベンチマーク**: 100ノード程度でレイテンシ計測

### P2.2 Merge ロジックの全操作カバー

現在の merge_three_way は `ModifyConfig` / `RemoveNode` のみ対応。全操作タイプをカバーする。

- **対象**: `crates/dirtydata-core/src/merge.rs`
- **追加対応**:
  - `AddNode` / `AddEdge`: 両側で同一 ID の追加 → コンフリクト
  - `RemoveEdge`: node 削除と同様のパターン
  - `ReplaceNode`: node の完全置換として競合検出
- **テスト**: 両側が異なる操作をするマージシナリオのテスト追加

### P2.3 マージコンフリクト履歴の精査

`fe9c97d` コミットのコンフリクトの原因と影響範囲を調査する。

- **対象**: git history
- **内容**: fe9c97d がどのようにコンフリクトを発生させたか、他のファイルに影響が及んでいないかを確認する。必要なフォローアップ修正を行う

### P2.4 hex エンコード戦略の統一

内部実装と外部クレート依存が混在している hex エンコードを統一する。

- **対象**:
  - `crates/dirtydata-core/src/patch.rs`（内部 `mod hex`）
  - `crates/dirtydata-core/Cargo.toml`
- **内容**: `hex` crate を dirtydata-core の依存に追加し、内部実装を置き換える。または `hex` crate に依存せず自前実装で統一する

## Priority 3 — 長期（1-2ヶ月）

### P3.1 テストカバレッジ拡充

テストが不足している領域を段階的にカバーする。

| クレート | 優先テスト | 種別 |
|----------|-----------|------|
| dirtydata-runtime ノード実装 | 全50+ノードの unit test（config 変更、エッジケース） | unit |
| dirtydata-runtime 全体 | 既知グラフのオフラインレンダリングとハッシュ一致確認 | integration |
| dirtydata-dsp-circuit | MnaSolver の収束性、エレメント追加・削除 | unit + property |
| dirtydata-core merge | 3-way マージの全操作タイプカバレッジ | property |
| dirtydata-core mutation | MutationEngine の seed 固定再現性 | unit |
| dirtydata-core exploration | MonteCarlo / Sensitivity / StabilityMap の結果検証 | unit |
| dirtydata-intent | Intent 制約評価の全ケース | unit |
| dirtydata-observer | 各 Evidence バリアントの観測ロジック | unit |
| dirtydata-host | Workspace 開閉、save/load ラウンドトリップ | integration |
| JIT コンパイラ | 全 JIT lowering パスの出力一致テスト | unit |

### P3.2 SmoothedValue 数値安定性改善

極端なパラメータでのスムージング暴走を防ぐ。

- **対象**: `crates/dirtydata-runtime/src/nodes/base.rs`
- **内容**: `coeff` 計算に下限ガードを追加。`tau * sample_rate` が極端に小さい/大きい場合のクリッピング

### P3.3 NodeState デシリアライズの安全性強化

信頼できない永続化データからの復元を安全にする。

- **対象**:
  - `crates/dirtydata-runtime/src/nodes/base.rs`（`NodeState::to_json`）
  - `crates/dirtyrack-gui/src/lib.rs`（circuit editor の `serde_json::from_slice`）
- **内容**: デシリアライズ結果のバリデーション追加、フォールバック値の設定

### P3.4 NaN Storm プロトコルの強化

NaN 検出後の状態復旧を明確にする。

- **対象**: `crates/dirtydata-host/src/lib.rs` / `crates/dirtydata-runtime/src/lib.rs`
- **内容**:
  1. NaN 検出時に全ノードの状態をラストグッドスナップショットにロールバック
  2. ユーザー通知チャネルの確立
  3. 自動ミュート＋手動リセットまでの状態遷移

## 非目標

以下の項目は現時点では対応せず、必要に応じて個別判断する。

- **完全なコンテナ分離**: PluginHost の seccomp / namespace 対応は OS 依存が大きく、優先度が低い
- **osc.rs の網羅的テスト**: UDP ネットワーク依存のため、モックレベルの最小限テストに留める
- **GUI コードの大幅リファクタ**: DirtyRack はまだ開発初期フェーズ。アーキテクチャが固まってから対応

## 進捗管理

各項目に対応ブランチを作成し、完了後に PR を作成する。

フォーマット:

```markdown
- [ ] P0.1 マージコンフリクト解消 `branch: fix/merge-conflicts`
- [ ] P0.2 PluginHost 最小サンドボックス `branch: fix/plugin-sandbox`
- [ ] P1.1 core unwrap 撲滅 `branch: refactor/core-unwrap`
...
```
