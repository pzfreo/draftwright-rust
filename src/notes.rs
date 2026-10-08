//! General notes: the spec's `notes` (`n:<k>`, set in the spec), drawn as draftwright draws its
//! document notes (`render_document_notes`): one `GENERAL NOTES` table whose rows are
//! `document_note_rows`' (each note numbered by its place in the list, `1  BREAK ALL SHARP
//! EDGES`, wrapped at 48 characters with continuation rows indented three spaces), in IBM Plex
//! Mono at 3 mm, each row 3 mm plus 2 mm of padding above and below, ruled as `_build_table`
//! rules it: draftwright's single-column table, sharing the table's pieces (`src/tables.rs`).
//!
//! The block is reserved when the sheet is chosen, top right (draftwright's
//! `layout_required_tables`), and placed after the dimensions, as draftwright places it after
//! everything else, by `fit_box` towards the top right, first with 6 mm of breathing room and
//! then at the text padding: clear of the views' areas (their bands, where the dimension lanes
//! run), the isometric's drawn box, every dimension's ink and the title block. A block that
//! fits nowhere is a `notes_not_placed` error naming every note, which stays in the plan
//! undrawn; one that no sheet can hold at a standard scale is not reserved, so it never shrinks
//! the views (draftwright would take a computed scale for it).
//!
//! Differences from draftwright: a continuation row keeps its three-space indent, where
//! draftwright's left alignment on each row's ink drops it; the views' areas are kept clear,
//! where draftwright's late placement sees the views' drawn boxes and the annotations' ink.

use serde::{Deserialize, Serialize};

use crate::dims::PlannedDimension;
use crate::layout::Corner;
use crate::model::{Analysis, Invalid, PlannedSheet, PlannedView, Problem};
use crate::tables;

/// The block's heading, as draftwright's `document_note_rows` writes it.
pub const TITLE: &str = "GENERAL NOTES";
/// The characters a row holds before it wraps (draftwright's `textwrap.wrap(width=48)`).
pub const WRAP: usize = 48;
/// The text height, the padding around it and one row's height, mm: a table's.
pub use crate::tables::{PAD, ROW, TEXT, ink};

/// A general note as the spec states it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteSpec {
    /// `n:<k>`, `k` a whole number from 1.
    pub id: String,
    pub text: String,
}

/// A general note the sheet will carry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedNote {
    pub id: String,
    /// The note as the spec gives it, without surrounding whitespace.
    pub text: String,
    /// Its number in the block: its place in the spec's list, from 1.
    pub number: usize,
    /// The rows it is printed as, numbered and wrapped.
    pub rows: Vec<String>,
    pub default: bool,
    pub basis: String,
    /// Where it is drawn; absent when the block has no place, and a problem says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<PlacedNote>,
}

/// A note's place in the block, mm on the page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacedNote {
    /// Each row's ink centre, in the order of `rows`.
    pub rows_at: Vec<[f64; 2]>,
    /// Its rows' cells, `[x0, y0, x1, y1]`.
    #[serde(rename = "box")]
    pub cells: [f64; 4],
}

/// The block the notes are drawn in: its heading and box, mm on the page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotesBlock {
    pub title: String,
    /// The heading's ink centre.
    pub title_at: [f64; 2],
    /// The whole table, `[x0, y0, x1, y1]`, a row of [`ROW`] per heading and note row.
    #[serde(rename = "box")]
    pub cells: [f64; 4],
}

/// A note's ID is `n:` and a whole number from 1, written without leading zeros.
fn well_formed(id: &str) -> bool {
    id.strip_prefix("n:").is_some_and(|k| {
        !k.is_empty() && !k.starts_with('0') && k.bytes().all(|b| b.is_ascii_digit())
    })
}

/// The first `n:<k>` after every well-formed ID in *ids*.
pub fn next_id<'a>(ids: impl IntoIterator<Item = &'a str>) -> String {
    let k = ids
        .into_iter()
        .filter(|id| well_formed(id))
        .filter_map(|id| id[2..].parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    format!("n:{}", k + 1)
}

/// The notes are refused, with what to write instead, when an ID is not `n:<k>`, two notes
/// share an ID, or a note has no text.
pub fn check(notes: &[NoteSpec]) -> Result<(), Invalid> {
    let free = || vec![next_id(notes.iter().map(|n| n.id.as_str()))];
    for (i, n) in notes.iter().enumerate() {
        if !well_formed(&n.id) {
            return Err(Invalid {
                message: format!(
                    "notes/{i} has the ID {:?}; a note's ID is n:<k>, k a whole number from 1 (n:1, n:2, …).",
                    n.id
                ),
                choices: free(),
            });
        }
        if notes[..i].iter().any(|m| m.id == n.id) {
            return Err(Invalid {
                message: format!(
                    "Two notes have the ID {:?} (notes/{i} and an earlier one); give each note its own.",
                    n.id
                ),
                choices: free(),
            });
        }
        if n.text.trim().is_empty() {
            return Err(Invalid {
                message: format!(
                    "Note {} (notes/{i}) has no text; give it its text, or remove it.",
                    n.id
                ),
                choices: vec![],
            });
        }
    }
    Ok(())
}

/// *text* broken into rows of at most [`WRAP`] characters as Python's `textwrap.wrap` breaks it
/// (`break_long_words=False`, `break_on_hyphens=False`): tabs expanded to 8 columns, ASCII
/// whitespace read as spaces, words never split (a longer one has a row of its own), runs of
/// spaces inside a row kept, and spaces at a row's ends dropped.
pub fn wrap(text: &str) -> Vec<String> {
    const SPACE: [char; 6] = ['\t', '\n', '\x0b', '\x0c', '\r', ' '];
    let mut expanded = String::new();
    let mut column = 0;
    for c in text.chars() {
        match c {
            '\t' => {
                let n = 8 - column % 8;
                expanded.extend(std::iter::repeat_n(' ', n));
                column += n;
            }
            '\n' | '\r' => {
                expanded.push(' ');
                column = 0;
            }
            c if SPACE.contains(&c) => {
                expanded.push(' ');
                column += 1;
            }
            c => {
                expanded.push(c);
                column += 1;
            }
        }
    }
    // Runs of spaces and runs of anything else, last first.
    let mut chunks: Vec<String> = Vec::new();
    for c in expanded.chars() {
        match chunks.last_mut() {
            Some(last) if (last.starts_with(' ')) == (c == ' ') => last.push(c),
            _ => chunks.push(c.to_string()),
        }
    }
    chunks.reverse();
    let blank = |s: &str| s.trim().is_empty();
    let len = |s: &str| s.chars().count();
    let mut lines = Vec::new();
    while !chunks.is_empty() {
        if !lines.is_empty() && chunks.last().is_some_and(|c| blank(c)) {
            chunks.pop();
        }
        let mut line: Vec<String> = Vec::new();
        let mut width = 0;
        while let Some(c) = chunks.last() {
            if width + len(c) > WRAP {
                break;
            }
            width += len(c);
            line.push(chunks.pop().expect("a chunk"));
        }
        if line.is_empty() && chunks.last().is_some_and(|c| len(c) > WRAP) {
            line.push(chunks.pop().expect("a chunk"));
        }
        if line.last().is_some_and(|c| blank(c)) {
            line.pop();
        }
        if !line.is_empty() {
            lines.push(line.concat());
        }
    }
    lines
}

/// Note *number*'s rows: the first numbered, the rest indented to its text.
pub fn rows(number: usize, text: &str) -> Vec<String> {
    let lines = wrap(text.trim());
    let lines = if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    };
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                format!("{number}  {l}")
            } else {
                format!("   {l}")
            }
        })
        .collect()
}

/// The table's rows, heading first, and each note's first row among them.
fn table(notes: &[NoteSpec]) -> (Vec<String>, Vec<Vec<String>>) {
    let per: Vec<Vec<String>> = notes
        .iter()
        .enumerate()
        .map(|(i, n)| rows(i + 1, &n.text))
        .collect();
    let mut all = vec![TITLE.to_string()];
    all.extend(per.iter().flatten().cloned());
    (all, per)
}

/// The block's width (its widest row's ink, the rows aligned on their pens, and the padding
/// each side; at least 2.5 text heights, as `_table_metrics`) and the pen's offset from its
/// left edge.
fn width(rows: &[String]) -> (f64, f64) {
    // Each row's ink laid from a pen at 0, leading spaces advancing the pen.
    let spans: Vec<[f64; 2]> = rows.iter().filter_map(|r| tables::span(r)).collect();
    let lo = spans.iter().map(|s| s[0]).fold(f64::INFINITY, f64::min);
    let hi = spans.iter().map(|s| s[1]).fold(f64::NEG_INFINITY, f64::max);
    ((hi - lo + 2.0 * PAD).max(2.5 * TEXT), PAD - lo)
}

/// The block's size, mm, or none when there are no notes: what the sheet reserves for it.
pub fn size(notes: &[NoteSpec]) -> Option<[f64; 2]> {
    if notes.is_empty() {
        return None;
    }
    let (rows, _) = table(notes);
    Some([width(&rows).0, ROW * rows.len() as f64])
}

/// What [`plan`] returns: the notes, the block when it has a place, and the problems.
pub type Planned = (Vec<PlannedNote>, Option<NotesBlock>, Vec<Problem>);

/// The spec's notes as Plan items, placed in one block clear of the views, the dimensions and
/// the title block, or reported when the block has no place.
pub fn plan(
    notes: &[NoteSpec],
    analysis: &Analysis,
    sheet: &PlannedSheet,
    views: &[PlannedView],
    dims: &[PlannedDimension],
) -> Planned {
    if notes.is_empty() {
        return (Vec::new(), None, Vec::new());
    }
    let (all, per) = table(notes);
    let (w, pen) = width(&all);
    let h = ROW * all.len() as f64;
    let at = tables::place(
        [w, h],
        sheet,
        &tables::obstacles(analysis, sheet, views, dims),
        Corner::TopRight,
    );
    // Row *i*'s ink centre, counted from the heading, and its cell's bottom.
    let font = crate::dims::font();
    let centre = |x0: f64, top: f64, i: usize, row: &str| {
        let laid = font.baseline(&ink(row), TEXT);
        let mid = top - (i as f64 + 0.5) * ROW;
        [x0 + pen + 0.5 * (laid.min[0] + laid.max[0]), mid]
    };
    let mut problems = Vec::new();
    let block = at.map(|[x0, y0]| NotesBlock {
        title: TITLE.into(),
        title_at: centre(x0, y0 + h, 0, TITLE),
        cells: [x0, y0, x0 + w, y0 + h],
    });
    if block.is_none() {
        problems.push(Problem {
            id: "p:notes_not_placed:notes".into(),
            code: "notes_not_placed".into(),
            severity: "error".into(),
            message: format!(
                "The general notes block ({w:.1} × {h:.1} mm) has no place on the sheet clear of the views, their dimensions and the title block; its notes stay in the plan undrawn. Shorten or remove notes, or choose a larger sheet (sheet/size) or a smaller scale (sheet/scale)."
            ),
            subjects: notes.iter().map(|n| n.id.clone()).collect(),
        });
    }
    let mut first = 1;
    let items = notes
        .iter()
        .zip(per)
        .enumerate()
        .map(|(i, (n, rows))| {
            let placed = at.map(|[x0, y0]| {
                let top = y0 + h;
                PlacedNote {
                    rows_at: rows
                        .iter()
                        .enumerate()
                        .map(|(j, r)| centre(x0, top, first + j, r))
                        .collect(),
                    cells: [
                        x0,
                        top - (first + rows.len()) as f64 * ROW,
                        x0 + w,
                        top - first as f64 * ROW,
                    ],
                }
            });
            first += rows.len();
            PlannedNote {
                id: n.id.clone(),
                text: n.text.trim().into(),
                number: i + 1,
                rows,
                default: false,
                basis: format!(
                    "authored: the spec's note {} (notes/{i}), general note {} in the {TITLE} block, which goes towards the sheet's top right as draftwright places its document notes",
                    n.id,
                    i + 1
                ),
                placed,
            }
        })
        .collect();
    (items, block, problems)
}

/// The block's rules: its outline and a line under each row, as `_build_table` rules a table.
pub fn rules(block: &NotesBlock) -> Vec<[[f64; 2]; 2]> {
    let [x0, _, x1, _] = block.cells;
    tables::ruling(block.cells, &[x0, x1])
}

/// Every row the block draws, heading first, with its ink centre.
pub fn texts<'a>(block: &'a NotesBlock, notes: &'a [PlannedNote]) -> Vec<(&'a str, [f64; 2])> {
    let mut out = vec![(block.title.as_str(), block.title_at)];
    for n in notes {
        if let Some(p) = &n.placed {
            out.extend(
                n.rows
                    .iter()
                    .map(String::as_str)
                    .zip(p.rows_at.iter().copied()),
            );
        }
    }
    out
}
