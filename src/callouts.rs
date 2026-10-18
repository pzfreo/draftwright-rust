//! Shared manufacturing presentation and font-measured, word-preserving callout wrapping.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Auto,
    Compact,
    Full,
    Note,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub style: Style,
}

pub(crate) fn style(spec: &crate::model::Spec, owner: &str) -> Style {
    spec.features
        .get(owner)
        .and_then(|f| f.callouts)
        .or(spec.sheet.callouts)
        .unwrap_or_default()
        .style
}

/// A standalone newline is a callout line break, distinct from a numeric deviation stack.
pub const BREAK: &str = "\n";
pub const WIDTH: f64 = 48.0;
pub const LINE: f64 = 5.0;

pub fn lines(terms: &[String]) -> Vec<&[String]> {
    terms.split(|t| t == BREAK).collect()
}

pub fn label(terms: &[String]) -> String {
    lines(terms)
        .iter()
        .map(|line| line.join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn width(terms: &[String]) -> f64 {
    let ink = crate::dims::label_ink(terms, crate::dims::font());
    ink.max[0] - ink.min[0]
}

/// Python's 48 mm policy: intact words; a semicolon closes a half-used line.
/// Explicit breaks are retained. Numeric upper/lower deviation tokens stay intact.
pub fn wrap(terms: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, forced) in lines(terms).iter().enumerate() {
        if i > 0 {
            out.push(BREAK.into());
        }
        let mut line: Vec<String> = Vec::new();
        let flush = |out: &mut Vec<String>, line: &mut Vec<String>| {
            out.append(line);
            out.push(BREAK.into());
        };
        for token in *forced {
            let words: Vec<_> = if token.contains('\n') {
                vec![token.as_str()]
            } else {
                token.split_whitespace().collect()
            };
            for word in words {
                let mut next = line.clone();
                next.push(word.into());
                if !line.is_empty() && width(&next) > WIDTH {
                    flush(&mut out, &mut line);
                }
                line.push(word.into());
                if word.ends_with(';') && width(&line) >= WIDTH / 2.0 {
                    flush(&mut out, &mut line);
                }
            }
        }
        out.append(&mut line);
        if out.last().is_some_and(|s| s == BREAK) {
            out.pop();
        }
    }
    out
}
