//! dirtydata-core — Canonical IR / Patch Engine.
pub mod actions;
pub mod constitution;
pub mod dsl;
pub mod exploration;
pub mod graph_utils;
pub mod hash;
pub mod ir;
pub mod mutation;
pub mod mutation_eval;
pub mod patch;
pub mod merge;
pub mod storage;
pub mod types;
pub mod validate;

pub use ir::{Edge, Graph, Node};
pub use patch::{Operation, Patch, PatchError, PatchSet};
pub use types::*;
pub use validate::{validate_commit, ValidationReport};
