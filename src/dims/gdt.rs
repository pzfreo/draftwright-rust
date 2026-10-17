//! Feature control frames, datum feature symbols and labels: the part's AP242 PMI as
//! [`crate::pmi_lowering`] lowers it, placed beside its view and drawn (draftwright's
//! `annotations/_gdt.py` `render_gdt`, and `build123d_drafting`'s `FeatureControlFrame`,
//! `DatumFeature`, `TextBlock` and `Leader`).
//!
//! - **A feature control frame** is draftwright's glyph at the dimensions' text height (3 mm):
//!   cells two text heights tall, the characteristic's symbol drawn in strokes in the first, the
//!   zone's `⌀` (a ring and a stroke) or `S⌀`, the magnitude and the material condition's
//!   ringed letter in the second, one cell per datum letter after. Its leader runs straight from
//!   its site into its strip, with an arrowhead at the site, a shelf of 2 mm, and the frame hung
//!   from the shelf's end (draftwright's `Leader` with a callout), the all-around circle or the
//!   all-over double circle at the bend.
//! - **A datum feature symbol** is drawn as ISO 5459 and ASME Y14.5 draw it: a filled triangle
//!   whose base sits on the datum's surface at its site, a line along the outward normal to a
//!   square frame two text heights wide holding the letter. draftwright hangs its `DatumFeature`
//!   glyph (triangle, line and frame together) off an arrowed leader's shelf instead, so its
//!   triangle sits beside the frame in mid-air, not on the surface.
//! - **A surface texture symbol** (a face's AP242 surface texture) is ISO 1302's graphical
//!   symbol, as the default finish beside the title block draws it, standing on its leader's
//!   shelf, its parameters (`Ra 0.8`) under the bar: its leader runs from a proven site on its
//!   face's edge line along the face's outward normal, as a datum's does, with an arrowhead at
//!   the site (draftwright's `Finish`, a `SurfaceFinish` glyph on a `Leader`, which only
//!   draftwright's face-finish lowering for one external cylinder makes).
//! - **A label** (a surface or common label: `A`, `F1.1`, `2X R`) is its text at the
//!   dimensions' text height, unframed, hung from a leader as a frame is (draftwright's `Note`,
//!   a `TextBlock` callout on a `Leader`).
//!
//! **Placement** (additive to the dimensions' and leaders'): the layout reserves each item's
//! lowering strip ([`demand`], [`crate::layout::Strips::reserve_gdt`]). Automatic datums first
//! try compact tiers between their face and the first dimension lane, retaining the triangle,
//! shaft, frame and normal clearances. Only after these choices are exhausted do they try the
//! existing tiers beyond the dimensions. Cylinder sites are proved on either trimmed-face
//! silhouette; planar symbols may stand on a drawn face extension beyond either end of the
//! view outline. Sites nearest the original are tried first. Finishes keep their outward side;
//! frames and labels can retry the other strips (`_gdt_retry_sides`).
//!
//! All glyphs share the same obstacle grade: clearance from labels, views, sheet furniture and
//! reserved-area boundaries, followed by the count of leader crossings with dimension lines,
//! extension lines, arrow shafts and other leaders (the collision reporter's `lines_meet`,
//! including collinear travel). Only the all-zero grade ends the search immediately. Within
//! equal clearances, fewer crossings win. A complete arrangement is accepted only if it does
//! not introduce a new annotation contact against the original arrangement (unique unordered
//! annotation pairs, independent of reporting direction): later frames
//! must not be displaced into overlaps to clear an earlier leader. Unavoidable collisions are
//! retained and reported (Policy B). Explicit placement intents retain their strip/tier rules.
//! If the full rearrangement introduces contact, the datum changes are also tried alone.
//! Pins and canonical hand placements take precedence over automatic compact placement.
//!
//! Each item's text layer is its compartments as the plan states them (`⌖`, `⌀0.05Ⓜ`, `A`, …),
//! one invisible run over each, so the PDF and SVG can be searched and lint reads it. The ink of
//! the symbols is strokes, so the font need hold only the digits, letters and `ø` (checked by
//! `tests/pmi_lowering.rs`); a label's text is drawn as the file states it, a character the font
//! lacks as its missing-glyph box (as draftwright's FreeType draws it).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::placement::{self, Frame, Page, boxes_overlap, crosses};
use super::{ARROW, ARROW_HALF, FONT, P2, Placed, PlannedDimension, TEXT, font};
use crate::items::{Op, State};
use crate::model::{Analysis, PlannedSheet, PlannedView, Problem, Spec};
use crate::output::{Annotation, Curve, HAlign, Item, TextRun, VAlign};
use crate::pmi_lowering::{Glyph, Lowered};
use crate::text::Segment;

/// A feature control frame or datum feature symbol the sheet will carry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedGdt {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub from_spec: Vec<String>,
    /// `pmi:<identity>`, the record's semantic identity ([`crate::pmi::identity`]), never its
    /// Part 21 instance (that is `source`).
    pub id: String,
    /// `control_frame`, `datum_feature` or `label`.
    pub kind: String,
    pub view: String,
    /// Its strip: `above`, `below`, `left` or `right` of the view.
    pub side: String,
    /// The PMI record it draws (`Record::source_id`).
    pub source: String,
    /// The drawing's faces it applies to.
    pub faces: Vec<usize>,
    /// What it states, compartment by compartment, as its text runs: the characteristic's
    /// symbol, the zone, magnitude and material condition, then each datum letter; or a datum's
    /// letter.
    pub cells: Vec<String>,
    /// `all_around` or `all_over`, drawn at the leader's bend.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<String>,
    /// What it states as one line ([`crate::items::frame_label`]).
    pub label: String,
    pub default: bool,
    pub basis: String,
    /// What became of it: shown, or why not ([`crate::items::State`]).
    pub state: crate::items::State,
    /// Its ink on the page; absent when it cannot be drawn, and a problem says why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<GdtPlaced>,
}

/// A frame's or symbol's ink, mm on the page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GdtPlaced {
    /// Where it attaches: the leader's arrowhead or the triangle's base.
    pub tip: P2,
    /// Its leader's lines: a frame's shaft and shelf, a symbol's line to its frame.
    pub lines: Vec<[P2; 2]>,
    /// Arrowheads, `[tip, tail]`.
    pub arrows: Vec<[P2; 2]>,
    /// The frame's box.
    pub label_box: [f64; 4],
    /// The glyph's straight strokes: the frame's rules and dividers, the symbols' lines.
    pub strokes: Vec<[P2; 2]>,
    /// Its circles and arcs, `[cx, cy, r, from, to]` (degrees anticlockwise).
    pub arcs: Vec<[f64; 5]>,
    /// Filled outlines: a datum's triangle.
    pub fills: Vec<Vec<P2>>,
    /// Letters and numbers drawn from the font, each centred on its point.
    pub letters: Vec<GdtText>,
    /// The text layer: each compartment's run, centred on its cell.
    pub runs: Vec<GdtText>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GdtText {
    pub text: String,
    pub at: P2,
    pub size: f64,
}

/// draftwright's draft: the leader's shelf (`pad_around_text`), the frame's metrics
/// (`_gdt_metrics`: cell padding, the zone sign's and the material ring's radii), and the
/// datum symbol's triangle and frame (`DatumFeature`).
const H: f64 = 2.0 * TEXT;
const SHELF: f64 = 2.0;
const PAD: f64 = 0.6 * TEXT;
const ZONE_R: f64 = 0.42 * TEXT;
const MATERIAL_R: f64 = 0.62 * TEXT;
const TRIANGLE: f64 = 1.4 * TEXT;
const RING: f64 = 0.7 * ARROW;
/// The clearance beyond the dimensions, and between tiers.
const CLEAR: f64 = crate::layout::GDT_CLEAR;
const TIERS: usize = 16;

/// A glyph laid out at its own origin, before it is placed.
#[derive(Default)]
struct Laid {
    strokes: Vec<[P2; 2]>,
    arcs: Vec<[f64; 5]>,
    letters: Vec<GdtText>,
    runs: Vec<GdtText>,
    size: P2,
}

fn text(s: &str, at: P2, size: f64) -> GdtText {
    GdtText {
        text: s.into(),
        at,
        size,
    }
}

fn circle(c: P2, r: f64) -> [f64; 5] {
    [c[0], c[1], r, 0.0, 360.0]
}

/// The ISO 1101 symbol of draftwright's characteristic, as the text layer states it.
fn symbol(characteristic: &str) -> &'static str {
    match characteristic {
        "straightness" => "⏤",
        "flatness" => "⏥",
        "circularity" => "○",
        "cylindricity" => "⌭",
        "profile_line" => "⌒",
        "profile_surface" => "⌓",
        "angularity" => "∠",
        "perpendicularity" => "⟂",
        "parallelism" => "∥",
        "position" => "⌖",
        "concentricity" | "coaxiality" => "◎",
        "symmetry" => "⌯",
        "circular_runout" => "↗",
        _ => "⌰",
    }
}

/// The characteristic's strokes about *c* (`_characteristic_edges`, s = 0.42 h). Coaxiality,
/// which draftwright's frame does not know, is drawn as concentricity's symbol, which ISO 1101
/// gives both.
fn characteristic(name: &str, c: P2, laid: &mut Laid) {
    let s = 0.42 * TEXT;
    let p = |x: f64, y: f64| [c[0] + x * s, c[1] + y * s];
    let mut st: Vec<[P2; 2]> = Vec::new();
    let mut arcs: Vec<[f64; 5]> = Vec::new();
    let barbs = |tip: P2, back: P2, size: f64| -> [[P2; 2]; 2] {
        let a = (back[1] - tip[1]).atan2(back[0] - tip[0]);
        [20f64, -20.0].map(|d| {
            let t = a + d.to_radians();
            [tip, [tip[0] + size * t.cos(), tip[1] + size * t.sin()]]
        })
    };
    match name {
        "straightness" => st.push([p(-1.0, 0.0), p(1.0, 0.0)]),
        "flatness" => {
            let q = [p(-1.0, -0.5), p(0.3, -0.5), p(1.0, 0.5), p(-0.3, 0.5)];
            for i in 0..4 {
                st.push([q[i], q[(i + 1) % 4]]);
            }
        }
        "circularity" => arcs.push(circle(c, s)),
        "cylindricity" => {
            arcs.push(circle(c, 0.55 * s));
            st.push([p(-1.0, -0.6), p(-0.55, 0.9)]);
            st.push([p(0.55, -0.9), p(1.0, 0.6)]);
        }
        "profile_line" => arcs.push([c[0], c[1], s, 0.0, 180.0]),
        "profile_surface" => {
            arcs.push([c[0], c[1], s, 0.0, 180.0]);
            st.push([p(-1.0, 0.0), p(1.0, 0.0)]);
        }
        "angularity" => {
            st.push([p(-1.0, -1.0), p(1.0, 1.0)]);
            st.push([p(-1.0, -1.0), p(1.0, -1.0)]);
        }
        "perpendicularity" => {
            st.push([p(0.0, -1.0), p(0.0, 1.0)]);
            st.push([p(-1.0, -1.0), p(1.0, -1.0)]);
        }
        "parallelism" => {
            st.push([p(-0.7, -1.0), p(0.1, 1.0)]);
            st.push([p(-0.1, -1.0), p(0.7, 1.0)]);
        }
        "position" => {
            arcs.push(circle(c, 0.6 * s));
            st.push([p(-1.0, 0.0), p(1.0, 0.0)]);
            st.push([p(0.0, -1.0), p(0.0, 1.0)]);
        }
        "concentricity" | "coaxiality" => {
            arcs.push(circle(c, s));
            arcs.push(circle(c, 0.5 * s));
        }
        "symmetry" => {
            st.push([p(0.0, -1.0), p(0.0, 1.0)]);
            st.push([p(-0.7, 0.45), p(0.7, 0.45)]);
            st.push([p(-0.7, -0.45), p(0.7, -0.45)]);
        }
        "circular_runout" => {
            st.push([p(-1.0, -1.0), p(1.0, 1.0)]);
            st.extend(barbs(p(1.0, 1.0), p(-1.0, -1.0), 0.5 * s));
        }
        _ => {
            for dx in [-0.25, 0.35] {
                let (a, b) = (p(-1.0 + dx, -1.0), p(0.4 + dx, 1.0));
                st.push([a, b]);
                st.extend(barbs(b, a, 0.45 * s));
            }
        }
    }
    laid.strokes.extend(st);
    laid.arcs.extend(arcs);
}

/// The width of *s*'s ink at *size* in the labels' font.
fn width(s: &str, size: f64) -> f64 {
    let l = font().text(s, size);
    l.max[0] - l.min[0]
}

/// A feature control frame laid out from its lower left corner (`FeatureControlFrame`).
fn control_frame(glyph: &Glyph) -> (Laid, Vec<String>) {
    let Glyph::ControlFrame {
        characteristic: name,
        tolerance,
        zone,
        material,
        datums,
        ..
    } = glyph
    else {
        unreachable!("a control frame")
    };
    let mut laid = Laid::default();
    let cy = H / 2.0;
    let spherical = *zone == Some("spherical_diameter");
    let diameter = *zone == Some("diameter");
    // `⌀` is drawn as a ring and a stroke; `S⌀` as the font's `Sø` before the value.
    let ink = if spherical {
        format!("Sø{tolerance}")
    } else {
        tolerance.clone()
    };
    let tw = width(&ink, TEXT);
    let w_tol = PAD
        + tw
        + PAD
        + if diameter { 2.0 * ZONE_R + PAD } else { 0.0 }
        + if material.is_some() {
            2.0 * MATERIAL_R + PAD
        } else {
            0.0
        };
    let mut xs = vec![0.0, H, H + w_tol];
    for _ in datums {
        xs.push(xs.last().expect("a cell") + H);
    }
    let w = *xs.last().expect("a cell");
    let corners = [[0.0, 0.0], [w, 0.0], [w, H], [0.0, H]];
    for i in 0..4 {
        laid.strokes.push([corners[i], corners[(i + 1) % 4]]);
    }
    for x in &xs[1..xs.len() - 1] {
        laid.strokes.push([[*x, 0.0], [*x, H]]);
    }
    characteristic(name, [H / 2.0, cy], &mut laid);
    let mut cells = vec![symbol(name).to_string()];
    laid.runs.push(text(symbol(name), [H / 2.0, cy], TEXT));
    let mut x = H + PAD;
    if diameter {
        let c = [x + ZONE_R, cy];
        laid.arcs.push(circle(c, ZONE_R));
        laid.strokes.push([
            [c[0] + 0.9 * ZONE_R, cy - 0.9 * ZONE_R],
            [c[0] - 0.9 * ZONE_R, cy + 0.9 * ZONE_R],
        ]);
        x = c[0] + ZONE_R + PAD;
    }
    laid.letters.push(text(&ink, [x + tw / 2.0, cy], TEXT));
    x += tw + PAD;
    if let Some(m) = material {
        let c = [x + MATERIAL_R, cy];
        laid.arcs.push(circle(c, MATERIAL_R));
        laid.letters.push(text(m, c, 0.8 * TEXT));
    }
    let run = format!(
        "{}{tolerance}{}",
        match zone {
            Some("diameter") => "⌀",
            Some(_) => "S⌀",
            None => "",
        },
        match material {
            Some("M") => "Ⓜ",
            Some(_) => "Ⓛ",
            None => "",
        }
    );
    laid.runs.push(text(&run, [H + w_tol / 2.0, cy], TEXT));
    cells.push(run);
    for (i, d) in datums.iter().enumerate() {
        let c = [xs[2 + i] + H / 2.0, cy];
        laid.letters.push(text(d, c, TEXT));
        laid.runs.push(text(d, c, TEXT));
        cells.push(d.clone());
    }
    laid.size = [w, H];
    (laid, cells)
}

/// A datum's frame laid out about its centre.
fn datum_frame(letter: &str) -> Laid {
    let h = TEXT;
    let corners = [[-h, -h], [h, -h], [h, h], [-h, h]];
    let mut laid = Laid::default();
    for i in 0..4 {
        laid.strokes.push([corners[i], corners[(i + 1) % 4]]);
    }
    laid.letters.push(text(letter, [0.0, 0.0], h));
    laid.runs.push(text(letter, [0.0, 0.0], h));
    laid.size = [H, H];
    laid
}

/// A label's text laid out from its lower left corner, a text height tall.
fn label(s: &str) -> Laid {
    let w = width(s, TEXT).max(0.5 * TEXT);
    let mut laid = Laid::default();
    laid.letters.push(text(s, [w / 2.0, TEXT / 2.0], TEXT));
    laid.runs.push(text(s, [w / 2.0, TEXT / 2.0], TEXT));
    laid.size = [w, TEXT];
    laid
}

/// A face's ISO 1302 surface texture symbol laid out from its lower left corner, which is the
/// left end of the reference line it stands on (its leader's shelf): the short and long legs at
/// 60° to a tip on that line (1.4 and 3 text heights tall, as the default finish beside the title
/// block draws them, [`crate::title_block::default_finish`]), the material removal condition's
/// mark ([`crate::title_block::removal_marks`]), the bar from the long leg's top, and the
/// parameters under it, one per line (ISO 1302's positions a and b). The reference line itself
/// is the leader's ([`candidate`]), as long as the glyph is wide.
fn finish_symbol(lines: &[String], removal: &str) -> Laid {
    let cot = 1.0 / 3f64.sqrt();
    let gap = 0.3 * TEXT;
    let short = 1.4 * TEXT;
    let long = (3.0 * TEXT).max(lines.len() as f64 * (TEXT + gap));
    let tip = [gap + short * cot, 0.0];
    let at = |p: P2| [p[0] + tip[0], p[1] + tip[1]];
    let long_top = at([long * cot, long]);
    let tw = lines
        .iter()
        .map(|l| width(l, TEXT))
        .fold(0.5 * TEXT, f64::max);
    let left = long_top[0] + gap;
    let end = left + tw + gap;
    let mut laid = Laid {
        strokes: vec![
            [at([-short * cot, short]), tip],
            [tip, long_top],
            [long_top, [end, long]],
        ],
        ..Laid::default()
    };
    let (marks, circles) = crate::title_block::removal_marks(removal, short);
    laid.strokes
        .extend(marks.iter().map(|[a, b]| [at(*a), at(*b)]));
    laid.arcs.extend(circles.iter().map(|c| {
        let [x, y] = at([c[0], c[1]]);
        [x, y, c[2], c[3], c[4]]
    }));
    for (i, l) in lines.iter().enumerate() {
        let c = [
            left + tw / 2.0,
            long - gap - TEXT / 2.0 - i as f64 * (TEXT + gap),
        ];
        laid.letters.push(text(l, c, TEXT));
        laid.runs.push(text(l, c, TEXT));
    }
    laid.size = [end, long];
    laid
}

/// How an item hangs from its site: a feature control frame or label from its leader's shelf,
/// a datum feature symbol's triangle on its surface, or a surface texture symbol standing on its
/// leader's shelf.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Frame,
    Datum,
    Finish,
}

impl Shape {
    fn of(glyph: &Glyph) -> Shape {
        match glyph {
            Glyph::Datum { .. } => Shape::Datum,
            Glyph::Finish { .. } => Shape::Finish,
            Glyph::ControlFrame { .. } | Glyph::Label { .. } => Shape::Frame,
        }
    }
}

/// The outward direction of strip *side*.
fn outward(side: &str) -> P2 {
    match side {
        "below" => [0.0, -1.0],
        "above" => [0.0, 1.0],
        "left" => [-1.0, 0.0],
        _ => [1.0, 0.0],
    }
}

fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// An item as placed at strip coordinate *pos* (along its strip's outward axis): its leader,
/// frame box and the laid glyph's offset.
struct Candidate {
    lines: Vec<[P2; 2]>,
    arrows: Vec<[P2; 2]>,
    fills: Vec<Vec<P2>>,
    label_box: [f64; 4],
    /// Where the glyph's origin lands.
    origin: P2,
    elbow: P2,
}

fn candidate(shape: Shape, tip: P2, side: &str, pos: f64, size: P2) -> Candidate {
    let d = outward(side);
    let horizontal = d[0] == 0.0;
    if shape == Shape::Finish {
        // The symbol stands on its shelf, which runs from the leader's end under it: above or
        // below the view the shelf is the box's bottom edge (*pos* the box's middle), the
        // leader rising or falling to its left end; beside the view the leader runs straight
        // out to *pos* and on under the symbol, which stands on it.
        let (elbow, x0) = if horizontal {
            ([tip[0], pos - size[1] / 2.0], tip[0])
        } else if d[0] > 0.0 {
            ([pos, tip[1]], pos)
        } else {
            ([pos, tip[1]], pos - size[0])
        };
        let shelf = if elbow[0] > x0 {
            [x0, elbow[1]]
        } else {
            [x0 + size[0], elbow[1]]
        };
        return Candidate {
            lines: vec![[tip, elbow], [elbow, shelf]],
            arrows: vec![placement::arrow(tip, elbow)],
            fills: Vec::new(),
            label_box: [x0, elbow[1], x0 + size[0], elbow[1] + size[1]],
            origin: [x0, elbow[1]],
            elbow,
        };
    }
    if shape == Shape::Datum {
        let centre = if horizontal {
            [tip[0], pos]
        } else {
            [pos, tip[1]]
        };
        let n = [-d[1], d[0]];
        let half = 0.5 * TRIANGLE;
        let apex = [
            tip[0] + d[0] * 0.9 * TRIANGLE,
            tip[1] + d[1] * 0.9 * TRIANGLE,
        ];
        let near = [centre[0] - d[0] * TEXT, centre[1] - d[1] * TEXT];
        return Candidate {
            lines: vec![[apex, near]],
            arrows: Vec::new(),
            fills: vec![vec![
                [tip[0] + n[0] * half, tip[1] + n[1] * half],
                apex,
                [tip[0] - n[0] * half, tip[1] - n[1] * half],
            ]],
            label_box: [
                centre[0] - TEXT,
                centre[1] - TEXT,
                centre[0] + TEXT,
                centre[1] + TEXT,
            ],
            origin: centre,
            elbow: near,
        };
    }
    let elbow = if horizontal {
        [tip[0], pos]
    } else {
        [pos, tip[1]]
    };
    // The shelf runs on in the leader's direction, to the right of a vertical one.
    let dir = if elbow[0] >= tip[0] { 1.0 } else { -1.0 };
    let end = [elbow[0] + dir * SHELF, elbow[1]];
    let x0 = if dir > 0.0 { end[0] } else { end[0] - size[0] };
    let y0 = elbow[1] - size[1] / 2.0;
    Candidate {
        lines: vec![[tip, elbow], [elbow, end]],
        arrows: vec![placement::arrow(tip, elbow)],
        fills: Vec::new(),
        label_box: [x0, y0, x0 + size[0], y0 + size[1]],
        origin: [x0, y0],
        elbow,
    }
}

/// An item's extent across strip *side* from its site, page mm: a frame's or label's leader,
/// shelf and box above or below its view, its box centred on the leader left or right; a datum's
/// frame centred on its line.
fn span(shape: Shape, side: &str, size: P2) -> [f64; 2] {
    if shape == Shape::Datum {
        [-TEXT, TEXT]
    } else if shape == Shape::Finish {
        // From its leader: rightwards above or below the view, upwards beside it.
        if outward(side)[0] == 0.0 {
            [0.0, size[0]]
        } else {
            [0.0, size[1]]
        }
    } else if outward(side)[0] == 0.0 {
        [0.0, SHELF + size[0]]
    } else {
        [-size[1] / 2.0, size[1] / 2.0]
    }
}

/// The strip coordinate of an item's first tier in *side* of the view whose outline is *o*:
/// beyond the outline and every piece of that view's dimensions (*ink*: lines, arrows and label
/// boxes as their corners) on that side that lies across from it, with room for the leader's
/// arrowhead or the datum's triangle. A dimension elsewhere along the strip does not push it
/// out (draftwright's lateral tier reuse, `_adjust_strip_candidate_labels`): its tier is free
/// there.
fn first_tier(shape: Shape, side: &str, tip: P2, o: [f64; 4], ink: &[[P2; 2]], size: P2) -> f64 {
    let d = outward(side);
    let horizontal = d[0] == 0.0;
    let edge = match side {
        "below" => -o[1],
        "above" => o[3],
        "left" => -o[0],
        _ => o[2],
    };
    let across = |p: P2| if horizontal { p[0] } else { p[1] };
    let [a, b] = span(shape, side, size).map(|v| across(tip) + v);
    let base = ink
        .iter()
        .filter(|[p, q]| {
            let (lo, hi) = (across(*p).min(across(*q)), across(*p).max(across(*q)));
            lo < b + CLEAR && hi > a - CLEAR
        })
        .flat_map(|s| s.iter().map(|q| dot(*q, d)))
        .filter(|v| *v > edge)
        .fold(edge, f64::max);
    let along = dot(tip, d);
    // A surface texture symbol's box starts at its leader's end beside the view, as a frame's
    // would without its shelf, so the same clearances hold.
    let (from_base, from_tip) = if shape == Shape::Datum {
        (CLEAR + TEXT, 0.9 * TRIANGLE + 0.8 * TEXT + TEXT)
    } else if horizontal {
        (CLEAR + size[1] / 2.0, ARROW + 1.0 + size[1] / 2.0)
    } else {
        (CLEAR, ARROW + 1.0)
    };
    let s = (base + from_base).max(along + from_tip);
    // Back from the outward axis to the page coordinate.
    if d[0] + d[1] < 0.0 { -s } else { s }
}

/// The step between tiers along the strip.
fn step(shape: Shape, side: &str, size: P2) -> f64 {
    let horizontal = outward(side)[0] == 0.0;
    if shape == Shape::Datum || horizontal {
        size[1] + CLEAR
    } else if shape == Shape::Finish {
        size[0] + CLEAR
    } else {
        size[0] + SHELF + CLEAR
    }
}

/// An attachment site, its outward side, and an optional planar face extension.
type Site<'a> = (P2, &'a str, Option<[P2; 2]>);
/// Clearance class, followed by line crossings; winning candidates have exact counts.
type Grade = (u8, usize);
type Choice<'a> = (Grade, &'a str, Candidate, P2, usize);

/// Compact datum tiers, starting with the normal triangle, shaft and frame clearances.
/// Exact obstacle grading rejects any intervening dimension, extension or text. The outer
/// edge must precede the first stroke across the frame's strip, with the usual clearance.
fn inside_tiers(side: &str, tip: P2, outline: [f64; 4], size: P2, ink: &[[P2; 2]]) -> Vec<f64> {
    let d = outward(side);
    let sign = d[0] + d[1];
    let first = first_tier(Shape::Datum, side, tip, outline, &[], size) * sign;
    let across = usize::from(d[0] != 0.0);
    let nearest = ink
        .iter()
        .filter(|[a, b]| {
            a[across].min(b[across]) < tip[across] + TEXT + CLEAR
                && a[across].max(b[across]) > tip[across] - TEXT - CLEAR
        })
        .flat_map(|l| l.iter().map(|p| dot(*p, d)))
        .filter(|v| *v > dot(tip, d) + 1e-6)
        .fold(f64::INFINITY, f64::min);
    (0..TIERS)
        .map(|k| first + k as f64 * step(Shape::Datum, side, size))
        .take_while(|p| *p + TEXT + CLEAR <= nearest)
        .map(|p| p * sign)
        .collect()
}

/// Both silhouettes are usable only where the reader proved points on the trimmed face.
/// Keep the original site first, then try the nearest sites in page space.
fn cylinder_sites<'a>(
    analysis: &Analysis,
    item: &'a Lowered,
    frame: &Frame,
    requested: Option<&str>,
) -> Vec<Site<'a>> {
    let original = frame.page(item.site);
    let mut sites = Vec::new();
    for face in &item.faces {
        let Some(a) = analysis.face_attachments.get(*face) else {
            continue;
        };
        let Some(axis) = a.exact_axis else {
            continue;
        };
        if a.support.len() != 2 {
            continue;
        }
        let mut centre = item.site;
        for (i, value) in (0..3).filter(|i| *i != axis).zip(&a.support) {
            centre[i] = *value;
        }
        let centre = frame.page(centre);
        for p in std::iter::once(item.site)
            .chain(item.alternatives.iter().copied())
            .chain(a.sites.iter().copied())
        {
            let tip = frame.page(p);
            let side = if outward(item.side)[0] != 0.0 {
                if tip[0] > centre[0] { "right" } else { "left" }
            } else if tip[1] > centre[1] {
                "above"
            } else {
                "below"
            };
            if requested.is_none_or(|s| s == side) {
                sites.push((tip, side, None));
            }
        }
    }
    if sites.is_empty() {
        return vec![(original, item.side, None)];
    }
    sites.sort_by(|a, b| {
        let distance = |p: P2| (p[0] - original[0]).hypot(p[1] - original[1]);
        distance(a.0).total_cmp(&distance(b.0))
    });
    sites.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    sites
}

/// A planar face seen edge-on continues as a straight extension beyond either end of
/// the view. Leave a 1 mm outline gap and carry the line beyond the triangle's full base.
fn plane_extensions<'a>(analysis: &Analysis, item: &'a Lowered, frame: &Frame) -> Vec<Site<'a>> {
    let tip = frame.page(item.site);
    let k = usize::from(outward(item.side)[0] != 0.0);
    let outline = frame.outline();
    let mut bounds = [f64::INFINITY, f64::NEG_INFINITY];
    for f in &item.faces {
        let Some(b) = analysis.face_attachments.get(*f).and_then(|a| a.bounds) else {
            continue;
        };
        for x in [b[0], b[3]] {
            for y in [b[1], b[4]] {
                for z in [b[2], b[5]] {
                    let p = frame.page([x, y, z]);
                    bounds[0] = bounds[0].min(p[k]);
                    bounds[1] = bounds[1].max(p[k]);
                }
            }
        }
    }
    if !bounds.iter().all(|v| v.is_finite()) {
        return Vec::new();
    }
    let mut sites = Vec::new();
    for (end, sign) in [(0, -1.0), (1, 1.0)] {
        // Starting 1 mm past the face must also clear the part's projected envelope.
        if (bounds[end] - outline[k + 2 * end]).abs() > 1.0 {
            continue;
        }
        for n in 0..TIERS {
            let mut at = tip;
            at[k] = outline[k + 2 * end] + sign * (CLEAR + TRIANGLE / 2.0 + n as f64 * (H + CLEAR));
            let mut start = tip;
            start[k] = bounds[end] + sign;
            let mut stop = at;
            stop[k] += sign * (TRIANGLE / 2.0 + CLEAR);
            sites.push((at, item.side, Some([start, stop])));
        }
    }
    sites.sort_by(|a, b| (a.0[k] - tip[k]).abs().total_cmp(&(b.0[k] - tip[k]).abs()));
    sites
}

/// What a candidate must stay clear of.
struct Obstacles<'a> {
    page: &'a Page<'a>,
    own: String,
    /// The area reserved for its view and bands.
    area: Option<[f64; 4]>,
    labels: Vec<[f64; 4]>,
    lines: Vec<[P2; 2]>,
}

impl Obstacles<'_> {
    /// How well *c* fits, best first: 0 clear of everything and inside its view's reserved area;
    /// 1 clear of everything, past the area; 2 on the sheet and off every label and frame, a line
    /// of it through a label or another view; 3 off every label and frame only; 4 neither.
    /// Add the crossing count as the second component: (0, 0) alone is clear, and fewer
    /// crossings win among candidates with otherwise equal clearances.
    fn grade(&self, c: &Candidate, best: Option<Grade>) -> Grade {
        let clearance = self.clearance_grade(c);
        // Clearance precedes crossings. A worse clearance, or a crossing count that
        // already ties the best at the same clearance, cannot replace that candidate.
        // Only losing candidates stop early; every saved best has its exact count.
        if best.is_some_and(|b| clearance > b.0) {
            return (clearance, 0);
        }
        let limit = best.filter(|b| b.0 == clearance).map(|b| b.1);
        if limit == Some(0) {
            return (clearance, 0);
        }
        let triangle: Vec<_> = c
            .fills
            .iter()
            .flat_map(|f| (0..f.len()).map(|i| [f[i], f[(i + 1) % f.len()]]))
            .collect();
        let mut crossings = 0;
        for s in c.lines.iter().chain(&triangle) {
            for t in &self.lines {
                if super::leaders::lines_meet(*s, *t) {
                    crossings += 1;
                    if limit.is_some_and(|n| crossings >= n) {
                        return (clearance, crossings);
                    }
                }
            }
        }
        (clearance, crossings)
    }

    fn clearance_grade(&self, c: &Candidate) -> u8 {
        let b = c.label_box;
        let d = self.page.drawable();
        let on_sheet = b[0] >= d[0] && b[1] >= d[1] && b[2] <= d[2] && b[3] <= d[3];
        let grown = [b[0] - 0.5, b[1] - 0.5, b[2] + 0.5, b[3] + 0.5];
        let off_labels = self.labels.iter().all(|l| !boxes_overlap(grown, *l));
        if !off_labels {
            return 4;
        }
        let furniture = on_sheet
            && !boxes_overlap(b, self.page.title_block())
            && self.page.views().iter().all(|(_, v)| !boxes_overlap(b, *v));
        if !furniture {
            return 3;
        }
        let lines_clear =
            self.page.views().iter().all(|(id, v)| {
                *id == self.own.as_str() || !c.lines.iter().any(|l| crosses(*l, *v))
            }) && !c.lines.iter().any(|l| crosses(*l, self.page.title_block()))
                && !self.lines.iter().any(|l| crosses(*l, b))
                && !c
                    .lines
                    .iter()
                    .any(|l| self.labels.iter().any(|x| crosses(*l, *x)));
        if !lines_clear {
            return 2;
        }
        let inside = |q: &P2, a: [f64; 4]| {
            q[0] >= a[0] - 1e-6 && q[0] <= a[2] + 1e-6 && q[1] >= a[1] - 1e-6 && q[1] <= a[3] + 1e-6
        };
        let within = self.area.is_none_or(|a| {
            c.lines.iter().flatten().all(|q| inside(q, a))
                && [[b[0], b[1]], [b[2], b[3]]].iter().all(|q| inside(q, a))
        });
        if within { 0 } else { 1 }
    }
}

/// The strips a feature control frame tries after its own (`_gdt_retry_sides`).
fn retries(side: &str) -> [&'static str; 3] {
    match side {
        "above" => ["below", "right", "left"],
        "below" => ["above", "right", "left"],
        "left" => ["right", "above", "below"],
        _ => ["left", "above", "below"],
    }
}

/// The principal views' towards-the-viewer and up directions, third angle (`model::VIEWS`).
const PRINCIPALS: [(&str, [f64; 3], [f64; 3]); 3] = [
    ("front", [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
    ("plan", [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ("side", [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
];

/// The part's lowered PMI as the layout reserves room for it ([`crate::layout::Gdt`],
/// [`crate::layout::Strips::reserve_gdt`]), *shown* saying which principal views the sheet
/// carries: each item's strip as lowering chose it, its site across the strip, and its glyph's
/// extents as [`plan`] places it (the leader and shelf of a frame or label, a datum's frame
/// centred on its line), with the room [`first_tier`] leaves for the arrowhead or triangle.
pub fn demand(analysis: &Analysis, shown: &dyn Fn(&str) -> bool) -> Vec<crate::layout::Gdt> {
    demand_with_spec(analysis, &Spec::default(), shown)
}

pub(crate) fn demand_with_spec(
    analysis: &Analysis,
    spec: &Spec,
    shown: &dyn Fn(&str) -> bool,
) -> Vec<crate::layout::Gdt> {
    let (mut items, _) = crate::pmi_lowering::lower(analysis, shown);
    crate::label_text::annotations(&mut items, spec).expect("validated annotation wording");
    let frames: BTreeMap<&str, Frame> = PRINCIPALS
        .iter()
        .map(|(name, toward, up)| (*name, Frame::looking(analysis, *toward, *up, [0.0; 2], 1.0)))
        .collect();
    items
        .iter()
        .map(|item| {
            let shape = Shape::of(&item.glyph);
            let datum = shape == Shape::Datum;
            let size = match &item.glyph {
                Glyph::Datum { .. } => [H, H],
                Glyph::Label { text } => label(text).size,
                Glyph::Finish { lines, removal, .. } => finish_symbol(lines, removal).size,
                g => control_frame(g).0.size,
            };
            let horizontal = outward(item.side)[0] == 0.0;
            let f = &frames[item.view];
            let p = f.page(item.site);
            let o = f.outline();
            let depth = if datum {
                H
            } else if horizontal {
                size[1]
            } else if shape == Shape::Finish {
                size[0]
            } else {
                SHELF + size[0]
            };
            crate::layout::Gdt {
                view: item.view,
                side: item.side,
                at: if horizontal { p[0] } else { p[1] },
                span: span(shape, item.side, size),
                ends: if horizontal {
                    [o[0], o[2]]
                } else {
                    [o[1], o[3]]
                },
                depth,
                lead: if datum {
                    0.9 * TRIANGLE + 0.8 * TEXT
                } else {
                    ARROW + 1.0
                },
            }
        })
        .collect()
}

/// The part's PMI lowered and placed around *views* on *sheet*, after the dimensions *dims*:
/// the items, and the problems met (lowering's, and what the placed items collide with). An
/// item whose view *spec* omits is fixed by restoring it.
pub fn plan(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    dims: &[PlannedDimension],
) -> (Vec<PlannedGdt>, Vec<Problem>) {
    plan_anchored(
        analysis,
        spec,
        sheet,
        views,
        dims,
        None,
        &super::footprints::Cache::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_anchored(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    dims: &[PlannedDimension],
    canonical: Option<&crate::model::Plan>,
    cache: &super::footprints::Cache,
) -> (Vec<PlannedGdt>, Vec<Problem>) {
    // A complete existing layout is also a candidate. Greedy improvements must not buy
    // fewer crossings by displacing later frames into text, other views or unreserved space.
    // Compare unique unordered pairs, since leader crossings can be reported both ways.
    let seed = plan_search(
        analysis,
        spec,
        sheet,
        views,
        dims,
        canonical,
        cache,
        Search::Seed,
        None,
    )
    .expect("the seed search is never pruned");
    let contacts = |problems: &[Problem]| {
        problems
            .iter()
            .filter(|p| p.code.starts_with("annotation_"))
            .map(|p| {
                let mut pair = p.subjects.clone();
                pair.sort_unstable();
                (p.code.clone(), pair)
            })
            .collect::<std::collections::BTreeSet<_>>()
    };
    let before = contacts(&seed.1);
    // Without datums, only the added dimension arrow shafts can change the datum-only
    // pass's clearance grading. If every seed label clears them, the same first-best
    // choice survives at each step; the pass would reproduce the seed.
    let datum_search_can_change = seed.0.iter().any(|g| g.kind == "datum_feature")
        || dims.iter().filter_map(|d| d.placed.as_ref()).any(|d| {
            d.arrows.iter().any(|a| {
                seed.0
                    .iter()
                    .filter_map(|g| g.placed.as_ref())
                    .any(|g| crosses(*a, g.label_box))
            })
        });
    if seed.0.iter().all(|g| g.placed.is_none()) {
        return seed;
    }
    for search in [Search::All, Search::Datums] {
        if search == Search::Datums && !datum_search_can_change {
            continue;
        }
        if let Some(compact) = plan_search(
            analysis,
            spec,
            sheet,
            views,
            dims,
            canonical,
            cache,
            search,
            Some(&before),
        ) && contacts(&compact.1).is_subset(&before)
        {
            return compact;
        }
    }
    seed
}

#[derive(Clone, Copy, PartialEq)]
enum Search {
    Seed,
    Datums,
    All,
}

#[allow(clippy::too_many_arguments)]
fn plan_search(
    analysis: &Analysis,
    spec: &Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    dims: &[PlannedDimension],
    canonical: Option<&crate::model::Plan>,
    cache: &super::footprints::Cache,
    search: Search,
    allowed: Option<&BTreeSet<(String, Vec<String>)>>,
) -> Option<(Vec<PlannedGdt>, Vec<Problem>)> {
    let compact = search != Search::Seed;
    let recess_frames = dims.iter().any(|d| {
        d.measurements
            .iter()
            .any(|m| m.feature.starts_with("f:section_recess:"))
    });
    let shown = |name: &str| views.iter().any(|v| v.id == format!("v:{name}"));
    let (mut items, mut problems) = crate::pmi_lowering::lower(analysis, &shown);
    crate::label_text::annotations(&mut items, spec).expect("validated annotation wording");
    if items.is_empty() {
        return Some((Vec::new(), problems));
    }
    let frames: BTreeMap<&str, Frame> = views
        .iter()
        .filter_map(|v| {
            let name = v.id.strip_prefix("v:")?;
            ["front", "plan", "side"]
                .contains(&name)
                .then(|| (name, Frame::new(analysis, v)))
        })
        .collect();
    let page = Page::new(analysis, sheet, views);
    let on_sheet = |view: &str| {
        views
            .iter()
            .find(|v| v.id == view)
            .map_or(sheet.number == 1, |v| v.sheet == sheet.number)
    };
    for item in &mut items {
        if let Some(p) = spec
            .annotations
            .get(&item.id)
            .and_then(|a| a.place.as_ref())
        {
            if let Some(v) = p.view.as_deref() {
                item.view = match v {
                    "plan" => "plan",
                    "side" => "side",
                    _ => "front",
                };
            }
            if let Some(side) = p.side {
                item.side = side.name();
            } else if matches!(item.glyph, Glyph::Datum { .. } | Glyph::Finish { .. })
                && let Some(crate::pmi::Reading::Read(report)) = &analysis.pmi
                && let Some(record) = report.records.iter().find(|r| r.source_id == item.source)
                && let Some((_, sides)) = placement_choices(item, record, views)
                    .into_iter()
                    .find(|(view, _)| *view == item.view)
            {
                item.side = sides[0];
            }
        }
    }
    // Each site's position across its strip.
    let across = |i: &Lowered| -> f64 {
        frames.get(i.view).map_or(0.0, |f| {
            let p = f.page(i.site);
            if outward(i.side)[0] == 0.0 {
                p[0]
            } else {
                p[1]
            }
        })
    };
    // A frame above or below its view runs right of its leader, so the rightmost leader takes
    // the nearest tier and no shaft runs through a frame placed before it. Leaders that stand
    // together keep the reading's order (its sources, as text), as before items were named by
    // their identity.
    items.sort_by(|a, b| {
        let order = if outward(a.side)[0] == 0.0 {
            across(b).total_cmp(&across(a))
        } else {
            across(a).total_cmp(&across(b))
        };
        (
            !spec
                .annotations
                .get(&a.id)
                .and_then(|a| a.place.as_ref())
                .is_some_and(|p| p.pinned),
            a.view,
            a.side,
        )
            .cmp(&(
                !spec
                    .annotations
                    .get(&b.id)
                    .and_then(|a| a.place.as_ref())
                    .is_some_and(|p| p.pinned),
                b.view,
                b.side,
            ))
            .then(order)
            .then(a.source.cmp(&b.source))
    });
    let mut labels: Vec<[f64; 4]> = dims
        .iter()
        .filter_map(|d| d.placed.as_ref().map(|p| p.label_box))
        .collect();
    let mut lines: Vec<[P2; 2]> = dims
        .iter()
        .filter_map(|d| d.placed.as_ref())
        .flat_map(|p| {
            p.lines
                .iter()
                .chain(p.arrows.iter().filter(|_| compact))
                .copied()
        })
        .collect();
    let mut out = Vec::new();
    for item in items {
        let shape = Shape::of(&item.glyph);
        let datum = shape == Shape::Datum;
        let (laid, cells) = match &item.glyph {
            Glyph::Datum { letter, .. } => (datum_frame(letter), vec![letter.clone()]),
            Glyph::Label { text } => (label(text), vec![text.clone()]),
            Glyph::Finish { lines, removal, .. } => (finish_symbol(lines, removal), lines.clone()),
            g => control_frame(g),
        };
        let modifiers: Vec<String> = match &item.glyph {
            Glyph::ControlFrame {
                all_around,
                all_over,
                ..
            } => [("all_around", *all_around), ("all_over", *all_over)]
                .into_iter()
                .filter(|(_, on)| *on)
                .map(|(m, _)| m.to_string())
                .collect(),
            Glyph::Datum { .. } | Glyph::Label { .. } | Glyph::Finish { .. } => Vec::new(),
        };
        let mut planned = PlannedGdt {
            from_spec: if spec
                .annotations
                .get(&item.id)
                .is_some_and(|a| a.text.is_some())
            {
                vec!["text".into()]
            } else {
                Vec::new()
            },
            id: item.id.clone(),
            kind: match item.glyph {
                Glyph::Datum { .. } => "datum_feature",
                Glyph::Label { .. } => "label",
                Glyph::ControlFrame { .. } => "control_frame",
                Glyph::Finish { .. } => "surface_finish",
            }
            .into(),
            view: format!("v:{}", item.view),
            side: item.side.into(),
            source: item.source.clone(),
            faces: item.faces.clone(),
            label: crate::items::frame_label(&cells),
            cells,
            modifiers,
            // A datum or control the Spec authors is never a default ([`crate::spec_gdt`]).
            default: !spec.annotations.contains_key(&item.id)
                && !item.source.starts_with(crate::spec_gdt::SOURCE_PREFIX),
            basis: item.basis.clone(),
            // Shown once placed; else its view is not drawn.
            state: State::not_drawable("pmi_not_drawn"),
            placed: None,
        };
        if spec.annotations.get(&item.id).is_some_and(|a| a.omit) {
            planned.state = State::OmittedByUser;
            planned.basis += "; omitted by the spec";
            out.push(planned);
            continue;
        }
        let intent = spec
            .annotations
            .get(&item.id)
            .and_then(|a| a.place.as_ref());
        let authored = intent.is_some_and(|p| {
            p.pinned
                || p.side.is_some()
                || p.lane.is_some()
                || p.along.is_some()
                || p.offset.is_some()
        });
        if intent.is_some() {
            planned.basis += "; placed relative to its anchor by the spec";
        }
        let Some(frame) = frames.get(item.view) else {
            problems.push(Problem {
                id: format!("p:pmi_not_drawn:{}", item.id),
                code: "pmi_not_drawn".into(),
                severity: "warning".into(),
                message: format!(
                    "{} is drawn in the {} view, which is not on the sheet; it stays in the plan undrawn. Restore the view (remove views/{}/omit).",
                    crate::wording::capital(&crate::wording::item(
                        &item.id,
                        "",
                        &planned.cells.join(" ")
                    )),
                    item.view,
                    item.view
                ),
                subjects: vec![item.id.clone()],
                fix: if spec.views.get(item.view).is_some_and(|v| v.omit) {
                    vec![Op::remove(format!("views/{}/omit", item.view))]
                } else {
                    Vec::new()
                },
                ..Default::default()
            });
            out.push(planned);
            continue;
        };
        if intent.is_some_and(crate::placement_intent::automatic_pin)
            && let Some(base) = canonical
            && let Some(g) = base
                .gdt
                .iter()
                .find(|g| g.id == item.id && g.view == planned.view)
            && let Some(delta) = crate::placement_intent::anchor_delta(base, views, &g.view)
            && let Some(mut p) = g.placed.clone()
        {
            use crate::placement_intent::{move_box, move_point};
            move_point(&mut p.tip, delta);
            move_box(&mut p.label_box, delta);
            for line in p
                .lines
                .iter_mut()
                .chain(&mut p.arrows)
                .chain(&mut p.strokes)
            {
                for q in line {
                    move_point(q, delta);
                }
            }
            for arc in &mut p.arcs {
                if delta != [0.0, 0.0] {
                    arc[0] += delta[0];
                    arc[1] += delta[1];
                }
            }
            for fill in &mut p.fills {
                for q in fill {
                    move_point(q, delta);
                }
            }
            for t in p.letters.iter_mut().chain(&mut p.runs) {
                move_point(&mut t.at, delta);
            }
            labels.push(p.label_box);
            lines.extend(p.lines.iter().copied());
            planned.side = g.side.clone();
            planned.state = State::Shown;
            planned.placed = Some(p);
            out.push(planned);
            continue;
        }
        let own = format!("v:{}", item.view);
        let ink: Vec<[P2; 2]> = dims
            .iter()
            .filter(|d| d.view == own)
            .filter_map(|d| d.placed.as_ref())
            .flat_map(|p| {
                let b = p.label_box;
                p.lines
                    .iter()
                    .chain(&p.arrows)
                    .copied()
                    .chain([[[b[0], b[1]], [b[2], b[3]]]])
                    .collect::<Vec<_>>()
            })
            .collect();
        let obstacles = Obstacles {
            page: &page,
            own: own.clone(),
            area: page.area(item.view),
            labels: labels.clone(),
            lines: lines.clone(),
        };
        let sides: Vec<&str> = if shape != Shape::Frame || intent.is_some_and(|p| p.side.is_some())
        {
            vec![item.side]
        } else {
            std::iter::once(item.side)
                .chain(retries(item.side))
                .collect()
        };
        let mut sites: Vec<Site<'_>> = std::iter::once(item.site)
            .chain(item.alternatives.iter().copied())
            .map(|s| (frame.page(s), item.side, None))
            .collect();
        if compact && let Glyph::Datum { surface, .. } = &item.glyph {
            if surface == "cylinder" {
                sites = cylinder_sites(analysis, &item, frame, authored.then_some(item.side));
            } else if !authored {
                sites.extend(plane_extensions(analysis, &item, frame));
            }
        }
        let tiers = |side: &str, tip: P2| {
            let pinned = intent.is_some_and(|p| p.pinned);
            let first = first_tier(
                shape,
                side,
                tip,
                frame.outline(),
                if pinned { &[] } else { &ink },
                laid.size,
            );
            let d = outward(side);
            let sign = if d[0] + d[1] < 0.0 { -1.0 } else { 1.0 };
            let step = step(shape, side, laid.size);
            let lane = intent.and_then(|p| p.lane).unwrap_or(0);
            let count = if intent.is_some_and(|p| p.pinned) {
                1
            } else {
                TIERS
            };
            (0..count)
                .map(move |k| first + sign * (k + lane) as f64 * step)
                .collect::<Vec<_>>()
        };
        // The best grade any place has, the first place of that grade: by site, then strip,
        // then tier.
        let mut best: Option<Choice<'_>> = None;
        // Exhaust the compact, face-to-first-lane choices before trying any outer tier.
        'search: for inside in [true, false] {
            if inside
                && (!compact || !(datum || recess_frames && shape == Shape::Frame) || authored)
            {
                continue;
            }
            for (n, (tip, site_side, extension)) in sites.iter().enumerate() {
                let choices = if datum {
                    vec![*site_side]
                } else {
                    sides.clone()
                };
                for side in choices {
                    let positions = if inside && datum {
                        inside_tiers(side, *tip, frame.outline(), laid.size, &ink)
                    } else if inside {
                        // Frame ink can fit between the face and the first
                        // dimension lane too. Exact obstacle grading, including
                        // every label and witness, still decides whether it fits.
                        let d = outward(side);
                        let sign = d[0] + d[1];
                        let first = first_tier(shape, side, *tip, frame.outline(), &[], laid.size);
                        let outer =
                            first_tier(shape, side, *tip, frame.outline(), &ink, laid.size) * sign;
                        (0..TIERS)
                            .map(|k| first + sign * k as f64 * step(shape, side, laid.size))
                            .take_while(|p| p * sign < outer - 1e-6)
                            .collect()
                    } else {
                        tiers(side, *tip)
                    };
                    for pos in positions {
                        let mut anchor = *tip;
                        if let Some(along) = intent.and_then(|p| p.along) {
                            let o = frame.outline();
                            let k = usize::from(outward(side)[0] != 0.0);
                            anchor[k] = o[k] + along.fraction() * (o[k + 2] - o[k]);
                        }
                        let mut c = candidate(shape, anchor, side, pos, laid.size);
                        if datum && intent.and_then(|p| p.along).is_some() {
                            for fill in &mut c.fills {
                                for q in fill {
                                    q[0] += tip[0] - anchor[0];
                                    q[1] += tip[1] - anchor[1];
                                }
                            }
                        }
                        if intent.and_then(|p| p.along).is_some() {
                            if let Some(line) = c.lines.first_mut() {
                                line[0] = if datum {
                                    [
                                        line[0][0] + tip[0] - anchor[0],
                                        line[0][1] + tip[1] - anchor[1],
                                    ]
                                } else {
                                    *tip
                                };
                            }
                            for a in &mut c.arrows {
                                *a = placement::arrow(*tip, c.elbow);
                            }
                        }
                        if let Some(line) = extension {
                            c.lines.push(*line);
                        }
                        let g = if intent.is_some_and(|p| p.pinned) {
                            (0, 0)
                        } else if search == Search::All || (compact && datum) {
                            obstacles.grade(&c, best.as_ref().map(|b| b.0))
                        } else {
                            (obstacles.clearance_grade(&c), 0)
                        };
                        if best.as_ref().is_none_or(|b| g < b.0) {
                            best = Some((g, side, c, *tip, n));
                            if g == (0, 0) {
                                break 'search;
                            }
                        }
                    }
                }
            }
        }
        let (_, side, chosen, tip, alternative) = best.expect("a tier");
        if sites[alternative].2.is_some() {
            planned.basis += "; on an extension of its planar face beyond the view outline";
        } else if alternative > 0 {
            planned.basis +=
                "; at another proven site on its face, as the first has no clear place";
        }
        if side != item.side {
            planned.basis += &format!(
                "; its {} strip has no clear place, so it stands {side} the view",
                item.side
            );
            planned.side = side.into();
        }
        let shift = |p: P2| [p[0] + chosen.origin[0], p[1] + chosen.origin[1]];
        let moved = |t: &GdtText| GdtText {
            text: t.text.clone(),
            at: shift(t.at),
            size: t.size,
        };
        let mut arcs: Vec<[f64; 5]> = laid
            .arcs
            .iter()
            .map(|a| {
                let c = shift([a[0], a[1]]);
                [c[0], c[1], a[2], a[3], a[4]]
            })
            .collect();
        for m in &planned.modifiers {
            arcs.push(circle(chosen.elbow, RING));
            if m == "all_over" {
                arcs.push(circle(chosen.elbow, 1.7 * RING));
            }
        }
        labels.push(chosen.label_box);
        lines.extend(chosen.lines.iter().copied());
        planned.state = State::Shown;
        planned.placed = Some(GdtPlaced {
            tip,
            lines: chosen.lines,
            arrows: chosen.arrows,
            label_box: chosen.label_box,
            strokes: laid
                .strokes
                .iter()
                .map(|[a, b]| [shift(*a), shift(*b)])
                .collect(),
            arcs,
            fills: chosen.fills,
            letters: laid.letters.iter().map(moved).collect(),
            runs: laid.runs.iter().map(moved).collect(),
        });
        // This contact cannot be undone by placing later items. Stop a search that the
        // complete-arrangement guard would reject, before searching the remaining items.
        if let Some(allowed) = allowed.filter(|_| on_sheet(&planned.view)) {
            let p = planned.placed.as_ref().expect("just placed");
            for (id, lines, arrows) in dims
                .iter()
                .filter(|d| on_sheet(&d.view))
                .filter_map(|d| d.placed.as_ref().map(|p| (&d.id, &p.lines, &p.arrows)))
                .chain(
                    out.iter()
                        .filter(|g: &&PlannedGdt| on_sheet(&g.view))
                        .filter_map(|g| g.placed.as_ref().map(|p| (&g.id, &p.lines, &p.arrows))),
                )
            {
                if *id == planned.id {
                    continue;
                }
                let mut pair = vec![planned.id.clone(), id.clone()];
                pair.sort_unstable();
                if !allowed.contains(&("annotation_leader_crossing".into(), pair))
                    && p.lines.iter().any(|s| {
                        lines
                            .iter()
                            .chain(arrows)
                            .any(|t| super::leaders::lines_meet(*s, *t))
                    })
                {
                    return None;
                }
            }
        }
        out.push(planned);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    // What the placed items collide with, as the dimensions' collisions are judged; the
    // dimensions' own are reported by them.
    let mut all: Vec<PlannedDimension> = dims.to_vec();
    all.extend(out.iter().filter_map(proxy));
    problems.extend(
        placement::collisions_cached(analysis, &all, sheet, views, cache)
            .into_iter()
            .filter(|p| p.subjects.iter().any(|s| s.starts_with("pmi:"))),
    );
    Some((out, problems))
}

/// A placed item as the dimensions' collision checks see it: a callout whose leader is its
/// lines and whose label is its frame.
pub fn proxy(g: &PlannedGdt) -> Option<PlannedDimension> {
    let p = g.placed.as_ref()?;
    Some(PlannedDimension {
        reference: false,
        determined_by: Vec::new(),
        region: None,
        id: g.id.clone(),
        kind: "callout".into(),
        view: g.view.clone(),
        value: 0.0,
        label: g.cells.join(" "),
        terms: g.cells.clone(),
        measurements: Vec::new(),
        default: true,
        from_spec: Vec::new(),
        basis: g.basis.clone(),
        state: State::Shown,
        placed: Some(Placed {
            lines: p.lines.clone(),
            arrows: p.arrows.clone(),
            label_at: [p.label_box[0], p.label_box[1]],
            rotation: 0.0,
            label_box: p.label_box,
        }),
    })
}

/// The default surface finish's symbol beside the title block (*r*) as the placed items'
/// collision checks and the blocks placed after it see it: its strokes as lines, its whole box
/// as its label.
pub fn finish_proxy(r: &crate::pmi_lowering::PlannedRequirement) -> Option<PlannedDimension> {
    let p = r.placed.as_ref()?;
    Some(PlannedDimension {
        reference: false,
        determined_by: Vec::new(),
        region: None,
        id: r.id.clone(),
        kind: "symbol".into(),
        view: String::new(),
        value: 0.0,
        label: r.text.clone(),
        terms: vec![r.text.clone()],
        measurements: Vec::new(),
        default: true,
        from_spec: Vec::new(),
        basis: r.basis.clone(),
        state: State::Shown,
        placed: Some(Placed {
            lines: p.strokes.clone(),
            arrows: Vec::new(),
            label_at: [p.glyph[0], p.glyph[1]],
            rotation: 0.0,
            label_box: p.glyph,
        }),
    })
}

/// What the default surface finish's symbol (*r*) collides with among the placed *items* (the
/// dimensions, frames and labels), judged as their collisions are.
pub fn finish_collisions(
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    items: &[PlannedDimension],
    r: &crate::pmi_lowering::PlannedRequirement,
) -> Vec<Problem> {
    finish_collisions_cached(
        analysis,
        sheet,
        views,
        items,
        r,
        &super::footprints::Cache::default(),
    )
}

pub(crate) fn finish_collisions_cached(
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    items: &[PlannedDimension],
    r: &crate::pmi_lowering::PlannedRequirement,
    cache: &super::footprints::Cache,
) -> Vec<Problem> {
    if sheet.number != 1 {
        return Vec::new();
    }
    let Some(proxy) = finish_proxy(r) else {
        return Vec::new();
    };
    let mut all = items.to_vec();
    all.push(proxy);
    // It stands in the band the title block's reservation keeps above the grid, as the scale
    // note does, so the reservation is not an obstacle to it.
    placement::collisions_cached(analysis, &all, sheet, views, cache)
        .into_iter()
        .filter(|q| q.subjects.contains(&r.id) && q.code != "annotation_over_title_block")
        .collect()
}

/// Draws a planned item into *items* (the dims layer) and its runs into *text*, from the plan
/// alone; returns its annotation for lint (no measurement claims: a frame states the source's
/// requirement, not a measurement of the part).
pub fn render(
    g: &PlannedGdt,
    items: &mut Vec<Item>,
    text: &mut Vec<TextRun>,
) -> Option<Annotation> {
    let p = g.placed.as_ref()?;
    let first = text.len();
    for [a, b] in p.lines.iter().chain(&p.strokes) {
        items.push(Item::Edge(Curve::Line(*a, *b)));
    }
    for [tip, tail] in &p.arrows {
        let n = [tail[1] - tip[1], tip[0] - tail[0]];
        let k = ARROW_HALF / n[0].hypot(n[1]).max(1e-12);
        let b1 = [tail[0] + k * n[0], tail[1] + k * n[1]];
        let b2 = [tail[0] - k * n[0], tail[1] - k * n[1]];
        items.push(Item::Face(vec![vec![
            Curve::Line(*tip, b1),
            Curve::Line(b1, b2),
            Curve::Line(b2, *tip),
        ]]));
    }
    for f in &p.fills {
        let wire: Vec<Curve> = (0..f.len())
            .map(|i| Curve::Line(f[i], f[(i + 1) % f.len()]))
            .collect();
        items.push(Item::Face(vec![wire]));
    }
    for a in &p.arcs {
        items.push(Item::Edge(Curve::Conic {
            centre: [a[0], a[1]],
            radii: [a[2], a[2]],
            x_axis: [1.0, 0.0],
            y_axis: [0.0, 1.0],
            t0: a[3].to_radians(),
            t1: a[4].to_radians(),
        }));
    }
    let f = font();
    for l in &p.letters {
        let laid = f.text(&l.text, l.size);
        let shift = |q: P2| [q[0] + l.at[0], q[1] + l.at[1]];
        let wires: Vec<Vec<Curve>> = laid
            .contours
            .iter()
            .filter(|c| !c.is_empty())
            .map(|c| {
                c.iter()
                    .map(|s| match *s {
                        Segment::Line(a, b) => Curve::Line(shift(a), shift(b)),
                        Segment::Quad(a, b, c) => {
                            Curve::Beziers(vec![vec![shift(a), shift(b), shift(c)]])
                        }
                        Segment::Cubic(a, b, c, d) => {
                            Curve::Beziers(vec![vec![shift(a), shift(b), shift(c), shift(d)]])
                        }
                    })
                    .collect()
            })
            .collect();
        if !wires.is_empty() {
            items.push(Item::Face(wires));
        }
    }
    for r in &p.runs {
        text.push(TextRun {
            text: r.text.clone(),
            at: r.at,
            size: r.size,
            rotation: 0.0,
            font: FONT.into(),
            h_align: HAlign::Center,
            v_align: VAlign::Middle,
        });
    }
    Some(Annotation {
        approved_claims: Vec::new(),
        approved_text: None,
        coordinate_origin: None,
        satisfies: Vec::new(),
        name: g.id.clone(),
        view: Some(g.view.clone()),
        runs: (first..text.len()).collect(),
        claims: Vec::new(),
    })
}

fn placement_choices(
    item: &Lowered,
    record: &crate::pmi::Record,
    views: &[PlannedView],
) -> Vec<(&'static str, Vec<&'static str>)> {
    views
        .iter()
        .filter_map(|view| {
            let name = match view.id.as_str() {
                "v:front" => "front",
                "v:plan" => "plan",
                "v:side" => "side",
                _ => return None,
            };
            match &item.glyph {
                Glyph::ControlFrame { .. } | Glyph::Label { .. } => {
                    (name == item.view).then(|| (name, vec!["above", "below", "left", "right"]))
                }
                Glyph::Datum { surface, .. } | Glyph::Finish { surface, .. }
                    if surface == "plane" =>
                {
                    let normal = record.geometry.as_ref()?.normal?;
                    let dot: f64 = normal.iter().zip(view.direction).map(|(a, b)| a * b).sum();
                    if dot.abs() > 1e-6 {
                        return None;
                    }
                    let projected = haecceity::hlr::View::new(view.direction, view.up)?.map(normal);
                    let side = if projected[0].abs() > 0.9 {
                        if projected[0] > 0.0 { "right" } else { "left" }
                    } else if projected[1].abs() > 0.9 {
                        if projected[1] > 0.0 { "above" } else { "below" }
                    } else {
                        return None;
                    };
                    Some((name, vec![side]))
                }
                Glyph::Datum { .. } | Glyph::Finish { .. } => {
                    (name == item.view).then(|| (name, vec![item.side]))
                }
            }
        })
        .collect()
}

/// Authored carriers must prove the attachment before placement.
pub(crate) fn validate_placements(
    analysis: &Analysis,
    spec: &Spec,
    views: &[PlannedView],
) -> Result<(), crate::model::Invalid> {
    if spec.annotations.is_empty() {
        return Ok(());
    }
    let shown = |name: &str| views.iter().any(|v| v.id == format!("v:{name}"));
    let (items, _) = crate::pmi_lowering::lower(analysis, &shown);
    let all: Vec<_> = PRINCIPALS
        .iter()
        .map(|(name, direction, up)| PlannedView {
            detail: None,
            sheet: 1,
            scale_note: None,
            xref: None,
            id: format!("v:{name}"),
            direction: *direction,
            up: *up,
            at: [0.0; 2],
            scale: 1.0,
            area: [0.0; 4],
            default: true,
            basis: String::new(),
            state: State::Shown,
            cut: None,
            line_work: Vec::new(),
            line_style: Default::default(),
            from_spec: Vec::new(),
        })
        .collect();
    let Some(crate::pmi::Reading::Read(report)) = &analysis.pmi else {
        return Ok(());
    };
    for item in items {
        let Some(entry) = spec.annotations.get(&item.id).filter(|e| !e.omit) else {
            continue;
        };
        let Some(place) = &entry.place else {
            continue;
        };
        let Some(record) = report.records.iter().find(|r| r.source_id == item.source) else {
            continue;
        };
        let choices = placement_choices(&item, record, &all);
        let view = place.view.as_deref().unwrap_or(item.view);
        let Some(_) = choices.iter().find(|(name, _)| *name == view) else {
            return Err(crate::model::Invalid {
                message: format!(
                    "{} cannot attach in the {view} view; choose a compatible view.",
                    record.label
                ),
                choices: choices.iter().map(|(name, _)| name.to_string()).collect(),
            });
        };
    }
    Ok(())
}

pub fn placement_options(
    analysis: &Analysis,
    views: &[PlannedView],
) -> BTreeMap<String, serde_json::Value> {
    let (items, _) = crate::pmi_lowering::lower(analysis, &|name| {
        views.iter().any(|view| view.id == format!("v:{name}"))
    });
    let identities = match analysis.pmi.as_ref() {
        Some(crate::pmi::Reading::Read(report)) => report.identities(),
        _ => return BTreeMap::new(),
    };
    items
        .into_iter()
        .filter_map(|item| {
            let Some(crate::pmi::Reading::Read(report)) = &analysis.pmi else {
                return None;
            };
            let record = report.records.iter().find(|r| r.source_id == item.source)?;
            let id = identities.id(&record.source_id).ok()?;
            let choices = placement_choices(&item, record, views)
                .into_iter()
                .map(|(view, sides)| serde_json::json!({"view": view, "sides": sides}))
                .collect::<Vec<_>>();
            Some((
                id.to_string(),
                serde_json::json!({"label": record.label, "views": choices}),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planar_extensions_require_the_face_to_reach_the_outline() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/samples/cnc-review-spindle.step.gz");
        let (_, mut analysis) = crate::model::analyse_native(&path).unwrap();
        let (items, _) = crate::pmi_lowering::lower(&analysis, &|_| true);
        let item = items
            .iter()
            .find(|i| matches!(&i.glyph, Glyph::Datum { letter, .. } if letter == "A"))
            .unwrap();
        let frame = Frame::looking(&analysis, [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0; 2], 1.0);
        assert!(!plane_extensions(&analysis, item, &frame).is_empty());
        // Inset both ends of this edge-on face while keeping the view outline fixed.
        for face in &item.faces {
            let bounds = analysis.face_attachments[*face].bounds.as_mut().unwrap();
            bounds[0] = analysis.envelope[0][0] + 2.0;
            bounds[3] = analysis.envelope[1][0] - 2.0;
        }
        assert!(plane_extensions(&analysis, item, &frame).is_empty());
        // The guard is directional: reaching only the right outline offers only that end.
        for face in &item.faces {
            analysis.face_attachments[*face].bounds.as_mut().unwrap()[3] = analysis.envelope[1][0];
        }
        let sites = plane_extensions(&analysis, item, &frame);
        assert!(!sites.is_empty());
        assert!(sites.iter().all(|s| s.0[0] > frame.outline()[2]));
    }
}
