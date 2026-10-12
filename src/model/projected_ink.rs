//! Regenerable principal-view ink, shared with rendering; no topology identities in the Plan.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectedInk {
    pub visible: Vec<Curve>,
    pub hidden: Vec<Curve>,
    /// Complete boundary loops of planar faces parallel to the view, as sampled edge segments at 1:1.
    /// Even/odd containment includes inner loops. Missing material evidence fails closed.
    pub faces: Vec<Vec<[[f64; 2]; 2]>>,
}

pub(super) fn evidence(part: &Part) -> BTreeMap<String, ProjectedInk> {
    let mut out = BTreeMap::new();
    for (name, direction, up) in &VIEWS[..3] {
        let view = View::new(*direction, *up).expect("standard view");
        let Ok(pieces) = project(part, &view) else {
            continue;
        };
        let mut ink = ProjectedInk {
            visible: Vec::new(),
            hidden: Vec::new(),
            faces: Vec::new(),
        };
        let mut complete = true;
        for piece in &pieces {
            if let Some(c) = projected(piece) {
                if piece.visible {
                    ink.visible.push(c);
                } else {
                    ink.hidden.push(c);
                }
            } else {
                complete = false;
            }
        }
        if !complete || ink.visible.is_empty() {
            continue;
        }
        let axis = direction.iter().position(|x| x.abs() > 0.5).unwrap();
        for (i, face) in part.faces.iter().enumerate() {
            if !matches!(face.surface, haecceity::geom::Surface::Plane { .. })
                || part.outer_loop(i).is_none()
            {
                continue;
            }
            let b = part.face_bounds(i);
            if b.max[axis] - b.min[axis] > 1e-5 {
                continue;
            }
            let mut segments = Vec::new();
            for loop_ in &face.loops {
                for &(edge, _) in &loop_.edges {
                    let e = &part.edges[edge];
                    let points = haecceity::sampling::sample_edge(
                        &e.curve,
                        e.start,
                        e.end,
                        e.same_sense,
                        e.is_closed(),
                    );
                    segments.extend(points.windows(2).map(|p| [view.map(p[0]), view.map(p[1])]));
                }
            }
            if !segments.is_empty() {
                ink.faces.push(segments);
            }
        }
        out.insert(format!("v:{name}"), ink);
    }
    out
}

/// Uniform placement of the renderer's lowered curves, preserving exact conics and splines.
pub(super) fn place(c: &Curve, s: f64, centre: [f64; 2], at: [f64; 2]) -> Curve {
    let p = |q: [f64; 2]| {
        [
            at[0] + s * (q[0] - centre[0]),
            at[1] + s * (q[1] - centre[1]),
        ]
    };
    match c {
        Curve::Line(a, b) => Curve::Line(p(*a), p(*b)),
        Curve::Conic {
            centre: c,
            radii,
            x_axis,
            y_axis,
            t0,
            t1,
        } => Curve::Conic {
            centre: p(*c),
            radii: [s * radii[0], s * radii[1]],
            x_axis: *x_axis,
            y_axis: *y_axis,
            t0: *t0,
            t1: *t1,
        },
        Curve::Beziers(poles) => Curve::Beziers(
            poles
                .iter()
                .map(|row| row.iter().map(|q| p(*q)).collect())
                .collect(),
        ),
        Curve::Rational(poles) => Curve::Rational(
            poles
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|q| {
                            let r = p([q[0] / q[2], q[1] / q[2]]);
                            [r[0] * q[2], r[1] * q[2], q[2]]
                        })
                        .collect()
                })
                .collect(),
        ),
    }
}

/// Evidence is bound to the standard projection basis, never merely to a view's name.
pub(crate) fn matches(v: &PlannedView) -> bool {
    v.cut.is_none()
        && VIEWS[..3].iter().any(|(name, direction, up)| {
            v.id.strip_prefix("v:") == Some(*name) && v.direction == *direction && v.up == *up
        })
}
