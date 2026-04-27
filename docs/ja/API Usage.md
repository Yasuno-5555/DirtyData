# DirtyData API 使用ガイド

このガイドでは、`dirtydata-sdk` を使用して DirtyData エンジンと対話し、ワークスペースを管理し、カスタム DSP ノードを構築する方法を説明します。

## 1. インストール

`Cargo.toml` に `dirtydata-sdk` を追加します。

```toml
[dependencies]
dirtydata-sdk = { path = "../path/to/dirtydata-sdk" }
```

## 2. 基本的なワークフロー

一般的なワークフローは、**Workspace** を開き、**Patch** を介して **Graph** を定義し、それを **AudioEngine** で実行するという流れになります。

### ワークスペースの初期化

ワークスペースは、プロジェクトの状態と履歴（フォレンジック・レコード）を管理します。

```rust
use dirtydata_sdk::Workspace;

fn main() -> anyhow::Result<()> {
    // 現在のディレクトリでワークスペースを開く、または作成する
    let mut ws = Workspace::open(".")?;
    
    println!("Workspace Root Hash: {:?}", ws.root_hash());
    Ok(())
}
```

### グラフの構築

`NodeFactory` を使用してノードを作成し、`Patch` を使用してそれらを組み立てます。

```rust
use dirtydata_sdk::{NodeFactory, Patch, Operation};
use dirtydata_sdk::types::StableId;

fn create_simple_synth() -> Patch {
    let mut patch = Patch::new();
    
    // 1. ノードの追加
    patch.add_operation(Operation::AddNode(NodeFactory::oscillator(440.0)));
    patch.add_operation(Operation::AddNode(NodeFactory::bit_crush(8.0, 2.0)));
    
    // 2. ノード間の接続（エッジの追加）
    // ※ 実際の API は接続専用のメソッドを提供する場合があります
    
    patch
}
```

## 3. リアルタイム・オーディオ実行

`AudioEngine` は、オーディオデバイスとの低レベルな対話を処理し、グリッチのないグラフ・ホットスワップ機能を提供します。

```rust
use dirtydata_sdk::{AudioEngine, SharedState};
use std::sync::Arc;

fn start_engine() {
    let shared_state = Arc::new(SharedState::new());
    let (midi_tx, midi_rx) = crossbeam_channel::unbounded();
    
    // エンジンの起動
    let engine = AudioEngine::new(shared_state, midi_rx);
    
    // リアルタイムでのパラメータ更新
    // engine.update_parameter(node_id, "frequency".to_string(), 880.0);
}
```

## 4. カスタム DSP ノード（プラグイン）の作成

`DspPlugin` トレイトを実装することで、DirtyData を拡張できます。

```rust
use dirtydata_sdk::{DspPlugin, declare_plugin};

#[derive(Default)]
struct MyDistortion {
    drive: f32,
}

impl DspPlugin for MyDistortion {
    fn init(&mut self, sample_rate: f32) {
        println!("Initialized at {}Hz", sample_rate);
    }

    fn process(&mut self, in_l: f32, in_r: f32) -> [f32; 2] {
        let out_l = (in_l * self.drive).tanh();
        let out_r = (in_r * self.drive).tanh();
        [out_l, out_r]
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        if id == 0 { self.drive = value; }
    }
}

// WASM 互換プラグインとしてエクスポート
declare_plugin!(MyDistortion);
```

## 5. セマンティック・マージ

DirtyData では、「意図（Intent）」を尊重しながら、異なるプロジェクト状態をマージすることができます。

```rust
use dirtydata_sdk::merge::SemanticMerge;

fn merge_collaboration(ws: &mut Workspace, incoming_patch: Patch) -> anyhow::Result<()> {
    // 構造的な競合だけでなく、セマンティックな制約も検証しながらマージ
    SemanticMerge::run(ws, incoming_patch)?;
    Ok(())
}
```
