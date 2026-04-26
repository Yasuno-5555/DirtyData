# DirtyData SDK API Reference (v0.1)

`dirtydata-sdk` は、DirtyData エコシステムと対話するための唯一の公式な入り口です。
「内輪の神話を、人類の標準へ。」

## 1. Workspace (プロジェクト管理)
`Workspace` は、特定のディレクトリにおけるフォレンジック・レコード（.dirtydata）の所有者です。

### `Workspace::open(path: &str) -> Result<Workspace>`
指定したディレクトリを開き、既存のフォレンジック・レコードをロードします。

### `workspace.apply_patch(patch: Patch) -> Result<()>`
グラフに対してパッチを適用し、Merkle 履歴を更新した上でディスクに永続化します。

### `workspace.audit() -> Result<AuditReport>`
現在のワークスペースの整合性を数学的に検証します。
- `root_hash_valid`: Merkle Root の一致確認
- `lineage_intact`: パッチ履歴のハッシュチェーン検証
- `cas_complete`: 参照されている回路の存在確認

---

## 2. NodeFactory (ノード生成)
DSL や JSON を直接書く代わりに、Rust の型安全なコードでノードを生成できます。

### `NodeFactory::oscillator(freq: f32) -> Node`
基本のオシレーター（Sine, Saw, Square）を生成します。

### `NodeFactory::bit_crush(bits: f32, sr_div: f32) -> Node`
破壊的なビット・リダクション・ノードを生成します。

### `NodeFactory::chua_circuit() -> Node`
カオス的な非線形発振を行う Chua 回路ノードを生成します。

---

## 3. SemanticMerge (高度な統合)
複数のフォレンジック履歴を統合する際の、意図（Intent）に基づいたマージ機能です。

### `merge::SemanticMerge::run(ws: &mut Workspace, patch: Patch) -> Result<()>`
単なるトポロジーの結合ではなく、既存の制約（Constraints）を侵害していないかを検証しながらマージを実行します。

---

## 4. Built-in DSP Modules
SDK は以下の DSP クレートを直接再エクスポートしており、カスタムプラグイン内から自由に呼び出し可能です。

- `spectral`: FFT/周波数ドメイン処理
- `chaos`: 非線形動学モデル（Chua, Lorenz, Mackey-Glass）
- `zdf`: ゼロ遅延フィードバック・フィルター群
- `destruction`: 歪み、ビット破砕、PLL
- `tape`: アナログテープ・サチュレーション、ワウ・フラッター
- `builtin`: ランタイム標準ノード（Gain, Biquad, Compressor 等）

---

## 5. Python Interop (`dirtydata-py`)
Python 側からは NumPy 配列を介して「現実」を観測します。

```python
import dirtydata as dd

# Chua回路の信号をNumPy配列として生成
signal = dd.chua_oscillator(n_samples=44100, rate=1.2)

# ラダーフィルターを通す
filtered = dd.process_ladder_filter(signal, cutoff=1000.0, resonance=0.8, sample_rate=44100)
```

---

## 6. Stability Policy (RFC-002)
- **v1.x**: コア IR 構造および SDK の主要 API は後方互換性が維持されます。
- **Deprecation**: API の削除には最低 6 ヶ月の猶予期間が設けられます。
