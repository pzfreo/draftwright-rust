//! Outer-profile corners (LINT-angular-profile_angle_requirement), after ISO 129-1 and ASME
//! Y14.5: a corner needs its geometry defined, not necessarily its angle printed.
//!
//! - A corner whose two edges the linear dimensions already locate (both coordinates of each
//!   oblique edge's ends, the offset of a principal edge) is defined: no angle is added. Only
//!   0°, 90° and 180° are implied.
//! - A corner the linear dimensions do not fix gets its angle, even one station short:
//!   completing it with a linear station is the `linear` profile choice, never automatic, since
//!   added stations have no lane reserved when the sheet is laid out (mfcadpp/1013 went from A4
//!   to A3 when one station was added automatically).
//!   The angle is drawn in the view that shows it true.
//! - A closed all-line profile whose corners are all issued and equal (FTC-08's octagon) gets
//!   one counted angle (`8× 135°`), never one per corner.
//! - IDs follow the notches': `d:envelope/profile_angle.angle.f<face>_e<i>_e<j>` per corner,
//!   `d:envelope/profile_angle.angle.f<face>` for a polygon's counted angle and
//!   `d:envelope/profile_location_<axis>.length.f<face>_<n>` for a completing station.
//! - Overrides: `dimensions/<id>` shows (`{}`, as a reference when linearly defined) or hides
//!   (`omit`) a corner's angle; `faces/face:<n>/profile_corners` chooses `angle` (every corner,
//!   reference where linearly defined) or `linear` (locate the missing stations instead).
//!
//! The angles are placed one at a time ([`super::angular::place_profile`]) and only where they
//! are clear: last of all, or before the callouts route when that states more corners and is
//! no worse. One with no clear
//! place stays undrawn (`angular_dimension_dropped`) and lint still reports its corner missing;
//! it never moves another item, takes a fallback or asks for more paper.
use super::{Measurement, Shape, Want, end_on, fmt, want_id};
use crate::lint::profile_angles::{Corner, requirements, undefined_stations};
use crate::model::{Analysis, Invalid, Spec};

pub const PARAMETER: &str = "profile_angle.angle";

/// How a profile's corners are defined (`faces/<face>/profile_corners`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CornerMode {
    /// No angle where the linear dimensions already fix a corner, else its angle.
    #[default]
    Auto,
    /// Every corner's angle; a reference where linear dimensions already fix it.
    Angle,
    /// Locate the missing stations with linear dimensions; no angles.
    Linear,
}

impl CornerMode {
    pub fn is_auto(&self) -> bool {
        *self == CornerMode::Auto
    }
}

/// A corner's support stamp (`f37_e124_e276`): its face and edges, as the notch ids name theirs.
fn support(c: &Corner) -> String {
    c.feature
        .strip_prefix("outer_profile:")
        .unwrap_or(&c.feature)
        .replace(':', "_")
}

/// Profile requirements belong to the part envelope, as the notches' do: `d:envelope/<role>.<support>`.
pub(crate) fn id(support: &str, role: &str) -> String {
    want_id("envelope", &format!("{role}.{support}"))
}

fn face_of(c: &Corner) -> usize {
    c.feature
        .strip_prefix("outer_profile:f")
        .and_then(|s| s.split(':').next())
        .and_then(|s| s.parse().ok())
        // Invariant: requirements() names every corner outer_profile:f<face>:e<i>:e<j>.
        .expect("outer-profile corner names its face")
}

/// The profile corner modes the spec authors, by face index.
fn modes(
    analysis: &Analysis,
    spec: &Spec,
) -> Result<std::collections::BTreeMap<usize, CornerMode>, Invalid> {
    let mut out = std::collections::BTreeMap::new();
    for (key, f) in &spec.faces {
        if f.profile_corners.is_auto() {
            continue;
        }
        for face in crate::spec_gdt::named_faces(analysis, key)? {
            out.insert(face, f.profile_corners);
        }
    }
    Ok(out)
}

/// Every station a linear dimension in *wants* reaches, rooted at the part's box and datums.
fn located(analysis: &Analysis, wants: &[Want]) -> impl Fn(usize, f64) -> bool + use<> {
    let spans: Vec<(usize, f64, f64)> = wants
        .iter()
        .filter(|w| !w.terms.first().is_some_and(|t| t.starts_with('(')))
        .filter_map(|w| match w.shape {
            Shape::Linear { a, b, axis, .. } => Some((axis, a[axis], b[axis])),
            _ => None,
        })
        .collect();
    let roots = crate::lint::location::roots(analysis);
    move |k, v| crate::lint::profile_angles::rooted_at(&roots[k], &spans, k, v)
}

fn angle_want(id: String, corners: &[&Corner], reference: bool, basis: &str) -> Want {
    let c = corners[0];
    // atan2 differs in its last bits between native and WASM libm; the plan's value must not.
    let value = (c.degrees() * 1e9).round() / 1e9;
    let mut text = format!("{}°", fmt(value));
    if corners.len() > 1 {
        text = format!("{}× {text}", corners.len());
    }
    if reference {
        text = format!("({text})");
    }
    Want {
        id,
        views: vec![end_on(c.axis)],
        value,
        terms: vec![text],
        measurements: corners
            .iter()
            .map(|c| Measurement {
                feature: c.feature.clone(),
                parameter: PARAMETER.into(),
                value,
            })
            .collect(),
        shape: Shape::Angular {
            angle: super::angular::Angle {
                vertex: c.vertex,
                first: c.first,
                second: c.second,
            },
        },
        turning: None,
        basis: basis.into(),
    }
}

/// The equal corners of a closed all-line profile when every corner is issued: one counted angle.
fn polygon<'a>(
    analysis: &Analysis,
    face: usize,
    corners: &[&'a Corner],
) -> Option<Vec<&'a Corner>> {
    let source = analysis.outer_profiles.iter().find(|s| s.face == face)?;
    let n = source.profile.supports.len();
    let lines = source
        .profile
        .supports
        .iter()
        .all(|s| matches!(s, crate::lint::profile_angles::Support::Line { .. }));
    let first = corners.first()?.degrees();
    (lines
        && n >= 3
        && corners.len() == n
        && corners
            .iter()
            .all(|c| !c.virtual_vertex && (c.degrees() - first).abs() < 1e-6))
    .then(|| corners.to_vec())
}

/// Adds the profile-corner items to *wants* (see the module's notes).
pub(super) fn apply(
    analysis: &Analysis,
    spec: &Spec,
    wants: &mut Vec<Want>,
) -> Result<(), Invalid> {
    let corners = requirements(analysis);
    if corners.is_empty() {
        return Ok(());
    }
    let modes = modes(analysis, spec)?;
    let located = located(analysis, wants);
    let shown = |id: &str| spec.dimensions.get(id).is_some_and(|d| !d.omit);
    let mut faces: Vec<usize> = corners.iter().map(face_of).collect();
    faces.dedup();
    let mut added = Vec::new();
    for face in faces {
        let mode = modes.get(&face).copied().unwrap_or_default();
        let mine: Vec<&Corner> = corners.iter().filter(|c| face_of(c) == face).collect();
        let missing: Vec<Vec<(usize, f64)>> = mine
            .iter()
            .map(|c| undefined_stations(c, &located))
            .collect();
        if mode != CornerMode::Linear
            && let Some(group) = polygon(analysis, face, &mine)
        {
            let id = id(&format!("f{face}"), PARAMETER);
            let defined = missing.iter().all(Vec::is_empty);
            if !defined || mode == CornerMode::Angle || shown(&id) {
                added.push(angle_want(
                    id,
                    &group,
                    defined,
                    "the equal corners of a closed polygonal outer profile, one counted angle in the view that shows them true",
                ));
            }
            continue;
        }
        let mut stations: Vec<(usize, f64, [f64; 3], usize)> = Vec::new();
        for (c, miss) in mine.iter().zip(&missing) {
            let id = id(&support(c), PARAMETER);
            // Automatic completion is not offered: station lanes are not reserved when the sheet
            // is laid out, so an added station could crowd a view onto larger paper.
            let linear = mode == CornerMode::Linear;
            if linear && !miss.is_empty() && !shown(&id) {
                for &(k, v) in miss {
                    if !stations.iter().any(|s| s.0 == k && (s.1 - v).abs() < 1e-4) {
                        let at = [c.vertex, c.ends[0], c.ends[1]]
                            .into_iter()
                            .find(|p| (p[k] - v).abs() < 1e-4)
                            .unwrap_or(c.vertex);
                        stations.push((k, v, at, c.axis));
                    }
                }
                continue;
            }
            let defined = miss.is_empty();
            if !defined || mode == CornerMode::Angle || shown(&id) {
                added.push(angle_want(
                    id,
                    &[c],
                    defined,
                    "a non-right convex outer-profile corner its linear dimensions do not fix, in the view that shows it true",
                ));
            }
        }
        stations.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        for (n, (k, v, at, axis)) in stations.into_iter().enumerate() {
            let mut a = at;
            a[k] = analysis.envelope[0][k];
            let value = (v - a[k]).abs();
            if value < 1e-6 {
                continue;
            }
            // Not `location.length.*`: that names hole locations, whose legibility the sheet
            // choice counts.
            let parameter = format!("profile_location_{}.length", super::AXES[k]);
            added.push(Want {
                id: id(&format!("f{face}_{n}"), &parameter),
                views: vec![end_on(axis)],
                value,
                terms: vec![fmt(value)],
                measurements: vec![Measurement {
                    feature: format!("outer_profile:f{face}:s{n}"),
                    parameter,
                    value,
                }],
                basis: "an outer-profile corner station located from the part's minimum datum, completing the corner linearly (the profile's `linear` choice)".into(),
                shape: Shape::Linear {
                    a,
                    b: at,
                    axis: k,
                    a_edge: true,
                    b_edge: false,
                    order: 1,
                },
                turning: None,
            });
        }
    }
    wants.extend(added);
    Ok(())
}

/// Whether *w* is a profile-corner angle, placed last and dropped when it cannot be clear.
pub(super) fn is_profile_angle(w: &Want) -> bool {
    matches!(w.shape, Shape::Angular { .. })
        && w.measurements.iter().any(|m| m.parameter == PARAMETER)
}
