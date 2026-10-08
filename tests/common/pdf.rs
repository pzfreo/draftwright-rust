//! A written PDF and SVG read back as a reader reads them: the PDF's objects, its page's text
//! through the fonts' ToUnicode maps, and an SVG's invisible text.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::Read as _;

/// The PDF's objects by number, found through its cross-reference table: each one's dictionary
/// and its stream, inflated.
pub fn pdf_objects(pdf: &[u8]) -> BTreeMap<u32, (String, Vec<u8>)> {
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
pub fn reference(dict: &str, key: &str) -> u32 {
    let rest = &dict[dict
        .find(&format!("{key} "))
        .unwrap_or_else(|| panic!("{key} in {dict}"))
        + key.len()..];
    rest.split_whitespace().next().unwrap().parse().unwrap()
}

/// A content stream's tokens: the operands of an operator, then the operator.
#[derive(Debug)]
pub enum Token {
    Name(String),
    Bytes(Vec<u8>),
    Array(Vec<Token>),
    Other(String),
}

pub fn tokens(s: &[u8], i: &mut usize, close: Option<u8>) -> Vec<Token> {
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
pub fn to_unicode(cmap: &[u8]) -> BTreeMap<u16, String> {
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
pub fn pdf_text(pdf: &[u8]) -> Vec<String> {
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
pub fn svg_text(svg: &str) -> Vec<String> {
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
