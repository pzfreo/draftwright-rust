//! Observed drawing-consumer coverage (Python #1202): size evidence comes from the actual
//! rendered sheets. Recognition supplies the denominator; an unsuccessful build is unknown,
//! never supported by a capability declaration or treated as a successful empty drawing.
use serde::Serialize;

use crate::{bridge::holes, lint, model::Analysis, output::Drawing};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HoleObservation {
    pub feature: String,
    pub state: State,
    pub carriers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DrawingConsumer {
    pub supported: usize,
    pub unsupported: usize,
    pub unknown: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_error: Option<String>,
    pub holes: Vec<HoleObservation>,
}

/// Observe a build, including a failed one. The caller supplies every sheet so a hole on a
/// later sheet is judged once against the same physical inventory. No planning is performed.
pub fn drawing_consumer(analysis: &Analysis, build: Result<&[Drawing], &str>) -> DrawingConsumer {
    let (marks, build_error) = match build {
        Ok(drawings) => (
            drawings.iter().flat_map(lint::marks).collect::<Vec<_>>(),
            None,
        ),
        Err(error) => (Vec::new(), Some(error.to_string())),
    };
    let observations: Vec<_> = holes::read(analysis)
        .into_iter()
        .map(|(feature, hole)| {
            let unknown = build_error.clone().or_else(|| hole.as_ref().err().cloned());
            if let Some(reason) = unknown {
                return HoleObservation {
                    feature: feature.id.clone(),
                    state: State::Unknown,
                    carriers: Vec::new(),
                    reason: Some(reason),
                };
            }
            let hole = hole.expect("the unreadable record returned above");
            let carriers: Vec<_> = marks
                .iter()
                .filter(|m| {
                    m.claims
                        .iter()
                        .any(|c| c.feature == feature.id && c.parameter == "bore.diameter")
                        && lint::holes::states_bore(&m.text, hole.diameter)
                        && lint::holes::verified_schedule_size(analysis, m)
                })
                .map(|m| m.name.clone())
                .collect();
            let supported = !carriers.is_empty();
            HoleObservation {
                feature: feature.id.clone(),
                state: if supported {
                    State::Supported
                } else {
                    State::Unsupported
                },
                carriers,
                reason: (!supported)
                    .then(|| "No rendered annotation states this hole's bore size.".into()),
            }
        })
        .collect();
    DrawingConsumer {
        supported: observations
            .iter()
            .filter(|o| o.state == State::Supported)
            .count(),
        unsupported: observations
            .iter()
            .filter(|o| o.state == State::Unsupported)
            .count(),
        unknown: observations
            .iter()
            .filter(|o| o.state == State::Unknown)
            .count(),
        build_error,
        holes: observations,
    }
}
