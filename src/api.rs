//! The operations every adapter calls (ADR R1): the command line, the JSON-lines service
//! (`draftwright serve`) and the MCP server (`draftwright-mcp`) are thin layers over these.
//!
//! - [`analyse`], [`plan`], [`edit`], [`render`] (with [`write`]), [`spec`] and [`review`],
//!   typed, over the documents of [`crate::model`]; [`plan_analysed`], [`render_analysed`] and
//!   [`review_analysed`] take an Analysis that [`analyse`] made, so the caller need not re-analyse;
//!   [`analyse_from`], [`plan_from`], [`render_from`] and [`review_from`] take where the features
//!   come from ([`Features`]), Rust quiddity's recognition among them (the command line's
//!   default); the others, and [`call`], take Python quiddity's JSON or none;
//! - [`call`]: the same operations by name with JSON arguments, as the service and the MCP server
//!   receive them. Requests carry their documents and name the STEP file by path, so nothing is
//!   kept between calls. [`Service`] additionally owns the live JSON-lines session.
//!
//! An [`Error`] is `invalid` (the request cannot be met as asked; it says what would be accepted)
//! or `failed` (something went wrong; each file is published only when complete).

use serde::Serialize;

use crate::model;

/// The drawing's font (IBM Plex Mono), bundled.
pub const MONO: &[u8] = include_bytes!("../fonts/IBMPlexMono-Regular.ttf");

/// The formats [`render`] writes. DXF export is disabled until it is fixed; [`crate::output::Drawing::dxf`]
/// itself is kept, and a DXF drawing's embedded spec is still read.
pub const FORMATS: [&str; 3] = ["svg", "pdf", "png"];

/// The formats written when none are named: the drawing's vector files.
pub const DEFAULT_FORMATS: [&str; 2] = ["svg", "pdf"];

/// The PNG's resolution when none is given, pixels per inch (draftwright's).
pub use crate::png::DPI;

/// Rendered files: each suffix (`svg`, `sheet2.svg`, `pdf`, etc.) and its bytes.
pub type Files = Vec<(String, Vec<u8>)>;

/// The operations [`call`] answers, in the order they are usually used.
pub const OPERATIONS: [&str; 7] = [
    "analyse", "plan", "edit", "render", "review", "spec", "parts",
];

/// Why an operation did not complete.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Error {
    /// `invalid` or `failed`.
    pub code: &'static str,
    pub message: String,
    /// What would be accepted, where the request chose from a list.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

impl Error {
    pub fn invalid(message: impl Into<String>) -> Error {
        Error {
            code: "invalid",
            message: message.into(),
            choices: Vec::new(),
        }
    }

    pub fn failed(message: impl Into<String>) -> Error {
        Error {
            code: "failed",
            message: message.into(),
            choices: Vec::new(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if !self.choices.is_empty() {
            write!(f, " Choose one of: {}.", self.choices.join(", "))?;
        }
        Ok(())
    }
}

impl From<model::Unanalysed> for Error {
    fn from(e: model::Unanalysed) -> Error {
        if e.failed {
            Error::failed(e.message)
        } else {
            Error::invalid(e.message)
        }
    }
}

impl From<model::Invalid> for Error {
    fn from(e: model::Invalid) -> Error {
        Error {
            code: "invalid",
            message: e.message,
            choices: e.choices,
        }
    }
}

// File and service adapters have no browser implementation.
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
