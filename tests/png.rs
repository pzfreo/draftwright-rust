//! The PNG (`src/png.rs`): the same bytes for the same drawing, in every time zone; the spec read
//! back from it; its size the sheet's at the resolution asked for; a resolution that is not
//! positive refused, as draftwright refuses it; and a corpus part's planned views inked (a sanity
//! check, not pixel parity with draftwright's PDFium raster).

#[path = "common/corpus.rs"]
mod corpus;

use std::path::{Path, PathBuf};
use std::process::Command;

use draftwright::api;
use draftwright::model::{self, Spec};
use serde_json::json;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn plate() -> (PathBuf, PathBuf) {
    (
        root().join("tests/fixtures/parts/plate.step"),
        root().join("tests/fixtures/recognition/plate.json"),
    )
}

/// The plate's PNG at *dpi* under *spec*, with its plan.
fn render(spec: &Spec, dpi: f64) -> (model::Plan, Vec<u8>) {
    let (step, features) = plate();
    let (plan, mut files) = api::render_at(&step, Some(&features), spec, &["png"], dpi).unwrap();
    assert_eq!(files.len(), 1);
    (plan, files.remove(0).1)
}

/// A PNG decoded: its header and its 8-bit RGB pixels.
fn decode(bytes: &[u8]) -> (png::Info<'static>, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    (reader.info().clone(), pixels)
}

#[test]
fn the_same_drawing_gives_the_same_bytes() {
    let spec = Spec::default();
    let (_, a) = render(&spec, api::DPI);
    let (_, b) = render(&spec, api::DPI);
    assert!(a == b, "two renders of the plate differ");
    // And through the operation by name, which writes the file.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("png-same");
    std::fs::create_dir_all(&dir).unwrap();
    let (step, features) = plate();
    let out = api::call(
        "render",
        &json!({ "step": step, "features": features, "output": dir.join("plate"),
            "formats": ["png"] }),
    )
    .unwrap();
    assert_eq!(
        out["written"],
        json!([dir.join("plate.png").display().to_string()])
    );
    assert!(std::fs::read(dir.join("plate.png")).unwrap() == a);
}

/// Zones whose offset in 1900 differs from their offset now, as in `tests/timezone.rs`.
const ZONES: [&str; 4] = [
    "UTC",
    "Asia/Kolkata",
    "America/St_Johns",
    "Pacific/Kiritimati",
];

#[test]
fn the_same_bytes_in_every_time_zone() {
    let zoneinfo = std::env::var_os("TZDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/zoneinfo"));
    for tz in &ZONES[1..] {
        assert!(
            zoneinfo.join(tz).is_file(),
            "{tz} is not in the zone database at {}",
            zoneinfo.display()
        );
    }
    let (step, features) = plate();
    let rendered: Vec<Vec<u8>> = ZONES
        .iter()
        .map(|tz| {
            let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
                .join("png-timezone")
                .join(tz.replace('/', "_"));
            std::fs::create_dir_all(&dir).unwrap();
            let out = Command::new(env!("CARGO_BIN_EXE_draftwright"))
                .env("TZ", tz)
                .arg("render")
                .arg(&step)
                .arg("--features")
                .arg(&features)
                .arg("-o")
                .arg(dir.join("plate"))
                .args(["--formats", "png", "--dpi", "100"])
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{tz}: {}",
                String::from_utf8_lossy(&out.stdout)
            );
            std::fs::read(dir.join("plate.png")).unwrap()
        })
        .collect();
    for (tz, bytes) in ZONES.iter().zip(&rendered).skip(1) {
        assert!(
            *bytes == rendered[0],
            "the PNG differs between UTC and {tz}"
        );
    }
}

#[test]
fn the_spec_reads_back_from_the_png() {
    let mut spec = Spec::default();
    spec.sheet.size = Some("A3".into());
    spec.sheet
        .title
        .insert("title".into(), "Plate ⌀6 × 4 — Ünïcode".into());
    let (plan, bytes) = render(&spec, api::DPI);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("png-spec");
    std::fs::create_dir_all(&dir).unwrap();
    let written = api::write(&dir.join("plate"), &[("png".into(), bytes.clone())]).unwrap();
    let back = api::spec(Path::new(&written[0])).unwrap();
    assert_eq!(
        serde_json::to_value(&back).unwrap(),
        serde_json::to_value(&plan.spec).unwrap()
    );
    assert_eq!(back.sheet.title["title"], "Plate ⌀6 × 4 — Ünïcode");

    // It is an iTXt chunk a PNG reader sees, with the attribution in tEXt chunks.
    let (info, _) = decode(&bytes);
    let itxt: Vec<_> = info
        .utf8_text
        .iter()
        .filter(|t| t.keyword == draftwright::png::SPEC_KEY)
        .collect();
    assert_eq!(itxt.len(), 1);
    let text: Vec<(&str, &str)> = info
        .uncompressed_latin1_text
        .iter()
        .map(|t| (t.keyword.as_str(), t.text.as_str()))
        .collect();
    assert_eq!(
        text,
        [
            ("Software", "draftwright"),
            (
                "Comment",
                "Generated by draftwright: https://draftwright.io"
            )
        ]
    );

    // A spec a tool has re-saved compressed still reads.
    let json = itxt[0].get_text().unwrap();
    let mut chunk = png::text_metadata::ITXtChunk::new(draftwright::png::SPEC_KEY, json.clone());
    chunk.compressed = true;
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 1, 1);
    encoder.set_color(png::ColorType::Grayscale);
    let mut writer = encoder.write_header().unwrap();
    writer.write_text_chunk(&chunk).unwrap();
    writer.write_image_data(&[255]).unwrap();
    writer.finish().unwrap();
    assert!(
        out.windows(4).any(|w| w == b"iTXt") && !out.windows(6).any(|w| w == b"schema"),
        "the test chunk is not compressed"
    );
    assert_eq!(draftwright::png::spec(&out).unwrap(), Some(json));

    // A PNG with no spec carries none; one cut off inside its spec is refused, not read as
    // having none.
    let mut plain = Vec::new();
    let mut writer = png::Encoder::new(&mut plain, 1, 1).write_header().unwrap();
    writer.write_image_data(&[0]).unwrap();
    writer.finish().unwrap();
    assert_eq!(model::embedded_spec(&plain).unwrap(), None);
    let at = bytes.windows(4).position(|w| w == b"iTXt").unwrap();
    assert!(model::embedded_spec(&bytes[..at + 40]).is_err());
}

#[test]
fn the_size_is_the_sheet_at_the_resolution() {
    for dpi in [api::DPI, 72.0, 96.5] {
        let (plan, bytes) = render(&Spec::default(), dpi);
        let (info, pixels) = decode(&bytes);
        let [w, h] = plan.sheet.page.map(|mm| (mm / 25.4 * dpi).ceil() as u32);
        assert_eq!((info.width, info.height), (w, h), "at {dpi} dpi");
        assert_eq!(pixels.len(), (w * h * 3) as usize, "8-bit RGB");
        let dims = info.pixel_dims.unwrap();
        assert_eq!(dims.unit, png::Unit::Meter);
        assert_eq!(dims.xppu, (dpi / 0.0254).round() as u32);
        assert_eq!(dims.yppu, dims.xppu);
    }
    // A page a whole number of pixels wide is that many, not one more (A4 at 254 dpi).
    assert_eq!(draftwright::png::size([210.0, 297.0], 254.0), [2100, 2970]);
    assert_eq!(draftwright::png::size([420.0, 297.0], 150.0), [2481, 1754]);
}

#[test]
fn a_resolution_that_is_not_positive_is_refused() {
    let (step, features) = plate();
    for dpi in [0.0, -150.0] {
        let e =
            api::render_at(&step, Some(&features), &Spec::default(), &["png"], dpi).unwrap_err();
        assert_eq!(e.code, "invalid");
        assert_eq!(e.message, format!("png export needs dpi > 0, got {dpi}"));
        let e = api::call(
            "render",
            &json!({ "step": step, "features": features, "output": "unused",
                "formats": ["svg", "png"], "dpi": dpi }),
        )
        .unwrap_err();
        assert_eq!(e.code, "invalid", "{e}");
    }
    // As draftwright's, the check is the PNG's: other formats take no resolution.
    let (_, files) =
        api::render_at(&step, Some(&features), &Spec::default(), &["svg"], 0.0).unwrap();
    assert_eq!(files.len(), 1);
    let e = api::call(
        "render",
        &json!({ "step": step, "output": "unused", "formats": ["png"], "dpi": "high" }),
    )
    .unwrap_err();
    assert_eq!(e.code, "invalid");
    // A raster too large to make is refused, not attempted.
    let e = api::render_at(&step, Some(&features), &Spec::default(), &["png"], 1e6).unwrap_err();
    assert_eq!(e.code, "invalid");
    assert!(e.message.contains("choose a lower dpi"), "{e}");
}

/// ftc06, with its section A–A hatched: every planned view's area has ink in it, and the page
/// outside the border is white.
#[test]
fn a_corpus_part_inks_every_planned_view() {
    let Some(dir) = corpus::dir() else {
        eprintln!("SKIPPED: the corpus is not found; set DRAFTWRIGHT_CORPUS");
        return;
    };
    let (part, analysis) = corpus::analyse(&dir, "nist/nist_ftc_06_asme1_rd.stp", "ftc06");
    let plan = model::plan(&analysis, &Spec::default()).unwrap();
    assert!(
        plan.views.iter().any(|v| v.cut.is_some()),
        "ftc06 has a section"
    );
    let font = draftwright::text::Font::new(
        std::fs::read(root().join("fonts/IBMPlexMono-Regular.ttf")).unwrap(),
    )
    .unwrap();
    let drawing = model::render(&part, &analysis, &plan, &font).unwrap();
    let dpi = 100.0;
    let (info, pixels) = decode(&drawing.png(dpi).unwrap());
    let k = dpi / 25.4;
    let (w, h) = (info.width as usize, info.height as usize);
    let page_h = plan.sheet.page[1];
    let inked = |[x0, y0, x1, y1]: [f64; 4]| {
        let cols = (x0 * k).floor().max(0.0) as usize..((x1 * k).ceil() as usize).min(w);
        let rows = ((page_h - y1) * k).floor().max(0.0) as usize
            ..(((page_h - y0) * k).ceil() as usize).min(h);
        rows.flat_map(|r| cols.clone().map(move |c| (r, c)))
            .filter(|(r, c)| pixels[3 * (r * w + c)..3 * (r * w + c) + 3] != [255, 255, 255])
            .count()
    };
    for v in &plan.views {
        let n = inked(v.area);
        assert!(n > 100, "{}: {n} inked pixels in {:?}", v.id, v.area);
    }
    // The visible lines are black: some pixel is.
    assert!(pixels.as_chunks::<3>().0.contains(&[0, 0, 0]));
    // A thin strip at the page's left edge, outside the border, carries only the ISO 5457
    // furniture (zone ticks and the centring mark): without it, the strip is white.
    assert!(inked([0.0, 0.0, 1.0, page_h]) > 0);
    let mut bare = plan.clone();
    bare.sheet.zones = None;
    let drawing = model::render(&part, &analysis, &bare, &font).unwrap();
    let (_, pixels) = decode(&drawing.png(dpi).unwrap());
    let rows = 0..h;
    let white = rows
        .flat_map(|r| (0..(k.ceil() as usize)).map(move |c| (r, c)))
        .all(|(r, c)| pixels[3 * (r * w + c)..3 * (r * w + c) + 3] == [255, 255, 255]);
    assert!(white);
}
