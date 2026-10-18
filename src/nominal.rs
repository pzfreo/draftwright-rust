//! Default nominal display intentionally departs from Python's one-decimal `_fmt`.
//! Tolerance magnitudes and explicit measurement precision have separate policies.
pub(crate) fn clean(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

pub(crate) fn format(value: f64) -> String {
    let value = clean(value);
    for places in 0..=3 {
        let text = format!("{value:.places$}");
        if places == 3
            || text
                .parse::<f64>()
                .is_ok_and(|v| (v - value).abs() <= 1e-6 + 1e-12)
        {
            let text = if places == 0 {
                text.as_str()
            } else {
                text.trim_end_matches('0').trim_end_matches('.')
            };
            return if text == "-0" {
                "0".into()
            } else {
                text.into()
            };
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    #[test]
    fn shortest_nominals_remove_noise_without_rounding_hundredths() {
        for (value, text) in [
            (4.75, "4.75"),
            (7.4, "7.4"),
            (12., "12"),
            (247.65000001, "247.65"),
            (247.64999999, "247.65"),
            (1.875, "1.875"),
            (1.23456, "1.235"),
            (-0.0000001, "0"),
            (4.750001, "4.75"),
            (-4.75, "-4.75"),
        ] {
            assert_eq!(super::format(value), text);
        }
    }
}

/// Refresh on the final live carriers, including notes and table-owned measurements.
pub(crate) fn report(plan: &mut crate::model::Plan) {
    plan.problems.retain(|p| p.code != "nominal_rounded");
    let spec = &plan.spec;
    let mut problems = std::collections::BTreeMap::new();
    let mut check = |id: &str, measurements: &[crate::dims::Measurement]| {
        for m in measurements {
            let measurement = format!("d:{}/{}", m.feature, m.parameter);
            if spec.precision.contains_key(&measurement)
                || (id
                    .rsplit_once('/')
                    .is_some_and(|(_, role)| role == m.parameter)
                    && spec.precision.contains_key(id))
            {
                continue;
            }
            let printed = format(m.value);
            if printed
                .parse::<f64>()
                .is_ok_and(|v| (v - clean(m.value)).abs() > 1e-6 + 1e-12)
            {
                problems.insert((id.to_string(), measurement.clone()), crate::model::Problem {
                    id: format!("p:nominal_rounded:{id}:{measurement}"),
                    code: "nominal_rounded".into(), severity: "info".into(),
                    message: format!("Dimension {id} ({measurement}) has modelled nominal {} but prints {printed} at the default three-place limit. Set its measurement precision to show more places.", clean(m.value)),
                    subjects: vec![id.into()], ..Default::default()
                });
            }
        }
    };
    for d in &plan.dimensions {
        if d.placed.is_some() {
            check(&d.id, &d.measurements);
        }
    }
    for n in &plan.notes {
        if n.placed.is_some() {
            check(&n.id, &n.measurements);
        }
    }
    for t in &plan.hole_tables {
        check(&t.id, &t.covered);
        for r in &t.rows {
            check(&t.id, &r.measurements);
        }
    }
    plan.problems.extend(problems.into_values());
}
