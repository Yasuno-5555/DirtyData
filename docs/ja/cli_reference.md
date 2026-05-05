# DirtyData CLI Reference

`dirty` は、システムと人間をつなぐ唯一の公式なインターフェースです。
「すべての状態は説明可能か、さもなくば破棄可能でなければならない」という哲学のもと、様々なコマンドが提供されています。

## 基本コマンド

### `dirty init`
現在のディレクトリを DirtyData プロジェクトとして初期化します。
`.dirtydata/` ディレクトリが生成され、デフォルトの `main` ブランチが作成されます。

### `dirty status`
現在のグラフ状態、ノード/エッジ数、直近のパッチ履歴、Active Intent、およびシステムの「信頼性スコア (Confidence Score)」を視覚的に表示します。

### `dirty doctor`
フォレンジック監査（Forensic Audit）を実行します。
Merkle Root の整合性、パッチ履歴のハッシュ、CAS（Content Addressable Storage）の完全性を検証し、プロジェクトの「正当性」を数学的に証明します。

### `dirty snapshot <NAME>`
現在の状態を名前付きスナップショットとして保存します。

---

## タイムラインと意図 (Timeline & Intent)

### `dirty patch apply <FILE> [--intent <INTENT_ID>]`
JSON 形式のパッチファイルを適用します。

### `dirty intent add <DESCRIPTION>`
新しい Intent（意図・制約）をシステムに登録します。

### `dirty branch [NAME] / dirty checkout <NAME>`
ブランチの作成と切り替えを行います。

---

## フォレンジック・インタラクティブ・パッチング (DirtyRack)

ターミナルから直接、あるいは GUI と連携して複雑なパッチ構築を行うためのコマンド群です。

### `dirtyrack new <PATCH> [--template <NAME>]`
新しいパッチファイルを生成します。

### `dirtyrack inspect <PATCH>`
パッチの構造（モジュール、結線、エイリアス）を詳細に表示します。

### `dirtyrack shell <PATCH>`
**対話型パッチング・シェル (Interactive Shell)** を起動します。
高速な構築を可能にする REPL 環境を提供します。

- **階層ナビゲーション (Navigation)**:
  - `ls`: 現在の階層のモジュール一覧を表示。
  - `cd <ID/ALIAS>`: サブパッチの内部へ移動。
  - `cd ..`: 親階層に戻る。
  - `pwd`: 現在の編集パスを表示。
- **編集コマンド (Editing)**:
  - `add <MODULE_ID>`, `rm <ID/ALIAS>`: モジュールの追加と削除。
  - `connect <FROM> <PORT> <TO> <PORT>`: ポート間の結線。
  - `set <ID> <PARAM> <VALUE>`: パラメータ設定。
  - `multiply <COUNT> <MODULE_ID>`: モジュールを一括配置。

---

## GUI 統合コマンド (Summoner HUD)

DirtyRack GUI は、マウスの直感性と CLI のパワーを融合させています。

### `召喚 (SUMMONER)`
GUI 上で `Space` または `Enter` キーを押すと、マウスカーソル位置にコマンドバーが出現します。

- **アドホック・デプロイ**: `add dirty_vco` と打てば、**マウスが指している位置**に即座にモジュールが召喚されます。
- **高速結線**: `conn 1 out 2 in` のようにキーボードで打つことで、マウス操作よりも遥かに速く複雑な配線を完結できます。
- **集団召喚**: `mul 16 dirty_vco` により、大量のオシレーターを瞬時に整列配置できます。

---

## 階層型サブパッチ設計 (Hierarchical Subpatching)

複雑な回路を一つのモジュールとしてカプセル化し、再利用可能です。

- **Composite Module**: 外部パッチファイルを読み込み、再帰的に実行します。
- **IO ブリッジ**:
  - `subpatch_in / subpatch_out`: 階層を越えた信号の受け渡し。
  - `subpatch_param (Macro Knob)`: 親パッチからの入力を受け取る「マクロノブ」として機能し、内部回路（DirtyData MNA等）をコントロールします。

---

## 出力とエクスポート

### `dirtyrack render`
オフラインレンダリングを実行し、WAV ファイルを出力します。

### `dirtyrack verify <AUDIO_WAV> <CERT_JSON>`
**音響公証 (Acoustic Notarization)** を実行し、オーディオファイルが指定のパッチから生成されたものであることを数学的に証明します。
