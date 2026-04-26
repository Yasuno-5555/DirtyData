# RFC 001: DirtyData Spec (v1.0.0)

## ステータス: PROPOSED
## 日付: 2026-04-26

## 1. 概要
DirtyData Specは、決定論的な音響合成のためのフォレンジック記録フォーマットです。DSPグラフの状態だけでなく、すべての変更の因果関係（Causality）、意図（Intent）、および系譜（Lineage）を記録します。

## 2. ディレクトリ構造
ワークスペースは、以下のファイルを含むディレクトリです：
- `manifest.json`: メタデータ、バージョニング、およびルートハッシュ。
- `topology.ir`: Layer 1 (現在のグラフ状態)。
- `lineage.dag`: Layer 3 (履歴とスナップショット)。
- `intents.json`: Layer 4 (セマンティックな意味)。
- `circuits/blake3/`: Layer 2 (コンテンツ・アドレス指定された回路定義)。

## 3. Layer 1: Topology (`topology.ir`)
以下の内容を含むJSONオブジェクト：
- `nodes`: `StableId` から `Node` 定義へのマップ。
- `edges`: `StableId` から `Edge` 定義へのマップ。
- `modulations`: `StableId` から `Modulation` 定義へのマップ。

## 4. Layer 2: Circuit Registry (`circuits/blake3/`)
すべての回路定義は、コンテンツ・アドレス指定ストレージ（CAS）を使用して保存しなければなりません（MUST）。
パス形式: `circuits/blake3/{HH}/{HH}/{HASH_HEX}`
ハッシュアルゴリズム: **BLAKE3**。

## 5. Layer 3: Lineage (`lineage.dag`)
`Patch` オブジェクトの有向非巡回グラフ（DAG）。
- `applied_patches`: 適用されたパッチIDの線形シーケンス。
- `history`: 再現性のためのパッチデータの完全なマップ。

## 6. Layer 5: Verification & Manifest
`manifest.json` はプロジェクトのアイデンティティを提供します。
- `root_hash`: Layer 1-4をカバーする再帰的なBLAKE3ハッシュ。
- `trust_state`: "verified"（検証済み）、"suspicious"（疑わしい）、または "quarantined"（隔離）。
- `signature`: root_hash の Ed25519 署名（任意）。

## 7. 標準的なシリアライズ
ハッシュの安定性を保証するため、すべてのJSON出力は以下を守らなければなりません（MUST）：
1. キーをアルファベット順にソートする。
2. 2スペースのインデントを使用する。
3. UTF-8 エンコーディングを使用する。

## 8. 安定ID（Stability IDs）
すべてのIDは `StableId`（ULID形式）でなければなりません（MUST）。インデックスベースの参照は厳格に禁止されています（STRICTLY FORBIDDEN）。
