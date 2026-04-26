# DirtyData プラグイン開発ガイド

DirtyDataエコシステムへようこそ。このガイドでは、**DirtyData SDK** を使用してカスタムDSPノードを構築する方法を説明します。

## 1. 前提条件

- [Rust](https://www.rust-lang.org/) がインストールされていること。
- Rustの `wasm32-unknown-unknown` ターゲット:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

## 2. プラグイン・プロジェクトの作成

新しいライブラリ・クレートを作成します：
```bash
cargo new my-awesome-dsp --lib
cd my-awesome-dsp
```

`Cargo.toml` に `dirtydata-sdk` を追加します。SDKは、カスタムプラグイン開発だけでなく、既存の高度なDSPコンポーネント（カオス系、回路シミュレーション等）へのアクセスも提供します。

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
dirtydata-sdk = { git = "https://github.com/Yasuno-5555/DirtyData" }
```

## 3. SDKの活用：ビルトインDSPの利用
DirtyData SDKは、コアシステムが提供する18種類以上のDSPクレートを再エクスポートしています。これらを組み合わせて、より複雑なプラグインを迅速に構築できます。

```rust
use dirtydata_sdk::{DspPlugin, declare_plugin, chaos, zdf};

#[derive(Default)]
pub struct ChaoticFilter {
    chua: chaos::ChuaCircuit,
    filter: zdf::LadderFilter,
}

impl DspPlugin for ChaoticFilter {
    fn init(&mut self, sample_rate: f32) {
        self.chua = chaos::ChuaCircuit::new(sample_rate);
        self.filter = zdf::LadderFilter::new(sample_rate);
    }

    fn process(&mut self, in_l: f32, in_r: f32) -> [f32; 2] {
        // カオス発振器でカットオフを変調
        let mod_val = self.chua.process(15.6, 28.0, 1.0);
        let cutoff = 1000.0 + mod_val * 500.0;
        
        let out_l = self.filter.process(in_l, cutoff, 0.707);
        let out_r = self.filter.process(in_r, cutoff, 0.707);
        [out_l, out_r]
    }
}
```

## 4. カスタムDSPの実装

`src/lib.rs` に記述します：

```rust
use dirtydata_sdk::{DspPlugin, declare_plugin};

#[derive(Default)]
pub struct MyPlugin {
    gain: f32,
}

impl DspPlugin for MyPlugin {
    fn set_parameter(&mut self, id: u32, value: f32) {
        if id == 0 { self.gain = value; }
    }

    fn process(&mut self, in_l: f32, in_r: f32) -> [f32; 2] {
        [in_l * self.gain, in_r * self.gain]
    }
}

declare_plugin!(MyPlugin);
```

## 4. DirtyData向けビルド

プラグインをWASMにコンパイルします：
```bash
cargo build --target wasm32-unknown-unknown --release
```

## 5. インストール

生成された `.wasm` ファイルをDirtyDataプラグインディレクトリにコピーします：
- **macOS/Linux**: `~/.dirtydata/plugins/`
- **Windows**: `%APPDATA%\DirtyData\plugins\`

## 6. CLI経由での利用

1. プロジェクトの初期化: `dirty init`
2. トポロジーにWASMノードを追加:
   ```bash
   # DSLまたはJSONの直接編集でカスタムノードを追加します
   # 'path' コンフィグに .wasm ファイルのパス、またはCAS内のBLAKE3ハッシュを指定します
   ```
3. 検証と監査: `dirty doctor`
4. VST3へのコンパイル: `dirty build --target vst3`

---

### SSS+: 高度な統合

「Experimental（試験的）」ステータスのノードについては、ドキュメント内に **Confidence Metadata** ブロックを提供してください。これにより、Constraint Engine（制約エンジン）がノードの挙動を検証できるようになります。
