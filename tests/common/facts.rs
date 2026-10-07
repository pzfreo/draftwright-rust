//! draftwright's sheets recorded as facts (`tools/capture_sheet_facts.py`, schema in
//! `docs/facts.md`): what each sheet says (its views, measurements, text and lint findings),
//! read back for a fact-by-fact comparison with the port's plan.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

pub const SCHEMA: &str = "draftwright.sheet-facts/1";

#[derive(Debug, Deserialize)]
pub struct Facts {
    pub schema: String,
    pub binding: Binding,
    pub sheet: Sheet,
    pub views: Vec<View>,
    pub annotations: Vec<Annotation>,
    /// Annotations whose compiled meaning draftwright could not confirm, and why.
    pub unconfirmed: Vec<Unconfirmed>,
    pub title_block: BTreeMap<String, String>,
    pub lint: Vec<Finding>,
    /// How the scale, views, arrangement, section and details were decided, as draftwright
    /// records them (kept whole, not typed).
    pub decisions: Value,
    pub suppressions: Value,
    pub producer: Value,
}

/// The corpus file the sheet was drawn from.
#[derive(Debug, Deserialize)]
pub struct Binding {
    /// Relative to the corpus directory.
    pub file: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
pub struct Sheet {
    /// `A4` … `A0`, or `<w>x<h>`.
    pub format: String,
    pub page: [f64; 2],
    /// `n:m`.
    pub scale: String,
    pub scale_factor: f64,
    /// `third` or `first`.
    pub projection: Option<String>,
}

/// A view: its camera as a world basis, its scale and where it sits on the page (mm).
#[derive(Debug, Deserialize)]
pub struct View {
    pub name: String,
    /// `principal`, `pictorial`, `section` or `detail`.
    pub kind: Option<String>,
    /// The projected geometry's box, `[x0, y0, x1, y1]`, and its centre.
    pub bbox: Option<[f64; 4]>,
    pub centre: Option<[f64; 2]>,
    pub scale: Option<f64>,
    /// Where `look_at` lands on the page.
    pub origin: Option<[f64; 2]>,
    /// The world point the view is centred on, mm.
    pub look_at: Option<[f64; 3]>,
    /// The world directions of the page's x and y, and towards the viewer.
    pub x_axis: Option<[f64; 3]>,
    pub up: Option<[f64; 3]>,
    pub direction: Option<[f64; 3]>,
}

impl View {
    /// Where the world point *p* lands on the page in this view.
    pub fn page(&self, p: [f64; 3]) -> Option<[f64; 2]> {
        let (o, c, x, y, s) = (
            self.origin?,
            self.look_at?,
            self.x_axis?,
            self.up?,
            self.scale?,
        );
        let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
        let dot = |a: [f64; 3]| a[0] * d[0] + a[1] * d[1] + a[2] * d[2];
        Some([o[0] + s * dot(x), o[1] + s * dot(y)])
    }
}

/// A feature as draftwright names it: `key` is `kind@(x,y,z)/axis[sizes]`.
#[derive(Debug, Deserialize)]
pub struct Feature {
    pub key: Option<String>,
    pub kind: String,
    pub origin: Option<[f64; 3]>,
    pub axis: Option<String>,
}

/// An annotation the sheet's registry names.
#[derive(Debug, Deserialize)]
pub struct Annotation {
    pub name: String,
    /// The drawing class: `Dimension`, `Leader`, `Note`, `CenterMark`, `TitleBlock`, …
    #[serde(rename = "type")]
    pub kind: String,
    pub view: Option<String>,
    /// The printed text runs, as the PDF's text layer carries them.
    pub text: Vec<String>,
    pub label: Option<String>,
    pub label_bbox: Option<[f64; 4]>,
    /// The features it is attached to.
    pub features: Vec<Option<Feature>>,
    pub measurements: Vec<Measurement>,
    #[serde(default)]
    pub cells: Vec<Cell>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
    pub section_mark: Option<SectionMark>,
    /// The measured span in world mm, where draftwright records one.
    pub span: Option<Value>,
}

/// One measurement an annotation draws: its identity (feature, parameter), and the compiled
/// value and tolerance (`value` is none where draftwright's meanings could not settle one; the
/// tolerance is kept as draftwright's own record, none when the general tolerance applies).
#[derive(Debug, Deserialize)]
pub struct Measurement {
    pub feature: Option<Feature>,
    /// The role vocabulary, `role.kind[.discriminator]`.
    pub parameter: String,
    pub compiled: Option<Vec<Value>>,
    pub value: Option<f64>,
    pub tolerance: Value,
}

#[derive(Debug, Deserialize)]
pub struct Cell {
    pub cell: [usize; 2],
    #[serde(flatten)]
    pub measurement: Measurement,
}

#[derive(Debug, Deserialize)]
pub struct SectionMark {
    pub cut_y: f64,
    pub view: String,
}

#[derive(Debug, Deserialize)]
pub struct Unconfirmed {
    pub annotation: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct Finding {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub annotation: Option<String>,
    pub related: Vec<String>,
    pub view: Option<String>,
    pub measurements: Value,
    pub source_ids: Vec<String>,
    pub stage: Option<String>,
}

pub fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sheets")
}

/// The captured sheets' names, sorted.
pub fn names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().to_string_lossy().into_owned();
            name.strip_suffix(".json.gz").map(str::to_string)
        })
        .collect();
    names.sort();
    names
}

pub fn load(name: &str) -> Facts {
    let mut text = String::new();
    flate2::read::GzDecoder::new(
        std::fs::File::open(dir().join(format!("{name}.json.gz"))).unwrap(),
    )
    .read_to_string(&mut text)
    .unwrap();
    let facts: Facts = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(facts.schema, SCHEMA, "{name}");
    facts
}
