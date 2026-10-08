//! The port's SVG and DXF against draftwright's, written from the same drawings
//! (`tools/capture_output.py`): every layer must carry the same style and the same lines, each
//! side's within `common::TOL` of the other's. (The PDF, rendered and searched, is compared by
//! `tools/compare_pdf.py`, which needs a PDF renderer.) The text a PDF and an SVG carry is read
//! back here and must be the labels' text, callout symbols included.

#[path = "common/output.rs"]
mod common;
#[path = "common/corpus.rs"]
mod corpus;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::Path;

use draftwright::model::{self, Spec};
use draftwright::output::Drawing;
use serde_json::{Value, json};

#[test]
fn svg_and_dxf_match_draftwright() {
    let mut problems = Vec::new();
    for name in ["m10127", "string_post", "ctc01"] {
        let drawing = common::drawing(&common::load(name));
        for (kind, theirs, mine) in [
            (
                "svg",
                common::read_svg(
                    &String::from_utf8(common::gunzip(&format!("{name}.svg.gz"))).unwrap(),
                ),
                common::read_svg(&drawing.svg()),
            ),
            (
                "dxf",
                common::read_dxf(&common::gunzip(&format!("{name}.dxf.gz"))),
                common::read_dxf(&drawing.dxf()),
            ),
        ] {
            let layers: std::collections::BTreeSet<&String> =
                theirs.keys().chain(mine.keys()).collect();
            for layer in layers {
                let (Some(t), Some(m)) = (theirs.get(layer), mine.get(layer)) else {
                    problems.push(format!("{name} {kind} {layer}: on one side only"));
                    continue;
                };
                let (a, b) = (
                    common::covered(&t.lines, &m.lines),
                    common::covered(&m.lines, &t.lines),
                );
                if a < 0.9999 || b < 0.9999 || t.style != m.style {
                    problems.push(format!(
                        "{name} {kind} {layer}: {a:.4} {b:.4}, style {:?} vs {:?}",
                        t.style, m.style
                    ));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// cgb242's front view had a hidden stretch of no span (an exact cubic from t = 0 to t = 0, at a
/// vertex several edges share), which the port left out and reported (`edges_not_drawn`, still
/// tested on its own in `output`). haecceity now passes over such stretches at source, so every
/// stretch is drawn, nothing is reported, and the sheet is drawn and reviewed.
#[test]
fn cgb242_is_drawn_whole() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let mut step = Vec::new();
    flate2::read::GzDecoder::new(
        std::fs::File::open(dir.join("cadgenbench_inputs/cgb242.step.gz")).unwrap(),
    )
    .read_to_end(&mut step)
    .unwrap();
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("cgb242.step");
    std::fs::write(&path, step).unwrap();

    let (part, analysis) = model::analyse(&path).unwrap();
    let plan = model::plan(&analysis, &Spec::default()).unwrap();
    let font = draftwright::text::Font::new(
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/IBMPlexMono-Regular.ttf"),
        )
        .unwrap(),
    )
    .unwrap();
    let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
    let ids: Vec<&str> = drawing.problems.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.is_empty(), "{ids:?}");
    assert!(drawing.svg().contains("<path d="));
    assert!(!drawing.dxf().is_empty());
    assert!(!drawing.pdf(&common::fonts).is_empty());

    let review = draftwright::api::review(&path, None, &Spec::default()).unwrap();
    assert!(review.svg.contains("id=\"hidden\""));
}

/// A change to each hole's recognition record.
type Edit = dyn Fn(&mut Value);

/// The plate (four ø6 holes in a grid) drawn with its recognition changed by *edit*, its spec
/// embedded.
fn plate(edit: &Edit) -> Drawing {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut json: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/fixtures/recognition/plate.json")).unwrap(),
    )
    .unwrap();
    for f in json["quiddity"]["features"].as_array_mut().unwrap() {
        edit(&mut f["record"]);
    }
    let (part, mut a) = model::analyse(&root.join("tests/fixtures/parts/plate.step")).unwrap();
    let (r, features) =
        draftwright::bridge::read(&part, &a.binding.source_sha256, &json.to_string()).unwrap();
    a.recognition = Some(r);
    a.features = features;
    let plan = model::plan(&a, &Spec::default()).unwrap();
    let font =
        draftwright::text::Font::new(common::fonts("IBMPlexMono-Regular.ttf").unwrap()).unwrap();
    model::render(&part, &a, &plan, &font).unwrap()
}

/// The PDF's objects by number, found through its cross-reference table: each one's dictionary
/// and its stream, inflated.
fn pdf_objects(pdf: &[u8]) -> BTreeMap<u32, (String, Vec<u8>)> {
    let find = |hay: &[u8], needle: &[u8], from: usize| {
        hay[from..]
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|i| i + from)
    };
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    let tail = text(&pdf[pdf.len().saturating_sub(64)..]);
    let xref: usize = tail[tail.rfind("startxref").unwrap() + 9..]
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let table = text(&pdf[xref..]);
    let mut lines = table.lines().skip(1);
    let header: Vec<u32> = lines
        .next()
        .unwrap()
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect();
    let mut out = BTreeMap::new();
    for n in header[0]..header[0] + header[1] {
        let entry: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
        if entry[2] != "n" {
            continue;
        }
        let start: usize = entry[0].parse().unwrap();
        assert!(pdf[start..].starts_with(format!("{n} 0 obj").as_bytes()));
        let end = find(pdf, b"endobj", start).unwrap();
        let body = &pdf[start..end];
        let (dict, stream) = match find(body, b"stream\n", 0) {
            Some(k) => {
                let dict = text(&body[..k]);
                let length: usize = dict[dict.find("/Length ").unwrap() + 8..]
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                let raw = &body[k + 7..k + 7 + length];
                let mut data = Vec::new();
                if dict.contains("/FlateDecode") {
                    flate2::read::ZlibDecoder::new(raw)
                        .read_to_end(&mut data)
                        .unwrap();
                } else {
                    data = raw.to_vec();
                }
                (dict, data)
            }
            None => (text(body), Vec::new()),
        };
        out.insert(n, (dict, stream));
    }
    out
}

/// The object number *key* refers to in *dict* (`/Key N 0 R`).
fn reference(dict: &str, key: &str) -> u32 {
    let rest = &dict[dict
        .find(&format!("{key} "))
        .unwrap_or_else(|| panic!("{key} in {dict}"))
        + key.len()..];
    rest.split_whitespace().next().unwrap().parse().unwrap()
}

/// A content stream's tokens: the operands of an operator, then the operator.
#[derive(Debug)]
enum Token {
    Name(String),
    Bytes(Vec<u8>),
    Array(Vec<Token>),
    Other(String),
}

fn tokens(s: &[u8], i: &mut usize, close: Option<u8>) -> Vec<Token> {
    let delimiter = |b: u8| b.is_ascii_whitespace() || b"()<>[]/".contains(&b);
    let mut out = Vec::new();
    while *i < s.len() {
        let b = s[*i];
        *i += 1;
        match b {
            _ if b.is_ascii_whitespace() => {}
            _ if Some(b) == close => return out,
            b'[' => out.push(Token::Array(tokens(s, i, Some(b']')))),
            b'/' => {
                let start = *i;
                while *i < s.len() && !delimiter(s[*i]) {
                    *i += 1;
                }
                out.push(Token::Name(
                    String::from_utf8_lossy(&s[start..*i]).into_owned(),
                ));
            }
            b'<' => {
                let end = *i + s[*i..].iter().position(|&c| c == b'>').unwrap();
                let hex: Vec<u8> = s[*i..end]
                    .iter()
                    .copied()
                    .filter(|c| !c.is_ascii_whitespace())
                    .collect();
                out.push(Token::Bytes(
                    hex.chunks(2)
                        .map(|h| u8::from_str_radix(std::str::from_utf8(h).unwrap(), 16).unwrap())
                        .collect(),
                ));
                *i = end + 1;
            }
            b'(' => {
                let (mut bytes, mut depth) = (Vec::new(), 1);
                loop {
                    let c = s[*i];
                    *i += 1;
                    match c {
                        b'\\' => {
                            let e = s[*i];
                            *i += 1;
                            match e {
                                b'n' => bytes.push(b'\n'),
                                b'r' => bytes.push(b'\r'),
                                b't' => bytes.push(b'\t'),
                                b'b' => bytes.push(8),
                                b'f' => bytes.push(12),
                                b'0'..=b'7' => {
                                    let mut v = u32::from(e - b'0');
                                    for _ in 0..2 {
                                        if (b'0'..=b'7').contains(&s[*i]) {
                                            v = v * 8 + u32::from(s[*i] - b'0');
                                            *i += 1;
                                        }
                                    }
                                    bytes.push(v as u8);
                                }
                                other => bytes.push(other),
                            }
                        }
                        b'(' => {
                            depth += 1;
                            bytes.push(c);
                        }
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                            bytes.push(c);
                        }
                        _ => bytes.push(c),
                    }
                }
                out.push(Token::Bytes(bytes));
            }
            _ => {
                let start = *i - 1;
                while *i < s.len() && !delimiter(s[*i]) {
                    *i += 1;
                }
                out.push(Token::Other(
                    String::from_utf8_lossy(&s[start..*i]).into_owned(),
                ));
            }
        }
    }
    out
}

/// A ToUnicode CMap's codes and the text each maps to (single `bfchar` pairs only).
fn to_unicode(cmap: &[u8]) -> BTreeMap<u16, String> {
    let cmap = String::from_utf8(cmap.to_vec()).unwrap();
    assert!(!cmap.contains("bfrange"), "{cmap}");
    let mut map = BTreeMap::new();
    for line in cmap.lines() {
        let hex: Vec<&str> = line
            .split(['<', '>'])
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .collect();
        if let [code, text] = hex[..]
            && line.starts_with('<')
        {
            let units: Vec<u16> = (0..text.len())
                .step_by(4)
                .map(|k| u16::from_str_radix(&text[k..k + 4], 16).unwrap())
                .collect();
            map.insert(
                u16::from_str_radix(code, 16).unwrap(),
                String::from_utf16(&units).unwrap(),
            );
        }
    }
    map
}

/// The text a PDF's page shows, one string per text object, as a reader extracts it: each
/// two-byte code (Identity-H) through its font's ToUnicode CMap. A code the CMap does not map
/// fails the test.
fn pdf_text(pdf: &[u8]) -> Vec<String> {
    let objects = pdf_objects(pdf);
    let (page, _) = objects
        .values()
        .find(|(d, _)| d.contains("/Contents"))
        .expect("a page");
    let content = &objects[&reference(page, "/Contents")].1;
    let fonts = &page[page.find("/Font").expect("fonts") + 5..];
    let fonts = &fonts[..fonts.find(">>").unwrap()];
    let mut cmaps: BTreeMap<String, BTreeMap<u16, String>> = BTreeMap::new();
    for (name, _) in fonts.match_indices("/F") {
        let name = fonts[name + 1..].split_whitespace().next().unwrap();
        let font = &objects[&reference(fonts, &format!("/{name}"))].0;
        cmaps.insert(
            name.to_string(),
            to_unicode(&objects[&reference(font, "/ToUnicode")].1),
        );
    }
    let (mut runs, mut font, mut operands) = (Vec::new(), String::new(), Vec::new());
    let mut at = 0;
    for token in tokens(content, &mut at, None) {
        let Token::Other(op) = &token else {
            operands.push(token);
            continue;
        };
        let show = |t: &Token, runs: &mut Vec<String>| {
            if let Token::Bytes(b) = t {
                for code in b.chunks(2) {
                    let code = u16::from_be_bytes([code[0], code[1]]);
                    let text = cmaps[&font]
                        .get(&code)
                        .unwrap_or_else(|| panic!("code {code} of {font} has no Unicode"));
                    runs.last_mut().unwrap().push_str(text);
                }
            }
        };
        match op.as_str() {
            "BT" => runs.push(String::new()),
            "Tf" => {
                if let Some(Token::Name(n)) = operands.first() {
                    font = n.clone();
                }
            }
            "Tj" => show(&operands[0], &mut runs),
            "TJ" => {
                if let Token::Array(items) = &operands[0] {
                    for t in items {
                        show(t, &mut runs);
                    }
                }
            }
            _ => {}
        }
        if !op.parse::<f64>().is_ok() {
            operands.clear();
        } else {
            operands.push(token);
        }
    }
    runs
}

/// The text of an SVG's `<text>` elements, and whether each is invisible.
fn svg_text(svg: &str) -> Vec<String> {
    let invisible = svg
        .find("fill-opacity=\"0\"")
        .expect("an invisible text group");
    svg.match_indices("<text ")
        .map(|(i, _)| {
            assert!(i > invisible, "text outside the invisible group");
            let body = &svg[i..];
            let body = &body[body.find('>').unwrap() + 1..body.find("</text>").unwrap()];
            body.replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&")
        })
        .collect()
}

/// Text drawn on the sheet is searched and copied as it is written, callout symbols included:
/// read back from the PDF through the fonts' ToUnicode maps, and from the SVG's invisible text,
/// every label is its text run, so `⌀6` and the depth, counterbore and countersink symbols are
/// found. (draftwright's PDF text has `ø6` and spaces where the symbols are drawn as paths.)
#[test]
fn callout_symbols_are_searchable_in_the_pdf_and_the_svg() {
    let cases: [(&Edit, &str); 4] = [
        (&|_| {}, "4× ⌀6 THRU (2×2)"),
        (&|r| r["bottom"] = "flat".into(), "4× ⌀6 ↧ 5 (2×2)"),
        (
            &|r| r["cbore"] = json!({ "diameter": 11.0, "depth": 6.4 }),
            "4× ⌀6 THRU ⌴ ⌀11 ↧ 6.4 (2×2)",
        ),
        (
            &|r| {
                r["csink"] = json!({
                    "axis": r["axis"].clone(), "location": r["location"].clone(),
                    "major_diameter": 12.0, "drill_diameter": 6.0, "included_angle": 90.0,
                    "depth": 3.0,
                })
            },
            "4× ⌀6 THRU ⌵ ⌀12 × 90° (2×2)",
        ),
    ];
    for (edit, callout) in cases {
        let drawing = plate(edit);
        let written: Vec<String> = drawing.text.iter().map(|r| r.text.clone()).collect();
        assert!(
            written.iter().any(|t| t == callout),
            "{callout}: {written:?}"
        );
        assert!(written.iter().any(|t| t == "PLATE"), "{written:?}");
        let pdf = drawing.pdf(&common::fonts);
        assert_eq!(pdf_text(&pdf), written, "{callout}: the PDF's text");
        let svg = drawing.svg();
        assert_eq!(svg_text(&svg), written, "{callout}: the SVG's text");
        // The spec still travels in both.
        for file in [pdf, svg.into_bytes()] {
            assert!(model::embedded_spec(&file).unwrap().is_some());
        }
    }
}

/// The text layers are deterministic: the same part drawn twice gives the same PDF and SVG
/// bytes, symbols and all (the time zone is `tests/timezone.rs`'s).
#[test]
fn the_text_layers_are_deterministic() {
    let edit = |r: &mut Value| r["cbore"] = json!({ "diameter": 11.0, "depth": 6.4 });
    let (a, b) = (plate(&edit), plate(&edit));
    assert!(
        a.pdf(&common::fonts) == b.pdf(&common::fonts),
        "pdf differs"
    );
    assert!(a.svg() == b.svg(), "svg differs");
    // And a sheet in both fonts, each subset.
    let ctc01 = common::load("ctc01");
    let (a, b) = (common::drawing(&ctc01), common::drawing(&ctc01));
    assert!(
        a.pdf(&common::fonts) == b.pdf(&common::fonts),
        "ctc01's pdf differs"
    );
}

/// *glyphs* and the glyphs their composite glyphs are built from, in *face*'s `glyf` table.
fn with_components(face: &ttf_parser::Face, mut glyphs: BTreeSet<u16>) -> BTreeSet<u16> {
    let glyf = face
        .raw_face()
        .table(ttf_parser::Tag::from_bytes(b"glyf"))
        .unwrap();
    let loca = face
        .raw_face()
        .table(ttf_parser::Tag::from_bytes(b"loca"))
        .unwrap();
    let be = |d: &[u8], k: usize| u16::from_be_bytes([d[k], d[k + 1]]);
    let offset = |g: usize| match face.tables().head.index_to_location_format {
        ttf_parser::head::IndexToLocationFormat::Short => be(loca, 2 * g) as usize * 2,
        ttf_parser::head::IndexToLocationFormat::Long => {
            u32::from_be_bytes(loca[4 * g..4 * g + 4].try_into().unwrap()) as usize
        }
    };
    let mut todo: Vec<u16> = glyphs.iter().copied().collect();
    while let Some(g) = todo.pop() {
        let d = &glyf[offset(g as usize)..offset(g as usize + 1)];
        if d.len() < 10 || (be(d, 0) as i16) >= 0 {
            continue;
        }
        // Each component: flags, glyph, its offsets (words or bytes), then any scale.
        let mut k = 10;
        loop {
            let (flags, component) = (be(d, k), be(d, k + 2));
            if glyphs.insert(component) {
                todo.push(component);
            }
            k += 4 + if flags & 0x1 != 0 { 4 } else { 2 };
            k += match flags {
                f if f & 0x8 != 0 => 2,
                f if f & 0x40 != 0 => 4,
                f if f & 0x80 != 0 => 8,
                _ => 0,
            };
            if flags & 0x20 == 0 {
                break;
            }
        }
    }
    glyphs
}

/// Each font a PDF embeds holds only the glyphs its text shows, `.notdef`, and the glyphs
/// composite ones are built from (Plex Mono's `:` is two `.`): the font program (`FontFile2`) has
/// exactly that many glyphs, tagged as a subset (`ABCDEF+name`, the tag from the
/// glyphs kept). Each code shown still reaches, through `CIDToGIDMap`, the glyph the whole font
/// has for its character (`.notdef` for the callout symbols the font lacks), with the whole
/// font's advance, outline box and `/W` width, so the text's selection boxes do not move; and
/// the text still reads back as written. Each program is under a tenth of the whole font: ctc01's
/// PDF went from 244,241 bytes (fonts embedded whole) to 146,131, the plate's (counterbored)
/// from 113,025 to 58,883.
#[test]
fn pdf_fonts_hold_only_the_glyphs_shown() {
    let cbore = |r: &mut Value| r["cbore"] = json!({ "diameter": 11.0, "depth": 6.4 });
    let csink = |r: &mut Value| {
        r["csink"] = json!({
            "axis": r["axis"].clone(), "location": r["location"].clone(),
            "major_diameter": 12.0, "drill_diameter": 6.0, "included_angle": 90.0, "depth": 3.0,
        })
    };
    let drawings = [
        ("plate, counterbored", plate(&cbore)),
        ("plate, countersunk", plate(&csink)),
        ("ctc01", common::drawing(&common::load("ctc01"))),
    ];
    for (name, drawing) in &drawings {
        let pdf = drawing.pdf(&common::fonts);
        let objects = pdf_objects(&pdf);
        let written: Vec<String> = drawing.text.iter().map(|r| r.text.clone()).collect();
        assert_eq!(pdf_text(&pdf), written, "{name}: the PDF's text");
        let type0: Vec<&String> = objects
            .values()
            .map(|(d, _)| d)
            .filter(|d| d.contains("/Subtype /Type0"))
            .collect();
        let files: BTreeSet<&str> = drawing.text.iter().map(|r| r.font.as_str()).collect();
        assert_eq!(type0.len(), files.len(), "{name}: one font per file");
        for font in type0 {
            let base = font[font.find("/BaseFont /").unwrap() + 11..]
                .split_whitespace()
                .next()
                .unwrap();
            let (tag, file) = base.split_once('+').expect("a subset tag");
            assert!(
                tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()),
                "{name}: {base}"
            );
            let file = format!("{file}.ttf");
            let whole_data = common::fonts(&file).unwrap();
            let whole = ttf_parser::Face::parse(&whole_data, 0).unwrap();
            let shown: BTreeSet<u16> = drawing
                .text
                .iter()
                .filter(|r| r.font == file)
                .flat_map(|r| r.text.chars())
                .map(|ch| whole.glyph_index(ch).map_or(0, |g| g.0))
                .chain([0])
                .collect();
            let shown = with_components(&whole, shown);

            let descendant = &font[font.find("/DescendantFonts [").unwrap() + 18..];
            let descendant: u32 = descendant
                .split_whitespace()
                .next()
                .unwrap()
                .parse()
                .unwrap();
            let cid_font = &objects[&descendant].0;
            assert!(
                cid_font.contains(&format!("/BaseFont /{base}")),
                "{cid_font}"
            );
            let descriptor = &objects[&reference(cid_font, "/FontDescriptor")].0;
            assert!(
                descriptor.contains(&format!("/FontName /{base}")),
                "{descriptor}"
            );
            let (program_dict, program) = &objects[&reference(descriptor, "/FontFile2")];
            assert!(
                program_dict.contains(&format!("/Length1 {}", program.len())),
                "{program_dict}"
            );
            let subset = ttf_parser::Face::parse(program, 0).expect("the subset parses");
            assert_eq!(
                subset.number_of_glyphs() as usize,
                shown.len(),
                "{name} {file}: the glyphs embedded"
            );
            assert!(program.len() * 10 < whole_data.len(), "{name} {file}");

            let map: Vec<u16> = objects[&reference(cid_font, "/CIDToGIDMap")]
                .1
                .chunks(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            let w = &cid_font[cid_font.find("/W [").unwrap() + 4..];
            let w: Vec<f64> = w[..w.rfind(']').unwrap()]
                .split(['[', ']', ' '])
                .filter(|t| !t.is_empty())
                .map(|t| t.parse().unwrap())
                .collect();
            let widths: BTreeMap<u16, f64> = w.chunks(2).map(|p| (p[0] as u16, p[1])).collect();
            let unicode = to_unicode(&objects[&reference(font, "/ToUnicode")].1);
            // Each CID shown has a width (the ToUnicode map also holds its code space range).
            for (cid, width) in &widths {
                let ch = unicode[cid].chars().next().unwrap();
                let g = ttf_parser::GlyphId(whole.glyph_index(ch).map_or(0, |g| g.0));
                let s = ttf_parser::GlyphId(map[*cid as usize]);
                let advance = whole.glyph_hor_advance(g).unwrap();
                assert_eq!(subset.glyph_hor_advance(s), Some(advance), "{name} {ch:?}");
                assert_eq!(
                    subset.glyph_bounding_box(s),
                    whole.glyph_bounding_box(g),
                    "{name} {ch:?}"
                );
                let em = whole.units_per_em() as f64;
                assert!(
                    (width - advance as f64 * 1000.0 / em).abs() < 1e-3,
                    "{name} {ch:?}: /W"
                );
            }
        }
    }
}

/// ISSUE-1585-a and ISSUE-1536-a, the PDF's text layer: a supplied date and an overflowing value
/// are read back from the PDF, each whole and once, in the title block's own face.
#[test]
fn title_block_values_reach_the_pdf_text_layer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (part, mut a) = model::analyse(&root.join("tests/fixtures/parts/plate.step")).unwrap();
    let json = std::fs::read_to_string(root.join("tests/fixtures/recognition/plate.json")).unwrap();
    model::recognise(&part, &mut a, &json).unwrap();
    let (date, long) = ("2026-09-11", "ENGINEERING REVIEW GROUP / draftwright");
    let spec: Spec = serde_json::from_value(json!({ "sheet": { "size": "A2", "title": {
        "title": "FRAME", "date": date, "drawn_by": long } } }))
    .unwrap();
    let plan = model::plan(&a, &spec).unwrap();
    assert!(
        plan.problems
            .iter()
            .any(|p| p.code == "title_field_overflow")
    );
    let font =
        draftwright::text::Font::new(common::fonts("IBMPlexMono-Regular.ttf").unwrap()).unwrap();
    let drawing = model::render(&part, &a, &plan, &font).unwrap();
    let text = pdf_text(&drawing.pdf(&common::fonts));
    for value in [date, long] {
        assert_eq!(text.iter().filter(|t| *t == value).count(), 1, "{value}");
        let run = drawing.text.iter().find(|t| t.text == value).unwrap();
        assert_eq!(run.font, draftwright::title_block::FONT);
    }
}
