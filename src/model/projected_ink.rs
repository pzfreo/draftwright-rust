//! The part's edges and faces as compact, view-independent evidence for interior placement.
//!
//! The planner has no Part (ADR R1), so it cannot run hidden-line removal. Every curve the
//! renderer draws in an orthographic, uncut view is a projected edge (visible or hidden) or a
//! curved face's silhouette. The Analysis therefore carries each edge once, sampled to a coarse
//! chord tolerance, the planar faces by their edges (for material containment), and the curved
//! faces by what bounds their silhouettes. The planner projects these into any view itself, a
//! linear map: a superset of the drawn ink, so "clear of this" implies "clear of what is drawn".
use super::*;

/// How far a stored chord may stray from its edge, mm. Placement pads every clearance by it.
pub const TOLERANCE: f64 = 0.05;
/// Stored coordinates are rounded to this, mm (it is added to the padding too).
pub const GRID: f64 = 1e-3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeInk {
    /// Exact recognition-independent dimension references, keyed by versioned fingerprint.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub references: BTreeMap<String, Vec<super::dimension_geometry::Reference>>,
    /// Every edge of the part as a polyline in the part's frame, within [`TOLERANCE`].
    pub edges: Vec<Vec<[f64; 3]>>,
    /// Planar faces: their normal and the edges (indices into `edges`) of all their loops.
    pub planes: Vec<PlanarFace>,
    /// Non-planar faces: what bounds their silhouettes in any view.
    pub curved: Vec<CurvedFace>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanarFace {
    pub normal: [f64; 3],
    pub edges: Vec<usize>,
}

/// A curved face's box, and for a cylinder its axis and radius: seen square to its axis its
/// silhouettes are the two generators at ± radius, along the box's sides. Any other curved face
/// (and a cylinder seen obliquely) is a forbidden box in every view.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurvedFace {
    pub bounds: [[f64; 3]; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cylinder: Option<Cylinder>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cylinder {
    pub origin: [f64; 3],
    pub axis: [f64; 3],
    pub radius: f64,
}

fn round(p: [f64; 3]) -> [f64; 3] {
    p.map(|x| (x / GRID).round() * GRID + 0.0)
}

/// Adaptive chord sampling of one edge to [`TOLERANCE`].
fn sample(e: &haecceity::brep::Edge) -> Vec<[f64; 3]> {
    use haecceity::geom::Curve as C;
    if matches!(e.curve, C::Line { .. }) {
        return vec![round(e.start), round(e.end)];
    }
    let (a, b) =
        haecceity::sampling::edge_interval(&e.curve, e.start, e.end, e.same_sense, e.is_closed());
    let n = match e.curve {
        C::Nurbs(_) => 16,
        _ => ((b - a).abs() / std::f64::consts::FRAC_PI_2 * 4.0)
            .ceil()
            .max(2.0) as usize,
    };
    fn refine(
        c: &haecceity::geom::Curve,
        lo: (f64, [f64; 3]),
        hi: (f64, [f64; 3]),
        depth: usize,
        out: &mut Vec<[f64; 3]>,
    ) {
        let tm = 0.5 * (lo.0 + hi.0);
        let pm = c.value(tm);
        let mid = [0, 1, 2].map(|k| 0.5 * (lo.1[k] + hi.1[k]));
        let d = (0..3).map(|k| (pm[k] - mid[k]).powi(2)).sum::<f64>().sqrt();
        if depth < 12 && d > TOLERANCE {
            refine(c, lo, (tm, pm), depth + 1, out);
            refine(c, (tm, pm), hi, depth + 1, out);
        } else {
            out.push(round(hi.1));
        }
    }
    let mut out = vec![round(e.start)];
    let mut last = (a, e.curve.value(a));
    for i in 1..=n {
        let t = a + (b - a) * i as f64 / n as f64;
        let p = (t, e.curve.value(t));
        refine(&e.curve, last, p, 0, &mut out);
        last = p;
    }
    *out.last_mut().expect("two samples") = round(e.end);
    out
}

/// The part's edge evidence. Deterministic: edges and faces in the part's own order.
pub(super) fn evidence(part: &Part) -> EdgeInk {
    use haecceity::geom::Surface;
    let edges = part.edges.iter().map(sample).collect();
    let mut planes = Vec::new();
    let mut curved = Vec::new();
    for (i, face) in part.faces.iter().enumerate() {
        let b = part.face_bounds(i);
        let bounds = [round(b.min), round(b.max)];
        match &face.surface {
            Surface::Plane { frame } => {
                if part.outer_loop(i).is_some() {
                    let mut edges: Vec<usize> = face
                        .loops
                        .iter()
                        .flat_map(|l| l.edges.iter().map(|&(e, _)| e))
                        .collect();
                    edges.sort_unstable();
                    edges.dedup();
                    planes.push(PlanarFace {
                        normal: frame.z,
                        edges,
                    });
                }
            }
            Surface::Cylinder { frame, radius } => curved.push(CurvedFace {
                bounds,
                cylinder: Some(Cylinder {
                    origin: frame.origin,
                    axis: frame.z,
                    radius: *radius,
                }),
            }),
            _ => curved.push(CurvedFace {
                bounds,
                cylinder: None,
            }),
        }
    }
    EdgeInk {
        references: super::dimension_geometry::evidence(part),
        edges,
        planes,
        curved,
    }
}
