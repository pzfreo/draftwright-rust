//! Structural checks of the drawn sheet (draftwright's `lint_drawing`, `linting/structural.py`):
//! what the sheet looks like, not what it says about the part. They read the rendered drawing
//! alone — its page, its text and what it records of the views and things drawn
//! ([`Drawing::sheet`]: each view's visible box, each annotation's and each piece of furniture's
//! ink and label boxes) — and never the part, the Plan or the Spec.
//!
//! Ported, in Python's order and with its thresholds and severities:
//! - `derived_view_identifier_reused` (`_lint_derived_view_identifiers`): one section or detail
//!   letter naming two views, read from the views' names, never their captions. Error.
//! - `scale_not_stated` (`_lint_scale_stated`): a finished sheet (one with a title block) with no
//!   scale note. Error.
//! - `annotation_overlap` (`_lint_annotation_pairs`, its label part): two labels whose text
//!   boxes overlap by more than 0.5 mm in both directions. A thing with no label (the title
//!   block, the notes block) takes part by its whole ink, as in Python. Warning.
//! - `annotation_out_of_bounds` (`_lint_annotation_bounds`): any ink past the drawable area,
//!   the title block allowed its border's half stroke. Error, once per side overshot.
//! - `view_overlap` and `view_out_of_bounds` (`_lint_view_shapes`, its box part): two views'
//!   boxes overlapping (warning), a view's box past the drawable area (error, once per side).
//!
//! Not ported here: `label_vs_measured` (the drawing does not record a dimension's measured
//! path or its scale), the ink, leader, centreline and view-annotation checks, and the sheet-level
//! `nominal_rounded` and `missing_principal_dimension` notes.
//!
//! Differences from Python, each judged in `tests/lint_structural.rs`:
//! - `annotation_overlap` gives no ink-crossing remedy (draftwright adds one when the pair's
//!   line-work also crosses, `annotation_ink_overlap`'s measure, not ported), and leaves out a
//!   section's hatch and cutting-plane ink, which draftwright compares by each arrowhead's,
//!   wing's and hatch's whole box (`Kind::SectionInk`); the chain line itself is a centreline,
//!   left out by Python too.
//! - `annotation_out_of_bounds` judges a section's hatch and cutting-plane ink as one thing,
//!   where draftwright judges each arrowhead, wing and the hatch on its own.
//! - The messages name the port's annotations and leave out draftwright's API advice
//!   (`label_offset_x`, `margin`), which the port's spec does not have.

use std::collections::BTreeMap;

use super::{LintIssue, Severity};
use crate::output::{Drawing, Drawn, Kind};

/// Below this a box past a bound is rounding (draftwright's `BOUNDS_ROUNDOFF`).
const BOUNDS_ROUNDOFF: f64 = 1e-9;
/// Half the title block's border stroke (draftwright's `_TB_LINE_WIDTH / 2`): the border may
/// straddle the drawable area's edge.
const TITLE_HALF_STROKE: f64 = 0.075;
/// Two labels overlap when they share more than this, mm, in both directions.
const OVERLAP: f64 = 0.5;

fn issue(id: String, code: &str, severity: Severity, message: String) -> LintIssue {
    LintIssue {
        id,
        code: code.into(),
        severity,
        message,
        at: None,
        view: None,
        subjects: Vec::new(),
        requirements: Vec::new(),
        measurements: Vec::new(),
        sources: Vec::new(),
        stage: None,
        annotation: None,
        related: Vec::new(),
        evidence: None,
        fix: None,
    }
}

/// Every structural finding on *drawing*, in draftwright's order.
pub fn check(drawing: &Drawing) -> Vec<LintIssue> {
    let mut issues = derived_view_identifiers(drawing);
    issues.extend(scale_stated(drawing));
    issues.extend(annotation_overlap(drawing));
    issues.extend(annotation_bounds(drawing));
    issues.extend(view_boxes(drawing));
    issues
}

/// The semantic identifier a derived view's name encodes (draftwright's
/// `derived_view_identifier`): `section_aa` is section A (the slug doubled), `detail_b` detail B;
/// none for any other name, or a slug that is not alphanumeric.
pub fn derived_view_identifier(name: &str) -> Option<String> {
    let name = name.strip_prefix("v:").unwrap_or(name);
    let identifier = if let Some(slug) = name.strip_prefix("detail_") {
        slug
    } else {
        let slug = name.strip_prefix("section_")?;
        let n = slug.chars().count();
        let half: String = slug.chars().take(n / 2).collect();
        if n % 2 == 1 || slug.chars().skip(n / 2).ne(half.chars()) {
            return None;
        }
        &slug[..half.len()]
    };
    (!identifier.is_empty() && identifier.chars().all(char::is_alphanumeric))
        .then(|| identifier.to_uppercase())
}

/// `derived_view_identifier_reused`: one identifier naming more than one drawn view.
fn derived_view_identifiers(drawing: &Drawing) -> Vec<LintIssue> {
    let mut by_identifier: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for v in &drawing.sheet.views {
        if let Some(id) = derived_view_identifier(&v.name) {
            by_identifier.entry(id).or_default().push(&v.name);
        }
    }
    by_identifier
        .into_iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|(id, names)| {
            let mut i = issue(
                format!("l:derived_view_identifier_reused:{id}"),
                "derived_view_identifier_reused",
                Severity::Error,
                format!(
                    "derived-view identifier '{id}' names multiple views: {}",
                    names
                        .iter()
                        .map(|n| format!("'{}'", bare(n)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
            i.related = names.iter().map(|n| n.to_string()).collect();
            i
        })
        .collect()
}

/// `scale_not_stated`: a finished sheet (it has a title block) states its scale. A drawing with
/// neither is a fragment, with nothing to state a scale about.
fn scale_stated(drawing: &Drawing) -> Option<LintIssue> {
    let has = |k: Kind| drawing.sheet.items.iter().any(|d| d.kind == k);
    if has(Kind::ScaleNote) || !has(Kind::TitleBlock) {
        return None;
    }
    let mut i = issue(
        "l:scale_not_stated:sheet".into(),
        "scale_not_stated",
        Severity::Error,
        "The sheet does not state its scale. ISO 7200 presents the scale outside the title \
         block; draftwright draws it beside the projection symbol. A drawing whose scale is not \
         stated cannot be measured off."
            .into(),
    );
    i.fix = Some("draw the SCALE note beside the title block".into());
    Some(i)
}

/// What a message calls *d*: a dimension by its label's text, anything else by its name.
fn called(drawing: &Drawing, d: &Drawn) -> String {
    if d.kind == Kind::Dimension
        && let Some(a) = drawing.annotations.iter().find(|a| a.name == d.name)
    {
        let text: Vec<&str> = a
            .runs
            .iter()
            .filter_map(|&i| drawing.text.get(i))
            .map(|r| r.text.as_str())
            .collect();
        if !text.is_empty() {
            return text.join(" ");
        }
    }
    d.name.clone()
}

/// `annotation_overlap`: each pair of labels, in drawing order, whose text boxes share more than
/// 0.5 mm both ways. A thing without a label takes part by its whole ink; a section's hatch and
/// cutting-plane ink take no part (see the module's note).
fn annotation_overlap(drawing: &Drawing) -> Vec<LintIssue> {
    let boxes: Vec<(&Drawn, [f64; 4])> = drawing
        .sheet
        .items
        .iter()
        .filter(|d| d.kind != Kind::SectionInk)
        .map(|d| (d, d.label.unwrap_or(d.ink)))
        .collect();
    let mut issues = Vec::new();
    for (k, (a, la)) in boxes.iter().enumerate() {
        for (b, lb) in &boxes[k + 1..] {
            let ox = (la[2].min(lb[2]) - la[0].max(lb[0])).max(0.0);
            let oy = (la[3].min(lb[3]) - la[1].max(lb[1])).max(0.0);
            if ox > OVERLAP && oy > OVERLAP {
                let mut i = issue(
                    format!("l:annotation_overlap:{}+{}", a.name, b.name),
                    "annotation_overlap",
                    Severity::Warning,
                    format!(
                        "labels '{}' and '{}' overlap by {ox:.1}×{oy:.1} mm",
                        called(drawing, a),
                        called(drawing, b)
                    ),
                );
                i.annotation = Some(a.name.clone());
                i.related = vec![b.name.clone()];
                i.fix = Some("move one of the labels clear of the other".into());
                issues.push(i);
            }
        }
    }
    issues
}

/// The sides on which *b* runs past *bounds*, and by how much, as draftwright words them.
fn overshoots(b: [f64; 4], bounds: [f64; 4]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if b[0] < bounds[0] - BOUNDS_ROUNDOFF {
        out.push(("left", format!("left by {:.1} mm", bounds[0] - b[0])));
    }
    if b[2] > bounds[2] + BOUNDS_ROUNDOFF {
        out.push(("right", format!("right by {:.1} mm", b[2] - bounds[2])));
    }
    if b[1] < bounds[1] - BOUNDS_ROUNDOFF {
        out.push(("below", format!("below by {:.1} mm", bounds[1] - b[1])));
    }
    if b[3] > bounds[3] + BOUNDS_ROUNDOFF {
        out.push(("above", format!("above by {:.1} mm", b[3] - bounds[3])));
    }
    out
}

/// `annotation_out_of_bounds`: every thing's whole ink inside the drawable area; the title block
/// may reach half its border's stroke past it.
fn annotation_bounds(drawing: &Drawing) -> Vec<LintIssue> {
    let Some(area) = drawing.sheet.drawable else {
        return Vec::new();
    };
    let mut issues = Vec::new();
    for d in &drawing.sheet.items {
        let bounds = if d.kind == Kind::TitleBlock {
            let h = TITLE_HALF_STROKE;
            [area[0] - h, area[1] - h, area[2] + h, area[3] + h]
        } else {
            area
        };
        for (side, detail) in overshoots(d.ink, bounds) {
            let mut i = issue(
                format!("l:annotation_out_of_bounds:{}:{side}", d.name),
                "annotation_out_of_bounds",
                Severity::Error,
                format!(
                    "annotation '{}' extends past drawable area ({detail})",
                    called(drawing, d)
                ),
            );
            i.annotation = Some(d.name.clone());
            i.fix = Some(
                "move it inside the border, or choose a larger sheet or a smaller scale".into(),
            );
            issues.push(i);
        }
    }
    issues
}

/// A view's name as draftwright writes it: `front`, not `v:front`.
fn bare(name: &str) -> &str {
    name.strip_prefix("v:").unwrap_or(name)
}

/// `view_overlap` (two views' boxes overlapping, a touch not counted) and `view_out_of_bounds`
/// (a view's box past the drawable area, once per side).
fn view_boxes(drawing: &Drawing) -> Vec<LintIssue> {
    let views = &drawing.sheet.views;
    let mut issues = Vec::new();
    for (k, a) in views.iter().enumerate() {
        let [ax0, ay0, ax1, ay1] = a.bbox;
        for b in &views[k + 1..] {
            let [bx0, by0, bx1, by1] = b.bbox;
            if ax0 < bx1 && ax1 > bx0 && ay0 < by1 && ay1 > by0 {
                let mut i = issue(
                    format!("l:view_overlap:{}+{}", a.name, b.name),
                    "view_overlap",
                    Severity::Warning,
                    format!(
                        "view '{}' bbox [x={ax0:.1}–{ax1:.1}, y={ay0:.1}–{ay1:.1}] overlaps view \
                         '{}' [x={bx0:.1}–{bx1:.1}, y={by0:.1}–{by1:.1}] — increase spacing \
                         between views",
                        bare(&a.name),
                        bare(&b.name)
                    ),
                );
                i.view = Some(a.name.clone());
                i.related = vec![b.name.clone()];
                issues.push(i);
            }
        }
    }
    if let Some(area) = drawing.sheet.drawable {
        for v in views {
            for (side, detail) in overshoots(v.bbox, area) {
                let mut i = issue(
                    format!("l:view_out_of_bounds:{}:{side}", v.name),
                    "view_out_of_bounds",
                    Severity::Error,
                    format!(
                        "view '{}' extends past drawable area ({detail}) — reduce the view scale \
                         or move the view",
                        bare(&v.name)
                    ),
                );
                i.view = Some(v.name.clone());
                issues.push(i);
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_view_identifiers_are_read_as_draftwright_reads_them() {
        assert_eq!(
            derived_view_identifier("v:section_aa").as_deref(),
            Some("A")
        );
        assert_eq!(
            derived_view_identifier("section_b1b1").as_deref(),
            Some("B1")
        );
        assert_eq!(derived_view_identifier("detail_c").as_deref(), Some("C"));
        assert_eq!(derived_view_identifier("section_ab"), None);
        assert_eq!(derived_view_identifier("section_a"), None);
        assert_eq!(derived_view_identifier("detail_"), None);
        assert_eq!(derived_view_identifier("detail_a-b"), None);
        assert_eq!(derived_view_identifier("front"), None);
    }
}
