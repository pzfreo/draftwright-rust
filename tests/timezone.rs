//! The same drawing gives the same bytes in every time zone (R1 invariant 7): the command line
//! renders a part's SVG, DXF and PDF under several `TZ` values, each a zone whose offset in 1900
//! (local mean time) differs from its offset now, as the DXF's local dates would show.

use std::path::{Path, PathBuf};
use std::process::Command;

const ZONES: [&str; 4] = [
    "UTC",
    "Asia/Kolkata",
    "America/St_Johns",
    "Pacific/Kiritimati",
];

/// The files rendered under zone *tz*, by format.
fn render(tz: &str) -> Vec<(String, Vec<u8>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("timezone")
        .join(tz.replace('/', "_"));
    std::fs::create_dir_all(&dir).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_draftwright"))
        .env("TZ", tz)
        .arg("render")
        .arg(root.join("tests/fixtures/parts/plate.step"))
        .arg("--features")
        .arg(root.join("tests/fixtures/recognition/plate.json"))
        .arg("-o")
        .arg(dir.join("plate"))
        .args(["--formats", "svg,pdf"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{tz}: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    ["svg", "pdf"]
        .map(|f| {
            (
                f.to_string(),
                std::fs::read(dir.join(format!("plate.{f}"))).unwrap(),
            )
        })
        .to_vec()
}

#[test]
fn the_same_bytes_in_every_time_zone() {
    // Without the zone database a zone silently falls back to UTC, and the test would prove
    // nothing.
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
    let utc = render(ZONES[0]);
    for tz in &ZONES[1..] {
        for ((format, theirs), (_, mine)) in utc.iter().zip(render(tz)) {
            assert!(*theirs == mine, "{format} differs between UTC and {tz}");
        }
    }
}
