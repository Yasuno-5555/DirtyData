# RFC 001: DirtyData Spec (v1.0.0)

## Status: PROPOSED
## Date: 2026-04-26

## 1. Abstract
DirtyData Spec is a forensic record format for deterministic sound synthesis. It captures not just the state of a DSP graph, but the causality, intent, and lineage of every change.

## 2. Directory Structure
The workspace is a directory containing the following:
- `manifest.json`: Metadata, Versioning, and Root Hash.
- `topology.ir`: Layer 1 (Current Graph State).
- `lineage.dag`: Layer 3 (History & Snapshots).
- `intents.json`: Layer 4 (Semantic Meaning).
- `circuits/blake3/`: Layer 2 (Content Addressed Circuit Definitions).

## 3. Layer 1: Topology (`topology.ir`)
A JSON object containing:
- `nodes`: Map of `StableId` to `Node` definitions.
- `edges`: Map of `StableId` to `Edge` definitions.
- `modulations`: Map of `StableId` to `Modulation` definitions.

## 4. Layer 2: Circuit Registry (`circuits/blake3/`)
All circuit definitions MUST be stored using Content Addressed Storage (CAS).
Path format: `circuits/blake3/{HH}/{HH}/{HASH_HEX}`
Hash algorithm: **BLAKE3**.

## 5. Layer 3: Lineage (`lineage.dag`)
A directed acyclic graph of `Patch` objects.
- `applied_patches`: Linear sequence of applied patch IDs.
- `history`: Full map of patch data for replayability.

## 6. Layer 5: Verification & Manifest
The `manifest.json` provides the project's identity.
- `root_hash`: A recursive BLAKE3 hash covering Layers 1-4.
- `trust_state`: "verified", "suspicious", or "quarantined".
- `signature`: Ed25519 signature of the root_hash (Optional).

## 7. Canonical Serialization
To ensure stable hashes, all JSON output MUST:
1. Sort keys alphabetically.
2. Use 2-space indentation.
3. Use UTF-8 encoding.

## 8. Stability IDs
All IDs MUST be `StableId` (ULID format). Index-based referencing is STRICTLY FORBIDDEN.
