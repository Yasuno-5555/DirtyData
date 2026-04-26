# DirtyData: Headless Forensic Audio Workbench

> "GUIは観光客のため。CLIは司法（エンジニア）のため。"

DirtyDataは、決定論的な音響再構成、セマンティック・バージョニング、および進化的パッチ適用を目的とした、ヘッドレスな音響フォレンジック・エンジンです。音を一時的なバッファとしてではなく、検証可能な「意図の系譜（Lineage）」として扱います。

---

## 🏛 アーキテクチャ

### Layer 1: The Nervous System (`dirtydata-core`)
音響フォレンジックの見えない背骨。
- **Forensic IR**: 決定論的なトポロジー表現。
- **Merkle DAG**: 暗号学的に検証可能な履歴。
- **Semantic Merge**: 意図の優先順位に基づく衝突解決。
- **JIT Compiler**: 高性能なDSP実行。

## 🧬 ドメイン分離（Domain Isolation）
DirtyDataは「現実（Reality）」と「観測（Observation）」を厳格に分離します。

- **🟥 Rust: 現実の階層**
    - DSPグラフ、MNAソルバー、JIT、Merkle履歴。
    - パフォーマンスと決定論的な正確性を司る「核」。
- **🟦 Python: 観測の階層**
    - データフェッチ、ML学習、波形分析、プロット。
    - Jupyter Notebook 等での研究・探索・学習を司る。
- **🟨 禁止領域 (Forbidden Region)**
    - Python側からのグラフ直接変更、リアルタイムDSP実行、ソルバーロジックの再実装は厳禁。
    - 観測層は常に「現実」を NumPy 配列として読み取るのみ。

### Layer 2: The Judiciary (`dirty` CLI)
フォレンジック・エンジニアのための主要インターフェース。Neovim + Tmux ワークフローに最適化されています。

```bash
# 新しいフォレンジック・レコードを初期化
dirty init

# 整合性監査（Forensic Audit）の実行
dirty doctor

# セマンティックな系譜と意図の鎖を表示
dirty log --graph

# ヘッドレス・バッチ変異（進化計算による探索）
dirty mutate tb303 --level radioactive --epochs 10000

# IRをスタンドアロンの Vst3/Clap プラグインへと変換
dirty build --target vst3
```

## 🛠 ワークフロー

1.  **Edit**: `topology.ir` や `.dsl` ファイルをエディタで直接編集。
2.  **Verify**: `dirty verify` で規格準拠とハッシュの整合性を確認。
3.  **Commit**: `dirty patch` を通じて変更を適用し、新たな Merkle リンクを生成。
4.  **Manifest**: `dirty build` でプロダクション用バイナリを書き出し。

## 📜 仕様
- **RFC 001**: [フォレンジック規格](docs/ja/RFC_001_DIRTYDATA_SPEC.md)
- **RFC 002**: [API Stability Policy](docs/RFC_002_API_STABILITY.md)
- **RFC 003**: [Architecture Manifesto](docs/RFC_003_ARCHITECTURE_MANIFESTO.md)
- **Architecture**: [コア設計](docs/ja/architecture.md)
- **Plugin Development**: [SDKガイド](docs/ja/plugin_development.md)
