//! Display policy by semantic measurement ID. Nominals and tolerances remain independent.
use super::{Want, fmt, want_id};
use crate::model::{Invalid, Spec};
use serde::de::{Error, MapAccess, Visitor};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn read<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, u8>, D::Error> {
    struct Policies;
    impl<'de> Visitor<'de> for Policies {
        type Value = BTreeMap<String, u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("measurement IDs with integer precision from 0 to 15")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((id, places)) = map.next_entry::<String, u8>()? {
                if places > 15 {
                    return Err(A::Error::custom(
                        "precision must be an integer from 0 to 15",
                    ));
                }
                if out
                    .insert(id.clone(), places)
                    .is_some_and(|old| old != places)
                {
                    return Err(A::Error::custom(format!("conflicting precision for {id}")));
                }
            }
            Ok(out)
        }
    }
    d.deserialize_map(Policies)
}

/// Explicit precision retains trailing zeroes, including at zero; default formatting is intact.
pub fn format(value: f64, places: u8) -> String {
    let text = format!("{value:.p$}", p = usize::from(places));
    if text.starts_with('-') && text.parse::<f64>() == Ok(0.0) {
        text[1..].into()
    } else {
        text
    }
}

pub(super) fn apply(spec: &Spec, wants: &mut [Want]) -> Result<(), Invalid> {
    let bad = |message| Invalid {
        message,
        choices: Vec::new(),
    };
    if spec.precision.values().any(|p| *p > 15) {
        return Err(bad("precision must be an integer from 0 to 15".into()));
    }
    let known: BTreeSet<_> = wants
        .iter()
        .flat_map(|w| {
            std::iter::once(w.id.clone()).chain(
                w.measurements
                    .iter()
                    .map(|m| want_id(&m.feature, &m.parameter)),
            )
        })
        .collect();
    for id in spec.precision.keys() {
        if !known.contains(id) {
            return Err(bad(format!("There is no measurement {id:?} on this part.")));
        }
    }
    labels(spec, wants)
}

pub(super) fn labels<'a>(
    spec: &Spec,
    wants: impl IntoIterator<Item = &'a mut Want>,
) -> Result<(), Invalid> {
    let bad = |message| Invalid {
        message,
        choices: Vec::new(),
    };
    for w in wants {
        // A compound callout's scalar terms are in measurement order. Repeat that mapping
        // for each physical member; all members sharing one printed term must agree.
        let mut replacements: BTreeMap<(usize, usize, usize), String> = BTreeMap::new();
        let mut cursor: BTreeMap<&str, usize> = BTreeMap::new();
        let mut slots = Vec::new();
        for (i, term) in w.terms.iter().enumerate() {
            let mut start = None;
            for (j, c) in term
                .char_indices()
                .chain(std::iter::once((term.len(), ' ')))
            {
                if c.is_ascii_digit() || c == '.' {
                    start.get_or_insert(j);
                } else if let Some(a) = start.take() {
                    let prefix = &term[..a];
                    let suffix = &term[j..];
                    if !suffix.starts_with('×')
                        && !prefix.ends_with(['+', '-', 'M', 'P'])
                        && !prefix.contains('(')
                        && (a == 0 || prefix.ends_with(['⌀', 'ø', ' ', 'R']))
                        && let Ok(value) = term[a..j].parse::<f64>()
                    {
                        slots.push((i, a, j, value));
                    }
                }
            }
        }
        let mut seen = BTreeSet::new();
        for m in &w.measurements {
            if m.feature.starts_with("pmi:") && !w.id.starts_with("d:pmi:") {
                continue;
            }
            if !seen.insert((&m.feature, &m.parameter)) {
                continue;
            }
            let at = cursor.entry(&m.feature).or_default();
            let found = slots
                .iter()
                .enumerate()
                .skip(*at)
                .find(|(_, (_, _, _, v))| {
                    (*v - m.value).abs() < 1e-9
                        || fmt(m.value)
                            .parse::<f64>()
                            .is_ok_and(|n| (n - v).abs() < 1e-9)
                });
            let Some((j, &(i, a, b, _))) = found else {
                continue;
            };
            *at = j + 1;
            let policy = spec
                .precision
                .get(&want_id(&m.feature, &m.parameter))
                .or_else(|| {
                    (m.parameter == w.id.rsplit_once('/').map_or("", |(_, p)| p))
                        .then(|| spec.precision.get(&w.id))
                        .flatten()
                });
            if let Some(&p) = policy {
                let text = format(m.value, p);
                if let Some(old) = replacements.insert((i, a, b), text.clone())
                    && old != text
                {
                    return Err(bad(format!(
                        "conflicting precision on the shared term of {}: {old} and {text}; its measurements require separate dimension owners",
                        w.id
                    )));
                }
            }
        }
        for ((i, a, b), text) in replacements.into_iter().rev() {
            w.terms[i].replace_range(a..b, &text);
        }
    }
    Ok(())
}

/// A scalar dimension's leading display policy, including its grouped measurement identity.
pub(super) fn leading(spec: &Spec, w: &Want) -> Option<u8> {
    spec.precision.get(&w.id).copied().or_else(|| {
        let role = w.id.rsplit_once('/')?.1;
        w.measurements
            .iter()
            .filter(|m| m.parameter == role)
            .find_map(|m| {
                spec.precision
                    .get(&want_id(&m.feature, &m.parameter))
                    .copied()
            })
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn precision_boundaries_and_negative_zero_are_stable() {
        assert_eq!(super::format(1.25, 0), "1");
        assert_eq!(super::format(1.0, 15), "1.000000000000000");
        assert_eq!(super::format(-0.0001, 2), "0.00");
    }
}
