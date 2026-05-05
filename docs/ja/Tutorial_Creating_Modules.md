# チュートリアル：初めてのDirtyRackモジュールを作ろう！

このガイドでは、音を大きくしたり小さくしたりする単純な「Gain（ゲイン）モジュール」を例に、DirtyRackの世界に自分のモジュールを追加する方法を解説します。

## 0. 準備するもの
- **Rustの開発環境**: パソコンにRustがインストールされていること。
- **ちょっとした好奇心**: 「音をプログラムで汚す」という楽しさを理解する心。

---

## ステップ1：プロジェクトを立ち上げる

まずは、モジュールの「器」となるプロジェクトを作ります。ターミナル（黒い画面）で以下のコマンドを打ちましょう。

```bash
cargo new my-gain-module --lib
cd my-gain-module
```

---

## ステップ2：設定ファイル（Cargo.toml）を書く

DirtyRackに「これは動くモジュールですよ」と教えるための設定を書きます。`Cargo.toml` を開いて、中身を以下のように書き換えてください。

```toml
[package]
name = "my-gain-module"
version = "0.1.0"
edition = "2021"

[lib]
# DirtyRackが読み込める「動的ライブラリ」形式で書き出します
crate-type = ["cdylib"]

[dependencies]
# DirtyRackの開発キット（SDK）を呼び出します
dirtyrack-sdk = { path = "../path/to/dirtyrack-sdk" } # 実際のパスに合わせてね
```

---

## ステップ3：プログラムを書く（src/lib.rs）

いよいよ中身です。`src/lib.rs` の中身を全部消して、以下のコードを貼り付けてください。

```rust
use dirtyrack_sdk::*;

// --- モジュールの「設計図」 ---
#[dirty_module] // これを書くだけで、面倒な「おまじない」を自動でやってくれます！
pub struct MyGainModule {
    // ここにモジュールの状態（メモリ）を保存できます
    // 今回は単純な音量調整なので、特に何も持ちません
}

impl MyGainModule {
    // モジュールが作られた瞬間に呼ばれる関数
    pub fn new(_sample_rate: f32) -> Self {
        Self {}
    }
}

// --- 音の「加工ルール」 ---
impl RackDspNode for MyGainModule {
    fn process(
        &mut self,
        inputs: &[f32],      // 入ってきた電気信号（音）
        outputs: &mut [f32], // 加工して出す電気信号
        params: &[f32],      // つまみ（ノブ）の値
        ctx: &RackProcessContext, // 「古さ」や「ゆらぎ」の情報が入った魔法のノート
    ) {
        // 0番目のつまみの値（音量）を取得します
        let gain_knob = params[0];
        
        // DirtyRackは常に16人合奏（16ボイス）！全員分ループで処理します
        for i in 0..16 {
            let input_signal = inputs[i]; // i番目の人の音を取り出す
            
            // 【ここがDirty！】
            // ただ掛け算するのではなく、SDKの便利な機能を使います
            let dirty_gain = gain_knob
                .apply_drift(i, ctx) // 機械ごとの「個体差」や「熱によるゆらぎ」をプラス！
                .apply_aging(i, ctx); // Agingノブの設定に合わせて「音の劣化」をプラス！

            // 最後に、計算した値を出す
            outputs[i] = input_signal * dirty_gain;
        }
    }

    // 鑑識（Forensic）画面で「今どうなってる？」を見せるための設定
    fn get_forensic_data(&self) -> Option<ForensicData> {
        let mut data = ForensicData::default();
        data.internal_state_summary = "音を元気に大きくしています！".to_string();
        Some(data)
    }

    // おまじない：これが必要な理由は「大人の事情（トレイトの制約）」です
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self.as_any_mut_impl() // マクロが作ってくれた関数を呼ぶだけ！
    }
}

// --- 見た目（パネル）の定義 ---
fn my_descriptor() -> &'static ModuleDescriptor {
    &ModuleDescriptor {
        id: "com.yourname.gain",
        name: "My First Gain",
        version: "1.0.0",
        manufacturer: "Independent Artist",
        hp_width: 4, // モジュールの横幅（スリムな4HP）
        
        // デザインの設定
        visuals: ModuleVisuals {
            background_color: [40, 45, 50], // 深い灰色のパネル
            text_color: [255, 255, 255],    // 白い文字
            accent_color: [255, 100, 0],    // オレンジ色のアクセント
            panel_texture: PanelTexture::BrushedAluminium, // アルミの質感
            knob_style: KnobStyle::VintageBakelite, // 渋いベークライト製ノブ
        },
        
        // つまみの配置
        params: &[
            ParamDescriptor {
                name: "音量",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 10.0 }, // 急に回してもプチプチ言わない
                min: 0.0, max: 2.0, default: 1.0,
                position: [0.5, 0.4], // パネルの真ん中らへん
                unit: "x",
            },
        ],
        
        // 穴（ポート）の配置
        ports: &[
            PortDescriptor { name: "IN", direction: PortDirection::Input, signal_type: SignalType::Audio, position: [0.5, 0.7], max_channels: 16 },
            PortDescriptor { name: "OUT", direction: PortDirection::Output, signal_type: SignalType::Audio, position: [0.5, 0.9], max_channels: 16 },
        ],
        
        factory: |sr| Box::new(MyGainModule::new(sr)),
    }
}

// DirtyRackの世界へエクスポート！
export_dirty_module!(my_descriptor);
```

---

## ステップ4：ビルドしてインストール！

プログラムが書けたら、実際に動く「ファイル」に変換します。

```bash
cargo build --release
```

成功すると、`target/release/` フォルダの中にファイルができます：
- macOS: `libmy_gain_module.dylib`
- Linux: `libmy_gain_module.so`
- Windows: `my_gain_module.dll`

このファイルを、DirtyRack本体がある場所の `modules/` という名前のフォルダに放り込みましょう。

---

## ステップ5：DirtyRackで確認

DirtyRackを起動して、モジュール追加画面を見てください。**"My First Gain"** が並んでいれば大成功です！

---

## もっと面白くするためのヒント
- **音を歪ませる**: `output` に出す前に、値をわざとクリップ（制限）させてみましょう。
- **履歴を覚える**: `struct` の中に「さっきの音の値」を保存しておけば、エコーやフィルターも作れます。
- **Agingを極める**: Agingが1.0の時だけ、盛大にノイズを混ぜるようにプログラムを書いてみましょう。
