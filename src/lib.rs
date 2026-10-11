//! Draftwright: automated technical drawings from STEP B-rep — Rust port.
//!
//! Drawing logic only: the geometry (projection with hidden lines, sections) comes from the
//! [`haecceity`] kernel, and feature recognition from quiddity.

pub use haecceity;

pub mod api;
pub mod bridge;
pub mod centrelines;
pub mod completeness;
pub mod demand;
pub mod dims;
pub mod hole_table;
pub mod items;
pub mod layout;
pub mod linework;
pub mod lint;
pub mod model;
pub mod notes;
pub mod output;
pub mod placement_intent;
pub mod pmi;
pub mod pmi_lowering;
pub mod png;
pub mod section;
pub mod session;
pub use session::Session;
pub mod tables;
pub mod text;
pub mod title_block;

pub mod mcp;
pub mod stateless;
