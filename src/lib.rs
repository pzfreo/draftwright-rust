//! Draftwright: automated technical drawings from STEP B-rep — Rust port.
//!
//! Drawing logic only: the geometry (projection with hidden lines, sections) comes from the
//! [`haecceity`] kernel, and feature recognition from quiddity.

pub use haecceity;

pub mod output;
pub mod text;
