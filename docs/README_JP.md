# DirtyRack

**Deterministic Eurorack Simulator & Modular DSP Engine**

DirtyRack は、ビット単位の再現性と物理的なモジュラー操作を融合させた、**高精度決定論的ユーロラック・シミュレータ**です。アナログの「カオス」や「揺らぎ」を愛しながらも、デジタルの「説明責任（Accountability）」を追求するアーティストとエンジニアのために設計されました。

単なるシンセサイザーではなく、音の状態を医学的な症例のように診断し、その因果関係を証明可能にする **Forensic Audio Infrastructure（音響鑑識インフラ）** です。

## 核心的設計思想

*   **決定論的カオス (Gehenna Engine)**: カオスアトラクタや非線形フィードバックを扱いながら、同一シード・同一入力からは常に 1bit の狂いもない出力を保証する。
*   **Massive Polyphony (16ch)**: VCV Rack 互換の 16 チャンネル・ポリフォニック・ケーブルを標準搭載。一本の結線がオーケストラのような密度を生む。
*   **Acoustic Forensics（音響鑑識）**: 「なぜこの音になったのか」を客観的に証明する鑑識レイヤー。**Patch MRI** により、内部の熱状態や個体差をリアルタイムに解剖可能。
*   **規格こそが真理**: **.dirtyrack Open Specification** を遵守。作成した音響は、将来にわたってポータブルかつ検証可能であり続ける。
*   **因果関係の観測**: **Divergence Map** や **Provenance Timeline** により、音作りの過程（意図）を完全に遡ることが可能。

## 主な機能

1.  **Gehenna Parallel Engine**: SIMD 最適化された第 2 世代並列 DSP エンジン。16 ボイス全てに決定論的な「機材の個性」を付与。
2.  **Patch MRI (病理スキャン)**: 信号の「外傷」を可視化。クリッピング（発光）、エネルギー密度（ヒートマップ）、DC ドリフト（オーラ）を面パネル上に直接表示。
3.  **Provenance Timeline**: パラメータ変更やスナップショットの全履歴。音の背後にある「意図」を地図化。
4.  **Forensic Inspector**: モジュールの内部状態を解剖。**Explain Why** ボタンにより、信号の異常に対する医学的な診断レポートを生成。
5.  **.dirtyrack Spec v1.0**: パッチ (`.dirtyrack`) と監査証明 (`.dirtyrack.cert`) の標準規格。音響の「公正証書化」を実現。
6.  **Differential Audit**: スナップショットやレンダリング結果をサンプル精度で比較。何が音を変えたのかを正確に特定。
7.  **Verification CLI**: CI/CD や運用環境向けのコマンドライン・ツール。`dirty verify` により、出力音声の完全性を証明。

### Distribution & Formats

- **Standalone App**: macOS 用 `.app` バンドルを提供。`/Applications` にドラッグ＆ドロップで即座に利用可能。
- **VST3 / CLAP Plugin**: DAW (Ableton, Bitwig, Reaper, etc.) 内で 16ch ポリフォニック・モジュラーとして動作。
- **CLI Tool**: 決定論的なオフラインレンダリングとハッシュ検証のためのコマンドライン・インターフェース。

## クイックスタート

### Standalone (macOS)
```bash
# ルートの DirtyRack.app を /Applications にコピー
open ./DirtyRack.app
```

### Plugin Deployment
ルートに生成された `DirtyRack.clap` をプラグインフォルダに配置してください。

**VST3 として使用する場合**:
1. `DirtyRack.vst3/Contents/MacOS/` というディレクトリ構造を作成。
2. その中に `DirtyRack.clap` を `DirtyRack` という名前でコピーして配置してください。

## プロジェクト構造

```text
DirtyRack/
├── crates/
│   ├── dirtyrack-sdk/      # サードパーティ開発用 SDK。コア・トレイトと SIMD ユーティリティ。
│   ├── dirtyrack-modules/  # 決定論的 DSP モジュール兵器廠（VCO, VCF, Chaos, etc.）
│   ├── dirtyrack-gui/      # egui ベースの「プロジェクター」。Triple-Buffer 同期。
│   └── dirtyrack-core/     # 決定論的基盤。因果関係を管理する DAG エンジン。
├── docs/                   # SDK ドキュメント、アーキテクチャ、哲学。
└── modules/                # サードパーティ製動的ライブラリ (.so, .dll) の配置場所。
```

## ドキュメント

- [Design Philosophy (設計哲学)](docs/design_philosophy.md)
- [Architecture (アーキテクチャ)](docs/architecture.md)
- [SDK Documentation (開発者ガイド)](docs/SDK_Documentation.md)
- [Creating Your First Module (チュートリアル)](docs/Tutorial_Creating_Modules.md)

## ライセンス

MIT License
