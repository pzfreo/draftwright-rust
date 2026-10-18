//! Corner-block tables: the spec's `tables` (`t:<k>`, set in the spec), drawn as draftwright's
//! `Sheet.table` draws a declared table (revision blocks, BOMs, schedules) through
//! `Drawing.add_table` and `_build_table`: row 0 the header, each column as wide as its widest
//! cell's ink with 2 mm of padding each side (at least 2.5 text heights, as `_table_metrics`),
//! each cell's ink centred in its cell, IBM Plex Mono at 3 mm, each row 3 mm plus 2 mm of
//! padding above and below, ruled at every column edge and every row edge.
//!
//! Each table is reserved when the sheet is chosen, towards its `prefer` corner (draftwright's
//! `layout_required_tables`, which reserves declared tables ahead of the general notes), and
//! placed after the dimensions and the general notes, in the spec's order, as draftwright places
//! them after the build: by `fit_box` towards that corner, first with 6 mm of breathing room and
//! then at the text padding, clear of the views' areas (the isometric's drawn box), every
//! dimension's ink, the title block, the notes block and the tables placed before it. A table
//! that fits nowhere is a `table_not_placed` error naming it (draftwright's `table_dropped`),
//! which stays in the plan undrawn.
//!
//! The pieces the general notes block shares with a table (its text height, padding and row,
//! the font-safe ink, what a block keeps clear of, its placement and its rules) live here; the
//! notes block is draftwright's single-column table (`src/notes.rs`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dims::PlannedDimension;
use crate::items::State;
use crate::layout::{self, Corner, MARGIN, Rect};
use crate::model::{Analysis, Invalid, PlannedSheet, PlannedView, Problem};

/// The text height and the padding around it, mm (draftwright's `_FONT_SIZE` and the draft
/// preset's `pad_around_text`).
pub const TEXT: f64 = 3.0;
pub const PAD: f64 = 2.0;
/// One row's height.
pub const ROW: f64 = TEXT + 2.0 * PAD;
/// The corners a table may prefer, as draftwright names them.
pub const CORNERS: [&str; 4] = ["tr", "tl", "br", "bl"];

/// What a cell draws: `⌀`, which the face lacks, as `ø` (draftwright's `_font_safe_text`); its
/// text run keeps the cell as given.
pub fn ink(text: &str) -> String {
    text.replace('⌀', "ø")
}

/// *text*'s ink `[x0, x1]` laid from a pen at 0, or none when it draws nothing.
pub(crate) fn span(text: &str) -> Option<[f64; 2]> {
    // The bundled table font and height are fixed. Layout trials repeatedly measure
    // the same schedule cells; retain their exact ink bounds, not their outlines.
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};
    type Memo = VecDeque<(String, Option<[f64; 2]>)>;
    static MEMO: OnceLock<Mutex<Memo>> = OnceLock::new();
    let text = ink(text);
    let memo = MEMO.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Some(span) = memo
        .lock()
        .expect("table metrics memo lock")
        .iter()
        .find(|(t, _)| *t == text)
        .map(|(_, span)| *span)
    {
        return span;
    }
    let laid = crate::dims::font().baseline(&text, TEXT);
    let span = (!laid.contours.is_empty()).then_some([laid.min[0], laid.max[0]]);
    if text.len() <= 4096 {
        let mut memo = memo.lock().expect("table metrics memo lock");
        memo.retain(|(t, _)| *t != text);
        if memo.len() == 128 {
            memo.pop_front();
        }
        memo.push_back((text, span));
    }
    span
}

/// What a block keeps clear of: the views' areas (the isometric's drawn box), every
/// dimension's ink and the title block.
pub(crate) fn obstacles(
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    dims: &[PlannedDimension],
) -> Vec<Rect> {
    let mut out: Vec<Rect> = views
        .iter()
        .filter(|v| v.sheet == sheet.number)
        .map(|v| {
            if v.id == "v:iso" {
                let ib = analysis.iso_bounds;
                [
                    v.at[0] + v.scale * ib[0],
                    v.at[1] + v.scale * ib[1],
                    v.at[0] + v.scale * ib[2],
                    v.at[1] + v.scale * ib[3],
                ]
            } else {
                v.area
            }
        })
        .collect();
    let bounds = |pts: &[[f64; 2]], grow: f64| {
        [
            pts.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min) - grow,
            pts.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min) - grow,
            pts.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max) + grow,
            pts.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max) + grow,
        ]
    };
    for p in dims
        .iter()
        .filter(|d| {
            views
                .iter()
                .find(|v| v.id == d.view)
                .map_or(sheet.number == 1, |v| v.sheet == sheet.number)
        })
        .filter_map(|d| d.placed.as_ref())
    {
        out.push(p.label_box);
        out.extend(p.lines.iter().map(|l| bounds(l, 0.0)));
        // An arrowhead is no wider than its length.
        out.extend(p.arrows.iter().map(|a| bounds(a, crate::dims::ARROW)));
    }
    out.push(sheet.title_block);
    out
}

/// A block of *size*'s lower-left corner on the sheet's drawing area, clear of *obstacles*,
/// towards *corner*, or none when it fits nowhere.
pub(crate) fn place(
    size: [f64; 2],
    sheet: &PlannedSheet,
    obstacles: &[Rect],
    corner: Corner,
) -> Option<[f64; 2]> {
    let [pw, ph] = sheet.page;
    let region = [MARGIN, MARGIN, pw - MARGIN, ph - MARGIN];
    layout::fit_auxiliary_box(size, region, obstacles, corner, PAD)
}

/// A block's rules, as `_build_table` rules a table: a vertical at each of *columns* (its
/// sides among them) and a horizontal at the top and bottom of each row.
pub fn ruling(cells: [f64; 4], columns: &[f64]) -> Vec<[[f64; 2]; 2]> {
    let [x0, y0, x1, y1] = cells;
    let n = ((y1 - y0) / ROW).round() as usize;
    let mut out: Vec<_> = columns.iter().map(|&x| [[x, y0], [x, y1]]).collect();
    out.extend((0..=n).map(|k| {
        let y = y1 - k as f64 * ROW;
        [[x0, y], [x1, y]]
    }));
    out
}

/// A table as the spec states it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<u32>,
    /// `t:<k>`, `k` a whole number from 1.
    pub id: String,
    /// Its rows, row 0 the header: lists of one length, each cell a string or a number.
    pub rows: Vec<Value>,
    /// The corner it goes towards: `tr` (the default), `tl`, `br` or `bl`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer: Option<String>,
}

impl TableSpec {
    /// The corner it goes towards, its name and the layout's.
    fn corner(&self) -> (&str, Corner) {
        match self.prefer.as_deref().unwrap_or("tr") {
            "tl" => ("tl", Corner::TopLeft),
            "br" => ("br", Corner::BottomRight),
            "bl" => ("bl", Corner::BottomLeft),
            _ => ("tr", Corner::TopRight),
        }
    }

    /// Its cells as text (a number as JSON writes it), once [`check`] has passed it.
    fn cells(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|r| {
                r.as_array()
                    .into_iter()
                    .flatten()
                    .map(|c| match c {
                        Value::String(s) => s.clone(),
                        c => c.to_string(),
                    })
                    .collect()
            })
            .collect()
    }
}

/// A table the sheet will carry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedTable {
    #[serde(
        default = "crate::sheets::one",
        skip_serializing_if = "crate::sheets::is_one"
    )]
    pub sheet: u32,
    pub id: String,
    /// Its cells as text, row 0 the header.
    pub rows: Vec<Vec<String>>,
    /// Compiler-owned provenance for tagged manufacturing rows, including wrapped continuations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_claims: Vec<RowClaims>,
    pub default: bool,
    pub basis: String,
    /// What became of it: shown, or why not ([`crate::items::State`]).
    pub state: crate::items::State,
    /// Where it is drawn; absent when it has no place, and a problem says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<PlacedTable>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowClaims {
    pub tag: String,
    pub claims: Vec<crate::output::Claim>,
}

/// A table's place, mm on the page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacedTable {
    /// Each cell's ink centre, by row and column; none for a cell that draws nothing.
    pub cells_at: Vec<Vec<Option<[f64; 2]>>>,
    /// The x of each column's left edge, and of the table's right edge.
    pub columns: Vec<f64>,
    /// The whole table, `[x0, y0, x1, y1]`, a row of [`ROW`] per row.
    #[serde(rename = "box")]
    pub cells: [f64; 4],
}

/// A table's ID is `t:` and a whole number from 1, written without leading zeros.
fn well_formed(id: &str) -> bool {
    id.strip_prefix("t:").is_some_and(|k| {
        !k.is_empty() && !k.starts_with('0') && k.bytes().all(|b| b.is_ascii_digit())
    })
}

/// The first `t:<k>` after every well-formed ID in *ids*.
pub fn next_id<'a>(ids: impl IntoIterator<Item = &'a str>) -> String {
    let k = ids
        .into_iter()
        .filter(|id| well_formed(id))
        .filter_map(|id| id[2..].parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    format!("t:{}", k + 1)
}

/// The tables are refused, with what to write instead, when an ID is not `t:<k>`, two tables
/// share an ID, a table has no rows, a row is a string or not a list, the rows are not all of
/// one non-zero length (draftwright's `Sheet.table` refusals), a cell is neither a string nor a
/// number, or a corner is not one of [`CORNERS`].
pub fn check(tables: &[TableSpec]) -> Result<(), Invalid> {
    let free = || vec![next_id(tables.iter().map(|t| t.id.as_str()))];
    let refuse = |message: String| Invalid {
        message,
        choices: vec![],
    };
    for (i, t) in tables.iter().enumerate() {
        if !well_formed(&t.id) {
            return Err(Invalid {
                message: format!(
                    "tables/{i} has the ID {:?}; a table's ID is t:<k>, k a whole number from 1 (t:1, t:2, …).",
                    t.id
                ),
                choices: free(),
            });
        }
        if tables[..i].iter().any(|u| u.id == t.id) {
            return Err(Invalid {
                message: format!(
                    "Two tables have the ID {:?} (tables/{i} and an earlier one); give each table its own.",
                    t.id
                ),
                choices: free(),
            });
        }
        let id = &t.id;
        if t.rows.is_empty() {
            return Err(refuse(format!(
                "Table {id} (tables/{i}) has no rows; give it a header row and its rows, or remove it."
            )));
        }
        let mut width = None;
        for (j, row) in t.rows.iter().enumerate() {
            let cells = match row {
                Value::String(_) => {
                    return Err(refuse(format!(
                        "Table {id} (tables/{i}/rows/{j}) has a row that is a string; a row is a list of cells. For a list of notes, use notes."
                    )));
                }
                Value::Array(cells) => cells,
                _ => {
                    return Err(refuse(format!(
                        "Table {id} (tables/{i}/rows/{j}) has a row that is not a list; a row is a list of cells."
                    )));
                }
            };
            let n = *width.get_or_insert(cells.len());
            if n == 0 || cells.len() != n {
                return Err(refuse(format!(
                    "Table {id} (tables/{i}/rows/{j}): its rows must all have the same number of cells, at least one (row 0 has {n}, row {j} has {}).",
                    cells.len()
                )));
            }
            if let Some(k) = cells.iter().position(|c| !(c.is_string() || c.is_number())) {
                return Err(refuse(format!(
                    "Table {id} (tables/{i}/rows/{j}/{k}) has a cell that is neither text nor a number; write it as text."
                )));
            }
        }
        if let Some(p) = &t.prefer
            && !CORNERS.contains(&p.as_str())
        {
            return Err(Invalid {
                message: format!(
                    "Table {id} (tables/{i}) prefers the corner {p:?}; a table goes towards tr, tl, br or bl."
                ),
                choices: CORNERS.iter().map(|c| c.to_string()).collect(),
            });
        }
    }
    Ok(())
}

/// Each column's width, as `_table_metrics` measures it: its widest cell's ink and the padding
/// each side, at least 2.5 text heights.
pub(crate) fn widths(rows: &[Vec<String>]) -> Vec<f64> {
    // Repeated cells have identical ink at the table's fixed font and text height. Keep
    // the exact measurement locally; long repeated requirements otherwise lay out every
    // glyph again for each row on every sheet/scale candidate.
    let mut spans = std::collections::BTreeMap::new();
    (0..rows[0].len())
        .map(|c| {
            let ink = rows
                .iter()
                .filter_map(|r| *spans.entry(r[c].as_str()).or_insert_with(|| span(&r[c])))
                .map(|s| s[1] - s[0])
                .fold(0.0, f64::max);
            (ink + 2.0 * PAD).max(2.5 * TEXT)
        })
        .collect()
}

/// A table's size, mm, from its cells.
fn measure(rows: &[Vec<String>]) -> [f64; 2] {
    [widths(rows).iter().sum(), ROW * rows.len() as f64]
}

/// Each table's size and corner, in the spec's order: what the sheet reserves for them.
pub fn sizes(tables: &[TableSpec]) -> Vec<([f64; 2], Corner)> {
    tables
        .iter()
        .map(|t| (measure(&t.cells()), t.corner().1))
        .collect()
}

/// The spec's tables as Plan items, each placed in the spec's order clear of the views, the
/// dimensions, the title block, *notes* (the general notes block's box) and the tables placed
/// before it, or reported when it has no place.
pub fn plan(
    tables: &[TableSpec],
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    dims: &[PlannedDimension],
    notes: Option<Rect>,
) -> (Vec<PlannedTable>, Vec<Problem>) {
    if tables.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut obstacles = obstacles(analysis, sheet, views, dims);
    obstacles.extend(notes);
    let mut problems = Vec::new();
    let items = tables
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let rows = t.cells();
            let widths = widths(&rows);
            let [w, h] = [widths.iter().sum::<f64>(), ROW * rows.len() as f64];
            let (prefer, corner) = t.corner();
            let at = place([w, h], sheet, &obstacles, corner);
            let placed = at.map(|at| {
                let p = lay(&rows, &widths, at);
                obstacles.push(p.cells);
                p
            });
            if placed.is_none() {
                problems.push(Problem {
                    id: format!("p:table_not_placed:{}", t.id),
                    code: "table_not_placed".into(),
                    severity: "error".into(),
                    message: format!(
                        "{} ({w:.1} × {h:.1} mm) has no place on the sheet clear of the views, their dimensions, the title block, the general notes and the tables before it; it stays in the plan undrawn. Shorten its cells, or choose a larger sheet (sheet/size) or a smaller scale (sheet/scale).",
                        crate::wording::capital(&crate::wording::block(&t.id))
                    ),
                    subjects: vec![t.id.clone()],
                    ..Default::default()
                });
            }
            PlannedTable {
                sheet: sheet.number,
                id: t.id.clone(),
                rows,
                row_claims: Vec::new(),
                default: false,
                basis: format!(
                    "authored: the spec's table {} (tables/{i}), towards the sheet's {prefer} corner as draftwright places a declared table",
                    t.id
                ),
                state: State::placed_or(placed.is_some(), "table_not_placed"),
                placed,
            }
        })
        .collect();
    (items, problems)
}

/// A table of *rows*, its columns *widths* wide, laid with its lower-left corner at *at*: each
/// column's edges and each cell's ink centre.
pub(crate) fn lay(rows: &[Vec<String>], widths: &[f64], at: [f64; 2]) -> PlacedTable {
    let [x0, y0] = at;
    let mut columns = vec![x0];
    for w in widths {
        columns.push(columns.last().expect("a column") + w);
    }
    let top = y0 + ROW * rows.len() as f64;
    let cells_at = rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            let y = top - (r as f64 + 0.5) * ROW;
            row.iter()
                .enumerate()
                .map(|(c, cell)| span(cell).map(|_| [0.5 * (columns[c] + columns[c + 1]), y]))
                .collect()
        })
        .collect();
    let x1 = *columns.last().expect("a column");
    PlacedTable {
        cells_at,
        columns,
        cells: [x0, y0, x1, top],
    }
}

/// Every cell a placed table draws, with its ink centre.
pub fn texts(table: &PlannedTable) -> Vec<(&str, [f64; 2])> {
    let Some(p) = &table.placed else {
        return Vec::new();
    };
    table
        .rows
        .iter()
        .zip(&p.cells_at)
        .flat_map(|(row, at)| row.iter().zip(at))
        .filter_map(|(cell, at)| at.map(|at| (cell.as_str(), at)))
        .collect()
}

/// The size of the manufacturing requirements table of *rows* (as [`measure`] sizes a table),
/// or none when there is none: what the sheet reserves for it, top right.
pub fn manufacturing_size(rows: &[Vec<String>]) -> Option<[f64; 2]> {
    (!rows.is_empty()).then(|| measure(rows))
}

/// The manufacturing requirements table (draftwright's `_place_manufacturing_schedule`): the
/// part's thread and knurl requirements when their callouts print `SEE MFG n`
/// ([`crate::pmi_lowering::manufacturing`]), drawn as a declared table is, its requirement
/// column left-aligned at the padding (`_left_align_cols=(1,)`), and placed towards the top
/// right clear of the views, the dimensions, the PMI and the title block, before the general
/// notes and the spec's tables (draftwright commits it before its other furniture). A table
/// that fits nowhere is a `table_not_placed` error naming it and the requirements it states,
/// which stays in the plan undrawn; draftwright then prints the requirements on their callouts
/// in full, which the port, whose callouts are placed by then, does not (a question for the
/// maintainer).
pub fn manufacturing(
    analysis: &Analysis,
    spec: &crate::model::Spec,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    dims: &[PlannedDimension],
) -> (Option<PlannedTable>, Vec<Problem>) {
    let m = crate::pmi_lowering::manufacturing_for(analysis, spec);
    let rows = m.rows();
    if rows.is_empty() {
        return (None, Vec::new());
    }
    let id = crate::pmi_lowering::SCHEDULE_ID;
    let widths = widths(&rows);
    let [w, h] = [widths.iter().sum::<f64>(), ROW * rows.len() as f64];
    let at = place(
        [w, h],
        sheet,
        &obstacles(analysis, sheet, views, dims),
        Corner::TopRight,
    );
    let placed = at.map(|at| {
        let mut p = lay(&rows, &widths, at);
        for (row, cells) in rows.iter().zip(&mut p.cells_at) {
            if let (Some(s), Some(c)) = (span(&row[1]), cells[1].as_mut()) {
                c[0] = p.columns[1] + PAD + 0.5 * (s[1] - s[0]);
            }
        }
        p
    });
    let scheduled = m.scheduled();
    let mut problems = Vec::new();
    if placed.is_none() {
        problems.push(Problem {
            id: format!("p:table_not_placed:{id}"),
            code: "table_not_placed".into(),
            severity: "error".into(),
            message: format!(
                "The manufacturing requirements table ({w:.1} × {h:.1} mm) has no place on the sheet clear of the views, their dimensions and the title block; it stays in the plan undrawn, so the callouts' SEE MFG references point to no table. Choose a larger sheet (sheet/size) or a smaller scale (sheet/scale)."
            ),
            subjects: vec![id.to_string()],
            // The threads and knurls it states, which are no items of their own.
            refs: scheduled,
            ..Default::default()
        });
    }
    let table = PlannedTable {
        sheet: 1,
        id: id.into(),
        rows,
        row_claims: m
            .tags
            .iter()
            .filter_map(|(source, tag)| {
                let owned = m.owned.iter().find(|o| &o.source == source)?;
                Some(RowClaims {
                    tag: tag.clone(),
                    claims: vec![crate::output::Claim {
                        feature: owned.id.clone(),
                        parameter: owned.aspect.into(),
                    }],
                })
            })
            .collect(),
        default: true,
        basis: format!(
            "source-authored: the part's effective thread and knurl requirements ({}): with two or more and one too long for its callout, each callout refers to its row (SEE MFG n) and this table states them, towards the sheet's top right, as draftwright's manufacturing schedule",
            m.tags
                .iter()
                .map(|(s, _)| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        state: State::placed_or(placed.is_some(), "table_not_placed"),
        placed,
    };
    (Some(table), problems)
}

/// The placed table as the blocks placed after it see it: its box as a label.
pub fn proxy(t: &PlannedTable) -> Option<PlannedDimension> {
    let p = t.placed.as_ref()?;
    Some(PlannedDimension {
        reference: false,
        determined_by: Vec::new(),
        region: None,
        id: t.id.clone(),
        kind: "table".into(),
        view: String::new(),
        value: 0.0,
        label: String::new(),
        terms: Vec::new(),
        measurements: Vec::new(),
        default: t.default,
        from_spec: Vec::new(),
        basis: t.basis.clone(),
        state: State::Shown,
        placed: Some(crate::dims::Placed {
            lines: Vec::new(),
            arrows: Vec::new(),
            label_at: [p.cells[0], p.cells[1]],
            rotation: 0.0,
            label_box: p.cells,
        }),
    })
}
