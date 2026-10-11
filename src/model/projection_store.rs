//! Stable disk representation of kernel projections; no changes to the kernel's API.
use super::*;
use haecceity::hlr::{Class, Exact};

#[derive(Serialize, Deserialize)]
#[serde(remote = "Exact")]
enum ExactDef {
    Line([f64; 2], [f64; 2]),
    Conic {
        centre: [f64; 2],
        radii: [f64; 2],
        x_axis: [f64; 2],
        y_axis: [f64; 2],
        t0: f64,
        t1: f64,
    },
    Nurbs {
        degree: usize,
        poles: Vec<[f64; 2]>,
        weights: Vec<f64>,
        knots: Vec<f64>,
        t0: f64,
        t1: f64,
    },
}
#[derive(Serialize, Deserialize)]
struct ExactValue(#[serde(with = "ExactDef")] Exact);
#[derive(Serialize, Deserialize)]
#[serde(remote = "Class")]
enum ClassDef {
    Sharp,
    Smooth,
    Outline,
}
#[derive(Serialize, Deserialize)]
struct Piece {
    #[serde(with = "ClassDef")]
    class: Class,
    visible: bool,
    points: Vec<[f64; 2]>,
    exact: Option<ExactValue>,
}
#[derive(Serialize, Deserialize)]
struct Stored {
    binding: Option<String>,
    views: BTreeMap<Vec<u64>, Vec<Piece>>,
    cuts: BTreeMap<Vec<u64>, Vec<Vec<[f64; 3]>>>,
    geometry: BTreeMap<Vec<u64>, (Vec<u64>, ViewGeometry)>,
}
// JSON map keys are strings; sorted pairs retain numeric cache keys without hash iteration order.
type CutPairs = Vec<(Vec<u64>, Vec<Vec<[f64; 3]>>)>;
type GeometryPairs = Vec<(Vec<u64>, (Vec<u64>, ViewGeometry))>;
#[derive(Serialize, Deserialize)]
struct Pairs {
    binding: Option<String>,
    views: Vec<(Vec<u64>, Vec<Piece>)>,
    cuts: CutPairs,
    geometry: GeometryPairs,
}
impl Projections {
    pub fn disk_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        let stored = Stored {
            binding: self.binding.clone(),
            views: self
                .views
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.iter()
                            .map(|p| Piece {
                                class: p.class,
                                visible: p.visible,
                                points: p.points.clone(),
                                exact: p.exact.clone().map(ExactValue),
                            })
                            .collect(),
                    )
                })
                .collect(),
            cuts: self.cuts.clone().into_iter().collect(),
            geometry: self.geometry.clone().into_iter().collect(),
        };
        serde_json::to_vec(&Pairs {
            binding: stored.binding,
            views: stored.views.into_iter().collect(),
            cuts: stored.cuts.into_iter().collect(),
            geometry: stored.geometry.into_iter().collect(),
        })
    }
    pub fn from_disk(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        let p: Pairs = serde_json::from_slice(bytes)?;
        Ok(Self {
            binding: p.binding,
            views: p
                .views
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        v.into_iter()
                            .map(|p| Projected {
                                class: p.class,
                                visible: p.visible,
                                points: p.points,
                                exact: p.exact.map(|e| e.0),
                            })
                            .collect(),
                    )
                })
                .collect(),
            cuts: p.cuts.into_iter().collect(),
            geometry: p.geometry.into_iter().collect(),
            stats: ProjectionStats::default(),
        })
    }
}
