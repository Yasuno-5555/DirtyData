# DirtyRack 開発者ガイド

DirtyRack 開発者エコシステムへようこそ。このガイドでは、宇宙で最も「決定的（Deterministic）」なモジュラーシンセサイザーのためのモジュールを開発、テスト、配布するために必要なすべてを解説します。

## 1. コア哲学: 信号憲法 (The Constitution)

すべての DirtyRack モジュールは「鑑識可能な成果物 (Forensic Artifact)」です。Merkle DAG（パッチ履歴）の完全性を維持するため、すべてのモジュールは以下のルールを厳守しなければなりません。

1.  **ビット精度の決定論**: 同じ入力サンプル、パラメータ、`project_seed` が与えられた場合、あなたのモジュールはすべてのマシン（Windows, Mac, Linux）で **完全に同一** の出力を生成しなければなりません。
2.  **副作用の禁止**: `process()` ループ内で `std::time`、`rand::thread_rng()`、またはファイル I/O を絶対に使用しないでください。提供される `RackProcessContext` を使用してください。
3.  **リアルタイム安全**: 処理中に動的なメモリ確保（`Vec::new()`、`Box::new()` など）やブロッキングロックを行わないでください。
4.  **16ボイス・ポリフォニー**: DirtyRack は標準で16チャンネルです。常に16チャンネルを処理するか、パフォーマンスのために SIMD (`f32x4`) を使用してください。

## 2. 環境構築

Rust ツールチェーンがインストールされている必要があります。

```bash
# Wasm ターゲットの追加 (強く推奨)
rustup target add wasm32-wasip1
```

## 3. モジュールの作成 (Wasm ターゲット)

Wasm はモジュール配布の推奨方法です。安全でクロスプラットフォームであり、`wasmtime` サンドボックス内で実行されます。

### ステップ 1: プロジェクトの初期化
```bash
cargo new my-chaos-gen --lib
cd my-chaos-gen
```

### ステップ 2: Cargo.toml の設定
```toml
[package]
name = "my-chaos-gen"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"] # Wasm または共有ライブラリに必要

[dependencies]
dirtyrack-sdk = { path = "../DirtyRack/crates/dirtyrack-sdk" }
```

### ステップ 3: 実装 (`src/lib.rs`)
```rust
use dirtydata_sdk::{DspPlugin, declare_plugin};

#[derive(Default)]
struct MyChaosGen {
    phase: f32,
    freq: f32,
}

impl DspPlugin for MyChaosGen {
    fn init(&mut self, _sample_rate: f32) {
        self.freq = 440.0;
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        if id == 0 {
            self.freq = 20.0 + value * 2000.0; // 簡単なマッピング
        }
    }

    fn process(&mut self, _in_l: f32, _in_r: f32) -> [f32; 2] {
        self.phase += self.freq / 44100.0;
        if self.phase > 1.0 { self.phase -= 1.0; }
        
        let out = (self.phase * 2.0 * 3.141592).sin();
        [out, out]
    }
}

// Wasmのエントリポイントをエクスポート
declare_plugin!(MyChaosGen);
```

### ステップ 4: Wasm ビルド
```bash
cargo build --target wasm32-wasip1 --release
```
生成された `.wasm` ファイルは、`Foreign` ノードとして直接 DirtyRack にロード可能です。

## 4. Dirty CLI の活用

`dirty` CLI ツールは、鑑識オーディオエンジニアリングの相棒です。

### プロジェクトの初期化
```bash
dirty init my_song
cd my_song
```

### パッチの適用
JSON 形式でパッチを記述し、ラックを構築できます。
```bash
dirty patch my_patch.json
```

### パラメータの変異 (Mutate)
進化エンジンを使って新しい音を探索します。
```bash
dirty log --graph # ノード ID を取得
dirty mutate <NODE_ID> --level wild --epochs 100
dirty patch patch_<HASH>.json # 変異を適用
```

### 監査 (Auditing)
プロジェクトが改ざんされていないか、決定論が維持されているかを確認します。
```bash
dirty doctor
```

## 5. 応用: 鑑識データ (Forensic Data)

ユーザーが「なぜ音が変わったのか」を理解するのを助けるために、`get_forensic_data` を実装してください。これにより、DirtyRack の GUI 上でフィルターの飽和度や隠れたモジュレーション状態などを可視化できます。

```rust
fn get_forensic_data(&self) -> Option<ForensicData> {
    let mut data = ForensicData::default();
    data.internal_state_summary = format!("Oscillator Phase: {:?}", self.phase);
    Some(data)
}
```

---

DirtyRack は単なるシンセではありません。音の「記録」です。ハッピー・ハッキング！
