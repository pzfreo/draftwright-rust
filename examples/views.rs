//! A part's four views projected by haecceity and written as SVG, DXF and PDF — the exact
//! projected curves carried through (`cargo run --release --example views -- part.step out`).
//! Views are set side by side, unscaled; draftwright's placement comes later.

use draftwright::output::{Drawing, Item, Layer, layer_styles, projected};
use haecceity::hlr::{View, project};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let part = haecceity::read_step_file(std::path::Path::new(&args[1])).unwrap();
    let mut layers: Vec<Layer> = layer_styles()
        .into_iter()
        .take(2)
        .map(|(name, style)| Layer { name: name.into(), style, items: Vec::new() })
        .collect();
    let views = [([0.0, -1.0, 0.0], [0.0, 0.0, 1.0]), ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]), ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]), ([1.0, -1.0, 1.0], [0.0, 0.0, 1.0])];
    let mut x = 20.0;
    for (toward, up) in views {
        let pieces = project(&part, &View::new(toward, up).unwrap());
        let (lo, hi) = pieces.iter().flat_map(|p| p.points.iter()).fold(([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]), |(lo, hi), q| {
            ([lo[0].min(q[0]), lo[1].min(q[1])], [hi[0].max(q[0]), hi[1].max(q[1])])
        });
        let shift = [x - lo[0], 150.0 - 0.5 * (lo[1] + hi[1])];
        for p in &pieces {
            let mut moved = p.clone();
            moved.points.iter_mut().for_each(|q| *q = [q[0] + shift[0], q[1] + shift[1]]);
            moved.exact = p.exact.as_ref().map(|e| shifted(e, shift));
            layers[usize::from(!p.visible)].items.push(Item::Edge(projected(&moved)));
        }
        x += hi[0] - lo[0] + 20.0;
    }
    let drawing = Drawing { page: [x.max(420.0), 297.0], layers, link: None, text: Vec::new() };
    let out = &args[2];
    std::fs::write(format!("{out}.svg"), drawing.svg()).unwrap();
    std::fs::write(format!("{out}.dxf"), drawing.dxf()).unwrap();
    std::fs::write(format!("{out}.pdf"), drawing.pdf(&|_| None)).unwrap();
}

fn shifted(e: &haecceity::hlr::Exact, d: [f64; 2]) -> haecceity::hlr::Exact {
    use haecceity::hlr::Exact;
    let m = |q: [f64; 2]| [q[0] + d[0], q[1] + d[1]];
    match e.clone() {
        Exact::Line(a, b) => Exact::Line(m(a), m(b)),
        Exact::Conic { centre, radii, x_axis, y_axis, t0, t1 } => Exact::Conic { centre: m(centre), radii, x_axis, y_axis, t0, t1 },
        Exact::Nurbs { degree, poles, weights, knots, t0, t1 } => {
            Exact::Nurbs { degree, poles: poles.into_iter().map(m).collect(), weights, knots, t0, t1 }
        }
    }
}
