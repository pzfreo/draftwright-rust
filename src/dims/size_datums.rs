//! Datums on a width, drawn on their size dimension. A datum whose faces are the two opposite
//! planes of a width (a slot, a tab, a part's overall width) is the width's median plane, not
//! either face, so lowering refuses it (`datum reference faces are not coplanar`,
//! [`crate::pmi::datum_geometry_of`]) and frames citing it could not be read
//! (`frame_datum_not_drawn`). ISO 5459 and ASME Y14.5 draw such a datum's triangle on an
//! extension line of the width's size dimension, its line in line with the dimension line; a
//! symbol on either face would make that face the datum. So once the drawing is finished, each
//! such datum a shown frame cites (every one with `sheet/uncited_datums: true`) whose size
//! dimension the drawing shows takes its triangle on one of that dimension's extension lines,
//! at its dimension line, its line running on outward, the frame in the nearest of a few tiers:
//! a place clear of every other item, other views and sheet furniture, as a datum on a hole's
//! callout takes ([`super::hole_marks::settle`]). Without a clear place, or with no size
//! dimension shown, it stays undrawn and the frame stays said. `annotations/<id>/omit` leaves it
//! out.

use std::collections::BTreeSet;

use super::{gdt, hole_marks};
use crate::items::State;
use crate::model::{Analysis, Plan, Spec};
use crate::pmi::{Category, Reading, Record};

/// The few tiers a datum symbol tries out from its extension line.
const TIERS: usize = 3;

/// The refusal a width's datum meets when lowered alone.
const NOT_COPLANAR: &str = "datum reference faces are not coplanar";

/// Each cited datum on a width drawn on its size dimension where it has a clear place.
pub(crate) fn attach(analysis: &Analysis, spec: &Spec, plan: &mut Plan) {
    let Some(Reading::Read(report)) = &analysis.pmi else {
        return;
    };
    let widths: Vec<(&Record, usize)> = report
        .records
        .iter()
        .filter(|r| r.category == Category::Datum && !r.label.trim().is_empty())
        .filter(|r| !plan.gdt.iter().any(|g| g.source == r.source_id))
        .filter_map(|r| Some((r, width(analysis, r)?)))
        .collect();
    if widths.is_empty() {
        return;
    }
    let cited: BTreeSet<String> = plan
        .gdt
        .iter()
        .filter(|g| g.kind == "control_frame" && g.state == State::Shown)
        .flat_map(|g| g.cells.iter().skip(2).cloned())
        .collect();
    let ids = report.identities();
    let exterior = super::interior::exterior_proofs(analysis, &plan.views);
    let interior = super::interior::callout_proofs(analysis, &plan.views);
    let cache = super::footprints::Cache::default();
    let mut changed = BTreeSet::new();
    for (r, k) in widths {
        let letter = r.label.trim().to_string();
        let Ok(id) = ids.id(&r.source_id) else {
            continue;
        };
        if spec.annotations.get(id).is_some_and(|a| a.omit)
            || (!cited.contains(&letter) && spec.sheet.uncited_datums != Some(true))
        {
            continue;
        }
        // Its size dimension: the shown PMI dimension between exactly its two faces.
        let feature = format!(
            "pmi:dimension.linear:f{}",
            r.faces
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        let Some(di) = plan.dimensions.iter().position(|d| {
            d.kind == "linear"
                && d.state == State::Shown
                && d.placed.as_ref().is_some_and(|p| p.rotation == 0.0)
                && d.measurements.iter().any(|m| m.feature == feature)
        }) else {
            continue;
        };
        let d = &plan.dimensions[di];
        let Some(view) = plan.views.iter().find(|v| v.id == d.view).cloned() else {
            continue;
        };
        // Invariant: the search admits only placed dimensions.
        let held = d.placed.clone().expect("placed");
        let dimension = (d.id.clone(), d.label.clone());
        let cells = vec![letter.clone()];
        let item = gdt::PlannedGdt {
            from_spec: Vec::new(),
            id: id.to_string(),
            kind: "datum_feature".into(),
            view: view.id.clone(),
            side: String::new(),
            source: r.source_id.clone(),
            faces: r.faces.clone(),
            label: crate::items::frame_label(&cells),
            cells,
            modifiers: Vec::new(),
            default: !spec.annotations.contains_key(id),
            basis: format!(
                "source-authored: the AP242 datum {letter} ({}), the median plane of the width between its two opposite planes along {}, on an extension line of its size dimension {} in the {} view, in line with its dimension line (ISO 5459: a datum on a feature of size; a symbol on either face would make that face the datum)",
                r.source_id,
                ["x", "y", "z"][k],
                dimension.1,
                view.id.trim_start_matches("v:")
            ),
            state: State::Shown,
            placed: None,
        };
        let attached = BTreeSet::from([dimension.0]);
        for (tip, side, trimmed) in ends(&held) {
            let candidates: Vec<_> = (0..TIERS)
                .map(|tier| (side, gdt::datum_at(&letter, tip, side, tier)))
                .collect();
            // Invariant: TIERS is positive.
            let first = candidates[0].1.clone();
            plan.dimensions[di].placed = Some(trimmed.clone());
            plan.gdt.push(gdt::PlannedGdt {
                side: side.into(),
                placed: Some(first),
                ..item.clone()
            });
            let gi = plan.gdt.len() - 1;
            let Some((side, next, area, _)) = hole_marks::settle(
                analysis,
                plan,
                gi,
                &view,
                candidates,
                &attached,
                (&exterior, &interior),
                &cache,
            ) else {
                plan.gdt.pop();
                plan.dimensions[di].placed = Some(held.clone());
                continue;
            };
            let g = &mut plan.gdt[gi];
            g.side = side.into();
            g.placed = Some(next);
            if trimmed != held {
                g.basis += ", its triangle in place of that end's arrowhead (no room for both)";
            }
            if let Some((vi, area)) = area {
                plan.views[vi].area = area;
            }
            // Drawn now: its refusal is no longer the drawing's.
            let refusal = format!("p:pmi_not_lowered:{}", r.source_id);
            plan.problems.retain(|p| p.id != refusal);
            changed.insert(id.to_string());
            changed.insert(plan.dimensions[di].id.clone());
            break;
        }
    }
    if changed.is_empty() {
        return;
    }
    let mut all = plan.dimensions.clone();
    all.extend(plan.gdt.iter().filter_map(gdt::proxy));
    super::callout_routes::refresh(analysis, plan, &all, &cache, &changed, &BTreeSet::new());
}

/// The identities of the datums on a width, which lowering refuses and [`attach`] may draw: the
/// Spec may name them (`annotations/<id>/omit`).
pub(crate) fn ids(analysis: &Analysis) -> Vec<String> {
    let Some(Reading::Read(report)) = &analysis.pmi else {
        return Vec::new();
    };
    let identities = report.identities();
    report
        .records
        .iter()
        .filter(|r| r.category == Category::Datum && width(analysis, r).is_some())
        .filter_map(|r| identities.id(&r.source_id).ok().map(str::to_string))
        .collect()
}

/// The principal axis of datum *r*'s width: its faces two parallel planes on that axis at
/// different places with opposite outward normals, and lowering refused it for that alone.
fn width(analysis: &Analysis, r: &Record) -> Option<usize> {
    let g = r.geometry.as_ref()?;
    if g.reasons != [NOT_COPLANAR]
        || !r
            .blockers
            .iter()
            .all(|b| crate::pmi_lowering::read_through(b))
    {
        return None;
    }
    let [a, b] = r.faces.as_slice() else {
        return None;
    };
    let (a, b) = (
        analysis.face_attachments.get(*a)?,
        analysis.face_attachments.get(*b)?,
    );
    let planar = |f: &crate::pmi::FaceAttachment| f.surface.as_deref() == Some("plane");
    let (Some(k), [sa], [sb], Some(na), Some(nb)) = (
        a.axis,
        a.support.as_slice(),
        b.support.as_slice(),
        a.normal,
        b.normal,
    ) else {
        return None;
    };
    let opposite = (0..3).map(|j| na[j] * nb[j]).sum::<f64>() < -0.999;
    (planar(a) && planar(b) && b.axis == Some(k) && opposite && (sa - sb).abs() > 1e-4).then_some(k)
}

/// The places a datum's triangle may stand on dimension *p*, each with the dimension it leaves:
/// on each extension line where the dimension line meets it, its line running on outward (the
/// side it runs toward). Inside arrowheads first, as they are; then where an arrowhead stands
/// outside its extension line (no room between them) and the text is not beyond it, the
/// triangle takes that arrowhead's place (ISO 5459), the dimension line ending at the extension
/// line.
fn ends(p: &super::Placed) -> Vec<(super::P2, &'static str, super::Placed)> {
    let [a, b] = p.arrows.as_slice() else {
        return Vec::new();
    };
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    for (n, (arrow, other)) in [(a, b), (b, a)].into_iter().enumerate() {
        let (tip, tail) = (arrow[0], arrow[1]);
        let u = [tip[0] - other[0][0], tip[1] - other[0][1]];
        let side = if u[1].abs() < 1e-6 && u[0].abs() > 1e-6 {
            if u[0] < 0.0 { "left" } else { "right" }
        } else if u[0].abs() < 1e-6 && u[1].abs() > 1e-6 {
            if u[1] < 0.0 { "below" } else { "above" }
        } else {
            continue;
        };
        let beyond = |q: super::P2| (q[0] - tip[0]) * u[0] + (q[1] - tip[1]) * u[1];
        if beyond(tail) <= 0.0 {
            inside.push((tip, side, p.clone()));
            continue;
        }
        let l = p.label_box;
        if beyond([0.5 * (l[0] + l[2]), 0.5 * (l[1] + l[3])]) > 0.0 {
            continue;
        }
        let mut trimmed = p.clone();
        trimmed.arrows.remove(n);
        // The dimension line, the one line through both tips, ends at this tip.
        for line in &mut trimmed.lines {
            let on = |q: super::P2| {
                let w = [q[0] - tip[0], q[1] - tip[1]];
                (w[0] * u[1] - w[1] * u[0]).abs() < 1e-6
            };
            if on(line[0]) && on(line[1]) {
                for q in line.iter_mut() {
                    if beyond(*q) > 0.0 {
                        *q = tip;
                    }
                }
            }
        }
        outside.push((tip, side, trimmed));
    }
    inside.extend(outside);
    inside
}

#[cfg(test)]
mod tests {
    use super::super::Placed;

    #[test]
    fn a_triangle_stands_inside_arrowheads_or_in_place_of_an_outside_one() {
        // A 15 mm width with inside arrowheads: both ends, the dimension as it is.
        let inside = Placed {
            lines: vec![
                [[0.0, -10.0], [0.0, 2.0]],
                [[15.0, -10.0], [15.0, 2.0]],
                [[0.0, 0.0], [15.0, 0.0]],
            ],
            arrows: vec![[[0.0, 0.0], [2.7, 0.0]], [[15.0, 0.0], [12.3, 0.0]]],
            label_at: [5.0, 1.0],
            rotation: 0.0,
            label_box: [5.0, 1.0, 10.0, 3.0],
        };
        let ends = super::ends(&inside);
        let sides: Vec<_> = ends.iter().map(|(tip, side, _)| (*tip, *side)).collect();
        assert_eq!(sides, [([0.0, 0.0], "left"), ([15.0, 0.0], "right")]);
        assert!(ends.iter().all(|(_, _, p)| *p == inside));

        // A narrow width with outside arrowheads and its text beyond the right one: only the
        // left end, its arrowhead gone and the dimension line ending at the extension line.
        let outside = Placed {
            lines: vec![
                [[0.0, -10.0], [0.0, 2.0]],
                [[3.0, -10.0], [3.0, 2.0]],
                [[-3.0, 0.0], [8.0, 0.0]],
            ],
            arrows: vec![[[0.0, 0.0], [-2.7, 0.0]], [[3.0, 0.0], [5.7, 0.0]]],
            label_at: [9.0, 1.0],
            rotation: 0.0,
            label_box: [9.0, 1.0, 14.0, 3.0],
        };
        let ends = super::ends(&outside);
        assert_eq!(ends.len(), 1);
        let (tip, side, trimmed) = &ends[0];
        assert_eq!((*tip, *side), ([0.0, 0.0], "left"));
        assert_eq!(trimmed.arrows, [[[3.0, 0.0], [5.7, 0.0]]]);
        assert_eq!(trimmed.lines[2], [[0.0, 0.0], [8.0, 0.0]]);
        assert_eq!(trimmed.lines[..2], outside.lines[..2]);
    }
}
