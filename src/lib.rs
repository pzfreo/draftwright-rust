//! Draftwright: automated technical drawings from STEP B-rep — Rust port.
//!
//! Drawing logic only: the geometry (projection with hidden lines, sections) comes from the
//! [`haecceity`] kernel, and feature recognition from quiddity.

pub use haecceity;

pub mod api;
pub mod bridge;
pub mod demand;
pub mod dims;
pub mod layout;
pub mod lint;
pub mod model;
pub mod output;
pub mod png;
pub mod section;
pub mod text;
