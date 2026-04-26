# DirtyData CLI Reference

`dirty` は、システムと人間をつなぐ唯一の公式なインターフェースです。
「すべての状態は説明可能か、さもなくば破棄可能でなければならない」という哲学のもと、様々なコマンドが提供されています。

## 基本コマンド

### `dirty init`
現在のディレクトリを DirtyData プロジェクトとして初期化します。
`.dirtydata/` ディレクトリが生成され、デフォルトの `main` ブランチが作成されます。

### `dirty status`
現在のグラフ状態、ノード/エッジ数、直近のパッチ履歴、Active Intent、およびシステムの「信頼性スコア (Confidence Score)」を視覚的に表示します。
このコマンドは、プロジェクトがどの程度「正しく説明可能な状態か」を確認するための最重要コマンドです。

### `dirty doctor`
フォレンジック監査（Forensic Audit）を実行します。
Merkle Rootの整合性、パッチ履歴のハッシュ、CAS（Content Addressable Storage）の完全性を検証し、プロジェクトの「正当性」を数学的に証明します。
不整合が発見された場合は、詳細な不整合箇所と、修復（Repair）の必要性をレポートします。

### `dirty snapshot <NAME>`
現在の状態を名前付きスナップショットとして保存します。

## パッチ操作

DirtyData では、状態の変更はすべてパッチを通じて行われます。

### `dirty patch apply <FILE> [--intent <INTENT_ID>]`
JSON 形式のパッチファイルを適用します。
内部的には、`UserAction` を `Operation` にコンパイルし、グラフの Revision を進め、現在のブランチの HEAD を更新します。

### `dirty patch list`
現在のブランチに適用されているパッチの履歴を一覧表示します。

### `dirty patch replay [--verify]`
現在の履歴に記録されているすべてのパッチを最初から再生（リプレイ）し、最終的な状態が現在のグラフと完全に一致するか（決定論的か）を検証します。

## タイムラインとブランチ (Timeline)

### `dirty branch [NAME]`
新しいブランチをフォークします。名前を省略した場合は現在のブランチ一覧を表示します。

### `dirty checkout <NAME>`
指定したブランチに切り替えます。
IR のポインタをスワップするだけで瞬時に別の状態へ遷移します。

## デーモンと監視 (Observer & Runtime)

### `dirty daemon`
プロジェクトディレクトリの変更監視と、リアルタイムのオーディオ再生（cpal）をバックグラウンドで開始します。

### `dirty observe`
外部ファイルシステム（WAV ファイル等）のハッシュやタイムスタンプを再計算します。

### `dirty repair <NODE_NAME>`
Observer によって検知された「意図しないハッシュの不一致」に対し、現在の外部ファイルの状態を「正しい」ものとして再定義します。

## 意図の管理 (Intent)

### `dirty intent add <DESCRIPTION> [--must <...>] [--prefer <...>] [--avoid <...>] [--never <...>]`
新しい Intent（意図・制約）をシステムに登録します。

### `dirty intent list`
現在システムに登録されている Intent の一覧を表示します。

### `dirty intent attach <INTENT_ID> <PATCH_ID>`
既存のパッチに Intent を紐付けます。

## 高度な操作とシミュレーション

### `dirty mutate <NODE> [--level <LEVEL>] [--count <N>]`
指定したノードのパラメーターを進化（Mutate）させます。
- **Level**: safe, wild, radioactive (デフォルト: wild)
- **Count**: 繰り返す回数 (デフォルト: 100)

### `dirty freeze <NODE_NAME> [--length <SEC>]`
ノードの出力を決定論的なアセット（WAV）としてフリーズ（固定）します。

### `dirty null-test [--length <SEC>]`
エンジンの決定論性を証明するための数学的なヌルテストを実行します。

### `dirty install <CRATE_NAME> [--version <VER>]`
外部の DSP クレートをエコシステムへインストールします。

### `dirty preset export/import`
ノードの設定をプリセットとして書き出し、または読み込みます。

## 出力とエクスポート

### `dirty render [--output <FILE>] [--length <SEC>] [--sample-rate <HZ>]`
現在のグラフをオフラインでレンダリング（Deterministic Bounce）し、WAV ファイルとして出力します。

### `dirty export <FORMAT>`
グラフを別の形式でエクスポートします。
- `dsl`: 人間が読みやすい Surface DSL 形式。
- `json`: JSON 形式。
- `vst3`: VST3 プラグイン形式。
- `clap`: CLAP プラグイン形式。
