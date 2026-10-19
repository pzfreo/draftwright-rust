//! Scope vocabulary shared with independent lint advice; no drawing or authoring state.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Local,
    View,
    Sheet,
}

/// Conservative scope when no addressable edit paths are available.
pub const UNBOUND_SCOPE: Scope = Scope::Sheet;
