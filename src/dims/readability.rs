//! Local geometric readability crops and their exact interval witnesses.
use super::*;

/// Model-space crops for local geometric floors. Keep exact witness endpoints;
/// crop ownership changes only the carrier, never a nominal measurement.
pub(crate) fn details(a: &Analysis, spec: &Spec, sheet: f64) -> Vec<crate::detail::Detail> {
    if spec.dimension_source == DimensionSource::Authored {
        return Vec::new();
    }
    let inventory: Vec<_> = wants(a)
        .0
        .into_iter()
        .filter(|w| !spec.dimensions.get(&w.id).is_some_and(|d| d.omit))
        .collect();
    let annotation_context = crate::detail::NAMES.iter().any(|name| {
        spec.views
            .get(*name)
            .is_some_and(|v| *v == crate::model::ViewSpec::default())
    });
    let mut crops: Vec<crate::detail::Detail> = Vec::new();
    for (d, _) in height_intervals(a.envelope, &inventory, sheet) {
        let scale = d.scale;
        if let Some(previous) = crops.iter_mut().find(|p| {
            [0, 2].into_iter().all(|k| {
                p.bounds[0][k] <= d.bounds[1][k] + 20.0 / p.scale.max(scale)
                    && d.bounds[0][k] <= p.bounds[1][k] + 20.0 / p.scale.max(scale)
                    && (p.bounds[1][k].max(d.bounds[1][k]) - p.bounds[0][k].min(d.bounds[0][k]))
                        * p.scale.max(scale)
                        <= 100.0
            })
        }) {
            for k in 0..3 {
                previous.bounds[0][k] = previous.bounds[0][k].min(d.bounds[0][k]);
                previous.bounds[1][k] = previous.bounds[1][k].max(d.bounds[1][k]);
                previous.marked_bounds[0][k] =
                    previous.marked_bounds[0][k].min(d.marked_bounds[0][k]);
                previous.marked_bounds[1][k] =
                    previous.marked_bounds[1][k].max(d.marked_bounds[1][k]);
            }
            previous.dimensions.extend(d.dimensions);
            previous.scale = previous.scale.max(scale);
        } else {
            crops.push(d);
        }
    }
    let thread_scales: BTreeMap<_, _> = crate::pmi_lowering::manufacturing(a)
        .owned
        .into_iter()
        .filter_map(|o| {
            let d = o.depiction?;
            let crate::pmi_lowering::DepictionKind::Thread { minor } = d.kind else {
                return None;
            };
            let gap = (d.diameter - minor) * 0.5;
            (gap > 0.0).then_some((o.owner, crate::sheet_fit::MIN_DEPICTION_GAP / gap))
        })
        .collect();
    for w in &inventory {
        let mut needed = w
            .measurements
            .iter()
            .filter_map(|m| {
                // Locations keep their whole-part datum. The local size owner
                // is what enlarges an opening or an axial band in a detail.
                if m.parameter.contains("location") || m.parameter == "step_position.length" {
                    return None;
                }
                if m.parameter == "step.diameter"
                    && let Some(scale) = thread_scales.get(&m.feature)
                {
                    return Some(*scale);
                }
                let size = if native_opening(&m.feature, &m.parameter)
                    || m.parameter.starts_with("through_step_leg.length")
                    || m.parameter == "circular_step_depth.length"
                    || m.parameter.starts_with("step_height.length")
                {
                    m.value.abs()
                } else if matches!(m.parameter.as_str(), "step.length" | "step_position.length") {
                    let f = a
                        .features
                        .iter()
                        .find(|f| f.id == m.feature && f.kind == "turned_step")?;
                    f.parameters.get("hi")?.as_f64()? - f.parameters.get("lo")?.as_f64()?
                } else {
                    let f = a.features.iter().find(|f| f.id == m.feature)?;
                    let parameter = match f.kind.as_str() {
                        "hole" => "diameter",
                        "slot" => "width",
                        _ => return None,
                    };
                    f.parameters.get(parameter)?.as_f64()?
                };
                let floor =
                    if matches!(m.parameter.as_str(), "step.length" | "step_position.length") {
                        crate::sheet_fit::MIN_STEP_SPAN
                    } else {
                        crate::sheet_fit::MIN_OPENING
                    };
                (size.is_finite() && size > 0.0 && size * sheet + 1e-6 < floor)
                    .then_some(floor / size)
            })
            .fold(0.0, f64::max);
        let mut context = false;
        if needed <= sheet && annotation_context && matches!(w.shape, Shape::Callout { .. }) {
            // A last-resort local carrier for an aperture whose label cannot
            // remain inside at the main scale. This proposes a crop; the same
            // finished ink, route and readability gates still qualify it.
            if let Some(diameter) = w
                .measurements
                .iter()
                .find_map(|m| (m.parameter == "bore.diameter").then_some(m.value.abs()))
                .filter(|d| *d > 0.0)
            {
                needed = (font().advance(&w.terms.join(" "), TEXT) + 2.0 * ARROW + 2.0) / diameter;
                context = needed > sheet;
            }
        }
        if needed <= sheet {
            continue;
        }
        let Some(source) = w.views.iter().find(|v| NAMES.contains(v)) else {
            continue;
        };
        let (points, primary): (Vec<P3>, usize) = match &w.shape {
            Shape::Linear { a, b, axis, .. } => (vec![*a, *b], *axis),
            Shape::Aligned { a, b } => (vec![*a, *b], 0),
            Shape::Callout { sites } => {
                // Keep the entire opening, not only the selected leader endpoint.
                let native = w
                    .measurements
                    .iter()
                    .find(|m| native_opening(&m.feature, &m.parameter));
                let size = native.map(|m| m.value.abs()).unwrap_or_else(|| {
                    w.measurements
                        .iter()
                        .filter(|m| {
                            matches!(m.parameter.as_str(), "slot_width.length" | "bore.diameter")
                        })
                        .map(|m| m.value.abs())
                        .fold(1.0, f64::max)
                });
                let axis = match *source {
                    "plan" => 2,
                    "side" => 0,
                    _ => 1,
                };
                let mut points = Vec::new();
                // One proven leader site and the complete local mouth width are
                // sufficient; a long channel need not be enlarged end to end.
                for (site, radius) in sites.iter().take(1) {
                    let half = if native.is_some() {
                        size * 1.5
                    } else {
                        size.max(2.0 * radius)
                    };
                    points.push(std::array::from_fn(|k| {
                        site[k] - if k == axis { 0.0 } else { half }
                    }));
                    points.push(std::array::from_fn(|k| {
                        site[k] + if k == axis { 0.0 } else { half }
                    }));
                }
                (points, (axis + 1) % 3)
            }
            Shape::Angular { .. } => continue,
        };
        if points.is_empty() {
            continue;
        }
        let scale = crate::layout::SCALES
            .into_iter()
            .rev()
            .find(|s| *s >= needed && *s > sheet)
            .unwrap_or(needed);
        let pad = (3.0 / scale).max(0.5);
        let bounds = [
            std::array::from_fn(|k| {
                (points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min) - pad)
                    .max(a.envelope[0][k])
            }),
            std::array::from_fn(|k| {
                (points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max)
                    + pad)
                    .min(a.envelope[1][k])
            }),
        ];
        let d = crate::detail::Detail {
            label: String::new(),
            axis: primary,
            source_view: format!("v:{source}"),
            source_cut: None,
            kind: if context {
                "readability-annotation-context"
            } else {
                "readability"
            }
            .into(),
            bounds,
            marked_bounds: bounds,
            dimensions: vec![w.id.clone()],
            scale,
            authored: false,
            partial_profile: true,
        };
        // Group adjacent regions only when the union remains a compact detail.
        let axes = match *source {
            "plan" => [0, 1],
            "side" => [1, 2],
            _ => [0, 2],
        };
        if let Some(previous) = crops.iter_mut().find(|p| {
            p.source_view == d.source_view
                && axes.iter().all(|&k| {
                    p.bounds[0][k] <= d.bounds[1][k] + (20.0 / p.scale.max(d.scale)).max(pad)
                        && d.bounds[0][k] <= p.bounds[1][k] + (20.0 / p.scale.max(d.scale)).max(pad)
                        && (p.bounds[1][k].max(d.bounds[1][k]) - p.bounds[0][k].min(d.bounds[0][k]))
                            * p.scale.max(d.scale)
                            <= 100.0
                })
        }) {
            for k in 0..3 {
                previous.bounds[0][k] = previous.bounds[0][k].min(d.bounds[0][k]);
                previous.bounds[1][k] = previous.bounds[1][k].max(d.bounds[1][k]);
            }
            previous.marked_bounds = previous.bounds;
            if annotation_context
                && (previous.kind.starts_with("readability-height-interval")
                    || d.kind.starts_with("readability-height-interval"))
            {
                // This late recipe mixes annotation grammars. The floor-only
                // recipe remains an independently qualified fallback, and pins
                // can reconstruct the selected mixed recipe from its provenance.
                previous.kind = "readability-height-interval-context".into();
            } else if context {
                previous.kind = "readability-annotation-context".into();
            }
            previous.dimensions.extend(d.dimensions);
            previous.scale = previous.scale.max(d.scale);
        } else {
            crops.push(d);
        }
    }
    // An interval is useful only alongside a real parent annotation. Never
    // create a reference-only detail to satisfy a numerical readability floor.
    crops.retain(|d| {
        d.dimensions
            .iter()
            .any(|id| inventory.iter().any(|w| w.id == *id))
    });
    for crop in &mut crops {
        if crop
            .dimensions
            .iter()
            .any(|id| id.contains("/step_interval.length."))
        {
            crop.kind = "readability-height-interval-context".into();
        }
    }
    crops
}

/// Keep the two baseline coordinates on the parent and explicitly dimension
/// their local difference in a small crop. Both values and both physical support
/// witnesses determine the new interval; no existing coordinate is discarded.
fn height_intervals(
    envelope: [[f64; 3]; 2],
    inventory: &[Want],
    sheet: f64,
) -> Vec<(crate::detail::Detail, Want)> {
    let mut heights: Vec<_> = inventory
        .iter()
        .filter(|w| {
            w.measurements.iter().any(|m| {
                (m.feature == "step_level" && m.parameter.starts_with("step_height.length"))
                    || (m.feature == "envelope" && m.parameter == "height.length")
            })
        })
        .collect();
    heights.sort_by(|a, b| a.value.total_cmp(&b.value));
    heights.dedup_by(|a, b| (a.value - b.value).abs() < 1e-6);
    let mut out = Vec::new();
    for pair in heights.windows(2) {
        let gap = pair[1].value - pair[0].value;
        if gap <= 1e-6 || gap * sheet + 1e-6 >= crate::sheet_fit::MIN_OPENING {
            continue;
        }
        let (Shape::Linear { b: lower, .. }, Shape::Linear { b: upper, .. }) =
            (&pair[0].shape, &pair[1].shape)
        else {
            continue;
        };
        if ((upper[2] - lower[2]) - gap).abs() > 1e-6 {
            continue;
        }
        let needed = crate::sheet_fit::MIN_OPENING / gap;
        let scale = crate::layout::SCALES
            .into_iter()
            .rev()
            .find(|s| *s >= needed && *s > sheet)
            .unwrap_or(needed);
        let pad = (3.0 / scale).max(0.5);
        let marked = [
            std::array::from_fn(|k| lower[k].min(upper[k])),
            std::array::from_fn(|k| lower[k].max(upper[k])),
        ];
        let bounds = [
            std::array::from_fn(|k| (marked[0][k] - pad).max(envelope[0][k])),
            std::array::from_fn(|k| (marked[1][k] + pad).min(envelope[1][k])),
        ];
        let id = interval_id(&pair[0].id, &pair[1].id);
        let parameter = id.strip_prefix("d:step_level/").unwrap().to_string();
        let w = Want {
            id: id.clone(),
            views: vec!["front"],
            value: gap,
            terms: vec![fmt(gap)],
            measurements: vec![Measurement {
                feature: "step_level".into(),
                parameter,
                value: gap,
            }],
            basis: format!(
                "exact local difference of {} and {}; both baseline coordinates remain on the parent",
                pair[0].id, pair[1].id
            ),
            shape: Shape::Linear {
                a: *lower,
                b: *upper,
                axis: 2,
                a_edge: false,
                b_edge: false,
                order: 0,
            },
            turning: None,
        };
        out.push((
            crate::detail::Detail {
                label: String::new(),
                axis: 2,
                source_view: "v:front".into(),
                source_cut: None,
                kind: "readability-height-interval".into(),
                bounds,
                marked_bounds: marked,
                dimensions: vec![id],
                scale,
                authored: false,
                partial_profile: true,
            },
            w,
        ));
    }
    out
}

pub(super) fn append_intervals(
    a: &Analysis,
    spec: &Spec,
    inventory: &mut Vec<Want>,
    views: &[PlannedView],
) {
    if !views.iter().any(|v| {
        v.detail
            .as_ref()
            .is_some_and(|d| d.kind.starts_with("readability-height-interval"))
    }) {
        return;
    }
    let eligible: Vec<_> = inventory
        .iter()
        .filter(|w| !spec.dimensions.get(&w.id).is_some_and(|d| d.omit))
        .cloned()
        .collect();
    let intervals = height_intervals(a.envelope, &eligible, 0.0);
    for (_, w) in intervals {
        if views.iter().any(|v| {
            v.detail.as_ref().is_some_and(|d| {
                d.kind.starts_with("readability-height-interval") && d.dimensions.contains(&w.id)
            })
        }) && !inventory.iter().any(|existing| existing.id == w.id)
        {
            inventory.push(w);
        }
    }
}

/// Stable endpoint identities, including the overall height which closes the
/// last local interval in the same baseline ladder.
pub(crate) fn interval_id(lower: &str, upper: &str) -> String {
    let endpoint = |id: &str| {
        if id == "d:envelope/height.length" {
            "overall".to_string()
        } else {
            id.rsplit('.').next().unwrap().to_string()
        }
    };
    format!(
        "d:step_level/step_interval.length.{}.{}",
        endpoint(lower),
        endpoint(upper)
    )
}
