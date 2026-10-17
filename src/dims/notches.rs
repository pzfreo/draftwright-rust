//! A V is specified by included angle, theoretical sharp depth, and mouth position.
use super::{AXES, Context, Measurement, Planner, Problem, Shape, Want, end_on, fmt, want_id};

pub(super) const PLANNER: Planner = Planner { kinds: &[], wants };
fn wants(cx: &Context) -> (Vec<Want>, Vec<Problem>) {
    let mut out = Vec::new();
    for n in crate::notches::requirements(cx.analysis) {
        let view = end_on(n.axis);
        let value = n.angle.degrees().unwrap();
        out.push(Want {
            id: want_id(&n.id,"notch.angle"), views:vec![view], value,
            terms:vec![format!("{}°",fmt(value))],
            measurements:vec![Measurement {feature:n.id.clone(),parameter:"notch.angle".into(),value}],
            basis:"a V-notch's included angle, in the normal view that shows it true; rounded roots extend to the theoretical sharp apex".into(),
            shape:Shape::Angular {angle:n.angle.clone()}, turning:None,
        });
        let k = n.depth_axis;
        let value = n.depth();
        out.push(Want {
            id:want_id(&n.id,"notch_depth.length"),views:vec![view],value,terms:vec![fmt(value)],
            measurements:vec![Measurement {feature:n.id.clone(),parameter:"notch_depth.length".into(),value}],
            basis:"V-notch depth from the mouth to the theoretical sharp apex; the included angle and depth define the mouth width".into(),
            shape:Shape::Linear {a:n.mouth,b:n.angle.vertex,axis:k,a_edge:false,b_edge:false,order:0},turning:None,
        });
        // Locate the mouth centre from the part's minimum datums; consolidation shares
        // coincident stations with existing dimensions and schemes can take ownership.
        for k in [n.width_axis, n.depth_axis] {
            let b = n.mouth;
            let mut a = b;
            a[k] = cx.analysis.envelope[0][k];
            let value = (b[k] - a[k]).abs();
            if value < 1e-6 {
                continue;
            }
            let parameter = format!("location.length.{}", AXES[k]);
            out.push(Want {
                id: want_id(&n.id, &parameter),
                views: vec![view],
                value,
                terms: vec![fmt(value)],
                measurements: vec![Measurement {
                    feature: n.id.clone(),
                    parameter,
                    value,
                }],
                basis: "the V-notch mouth centre located from the part's minimum datum".into(),
                shape: Shape::Linear {
                    a,
                    b,
                    axis: k,
                    a_edge: true,
                    b_edge: false,
                    order: 1,
                },
                turning: None,
            });
        }
    }
    // Standalone oblique bevels are located by their virtual corner. An exact shared edge
    // with a dimensioned polygonal host already locates its treatment.
    for f in cx
        .analysis
        .features
        .iter()
        .filter(|f| f.kind == "oriented_chamfer")
    {
        let Ok(r) = crate::oriented_chamfers::read(cx.analysis, f) else {
            continue;
        };
        let bevel_edges: Vec<_> = cx
            .analysis
            .outer_profiles
            .iter()
            .filter(|p| f.faces.contains(&p.face))
            .flat_map(|p| &p.edges)
            .collect();
        let hosted = cx
            .analysis
            .features
            .iter()
            .filter(|p| matches!(p.kind.as_str(), "polygonal_boss" | "polygonal_stock"))
            .any(|host| {
                cx.analysis
                    .outer_profiles
                    .iter()
                    .filter(|p| host.faces.contains(&p.face))
                    .any(|p| p.edges.iter().any(|e| bevel_edges.contains(&e)))
            });
        if hosted {
            continue;
        }
        for k in (0..3).filter(|&k| k != r.view_axis) {
            let b = r.corner;
            let mut a = b;
            a[k] = cx.analysis.envelope[0][k];
            let value = (b[k] - a[k]).abs();
            if value < 1e-6 {
                continue;
            }
            let parameter = format!("location.length.{}", AXES[k]);
            out.push(Want {id:want_id(&f.id,&parameter),views:vec![end_on(r.view_axis)],value,terms:vec![fmt(value)],
                measurements:vec![Measurement{feature:f.id.clone(),parameter,value}],
                basis:"the oblique bevel's virtual sharp corner located from the minimum datum in its carrying view".into(),
                shape:Shape::Linear{a,b,axis:k,a_edge:true,b_edge:false,order:1},turning:None});
        }
    }
    (out, vec![])
}
