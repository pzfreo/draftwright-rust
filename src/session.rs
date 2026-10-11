//! A live drawing over the same Spec edits and recovered planner as the batch API.
use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{self, Error, Features};
use crate::haecceity::Part;
use crate::items::{self, Op, State as ItemState};
use crate::model::{
    self, Analysis, AnnotationItem, GeometryLayer, Plan, Projections, RenderedItem, Spec,
};
use crate::output::Drawing;
use crate::placement_intent::{Along, Place, Side, Slot};
use crate::text::Font;

const COLLECTIONS: [&str; 9] = [
    "views",
    "dimensions",
    "omitted",
    "gdt",
    "requirements",
    "notes",
    "tables",
    "balloons",
    "problems",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "geometry", rename_all = "snake_case")]
pub enum Placement {
    Dimension(crate::dims::Placed),
    Annotation(crate::dims::gdt::GdtPlaced),
    View {
        at: [f64; 2],
        area: [f64; 4],
        scale: f64,
    },
    Box([f64; 4]),
}

/// A compact answer; numbers and page geometry are resolved outputs, never Spec inputs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemAnswer {
    pub item: items::Item,
    pub value: Option<f64>,
    pub dimension_kind: Option<String>,
    pub placement: Option<Placement>,
    pub anchors: Vec<String>,
    pub features: Vec<String>,
    pub basis: String,
    pub operations: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ItemFilter {
    pub view: Option<String>,
    pub kind: Option<String>,
    pub state: Option<ItemState>,
    pub feature: Option<String>,
    /// Page millimetres, [left, bottom, right, top].
    pub region: Option<[f64; 4]>,
    pub near: Option<String>,
    /// Page millimetres around the near item's box; default 10.
    pub radius: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProblemFilter {
    pub code: Option<String>,
    pub severity: Option<String>,
    pub subject: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Options {
    pub id: String,
    pub operations: Vec<String>,
    /// Conservative, valid placements on the current carrier. Collisions remain problems.
    pub placements: Vec<Place>,
    pub view_anchors: Vec<String>,
    pub scales: Vec<String>,
}

/// An ordered collection's changed records, keyed by stable ID. An absent order keeps it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CollectionDelta {
    pub set: BTreeMap<String, Value>,
    pub removed: Vec<String>,
    pub order: Option<Vec<String>>,
}

/// A lossless patch of the Plan, including non-item furniture and collection order.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlanDelta {
    pub fields: BTreeMap<String, Value>,
    pub collections: BTreeMap<String, CollectionDelta>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delta {
    /// Complete recovered-planner invocations used for this edit, including a preview.
    pub plans: usize,
    pub changed: Vec<ItemAnswer>,
    pub added: Vec<ItemAnswer>,
    pub removed: Vec<String>,
    pub problems_added: Vec<model::Problem>,
    pub problems_resolved: Vec<model::Problem>,
    pub plan: PlanDelta,
    /// Only changed view geometry and annotation ink. Removed IDs use `removed`.
    pub geometry: GeometryLayer,
    pub annotations: BTreeMap<String, RenderedItem>,
    pub geometry_order: Option<Vec<String>>,
    pub problem_order: Option<Vec<String>>,
}

/// The client-replayable state. Its render reuses the supplied geometry and annotation ink.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub plan: Plan,
    pub geometry: GeometryLayer,
    pub annotations: BTreeMap<String, RenderedItem>,
    pub problems: Vec<model::Problem>,
}
impl State {
    pub fn render(
        &self,
        part: &Part,
        analysis: &Analysis,
        font: &Font,
        cache: &mut Projections,
    ) -> Result<Drawing, Error> {
        model::render_annotations_with_items(
            part,
            analysis,
            &self.plan,
            font,
            self.geometry.clone(),
            cache,
            &self.annotations,
        )
        .map_err(|e| Error::failed(format!("cannot render session: {e}")))
    }
}

fn document(p: &Plan) -> Value {
    serde_json::to_value(p).expect("a plan serialises")
}
fn records(v: &Value) -> BTreeMap<String, Value> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|v| {
            (
                v["id"].as_str().expect("a planned record has an ID").into(),
                v.clone(),
            )
        })
        .collect()
}
fn order(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|v| v["id"].as_str().unwrap().into())
        .collect()
}
fn patch(old: &Plan, new: &Plan) -> PlanDelta {
    let (a, b) = (document(old), document(new));
    let mut delta = PlanDelta::default();
    for name in COLLECTIONS {
        if a[name] == b[name] {
            continue;
        }
        let (old, new) = (records(&a[name]), records(&b[name]));
        delta.collections.insert(
            name.into(),
            CollectionDelta {
                set: new
                    .iter()
                    .filter(|(id, v)| old.get(*id) != Some(*v))
                    .map(|(id, v)| (id.clone(), v.clone()))
                    .collect(),
                removed: old
                    .keys()
                    .filter(|id| !new.contains_key(*id))
                    .cloned()
                    .collect(),
                order: (order(&a[name]) != order(&b[name])).then(|| order(&b[name])),
            },
        );
    }
    for name in a
        .as_object()
        .unwrap()
        .keys()
        .chain(b.as_object().unwrap().keys())
    {
        if !COLLECTIONS.contains(&name.as_str()) && a[name] != b[name] {
            delta.fields.insert(name.clone(), b[name].clone());
        }
    }
    delta
}
impl Delta {
    /// Applies a server delta without planning or formatting an annotation.
    pub fn apply_to(&self, state: &mut State) -> Result<(), Error> {
        let mut doc = document(&state.plan);
        for (name, value) in &self.plan.fields {
            if value.is_null() {
                doc.as_object_mut().unwrap().remove(name);
            } else {
                doc[name] = value.clone();
            }
        }
        for (name, delta) in &self.plan.collections {
            let mut entries = records(&doc[name]);
            let ids = delta.order.clone().unwrap_or_else(|| order(&doc[name]));
            for id in &delta.removed {
                entries.remove(id);
            }
            entries.extend(delta.set.clone());
            doc[name] = Value::Array(ids.iter().filter_map(|id| entries.remove(id)).collect());
            if !entries.is_empty() {
                return Err(Error::invalid("delta has records missing from its order"));
            }
        }
        let plan = serde_json::from_value(doc)
            .map_err(|e| Error::invalid(format!("invalid plan delta: {e}")))?;
        let mut geometry: BTreeMap<_, _> = state.geometry.views.iter().cloned().collect();
        let ids = self.geometry_order.clone().unwrap_or_else(|| {
            state
                .geometry
                .views
                .iter()
                .map(|(id, _)| id.clone())
                .collect()
        });
        for id in &self.removed {
            geometry.remove(id);
        }
        geometry.extend(self.geometry.views.iter().cloned());
        let views = ids
            .into_iter()
            .map(|id| {
                geometry
                    .remove(&id)
                    .map(|v| (id, v))
                    .ok_or_else(|| Error::invalid("delta is missing view geometry"))
            })
            .collect::<Result<_, _>>()?;
        let mut annotations = state.annotations.clone();
        // A shown-to-omitted transition keeps the item ID but removes its ink.
        for id in self.removed.iter().chain(
            self.changed
                .iter()
                .filter(|i| {
                    !self.annotations.contains_key(&i.item.id)
                        && !matches!(
                            i.placement,
                            Some(Placement::Dimension(_) | Placement::Annotation(_))
                        )
                })
                .map(|i| &i.item.id),
        ) {
            annotations.remove(id);
        }
        annotations.extend(self.annotations.clone());
        let mut problems = state.problems.clone();
        problems.retain(|p| !self.problems_resolved.contains(p));
        problems.extend(self.problems_added.clone());
        // Plan problems keep planner order; render-only problems follow them.
        let plan: Plan = plan;
        problems.sort_by_key(|p| {
            if let Some(order) = &self.problem_order {
                return order
                    .iter()
                    .position(|id| id == &p.id)
                    .unwrap_or(usize::MAX);
            }
            plan.problems
                .iter()
                .position(|q| q == p)
                .unwrap_or(usize::MAX)
        });
        state.plan = plan;
        state.geometry.views = views;
        state.annotations = annotations;
        state.problems = problems;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Stats {
    /// Complete recovered-planner invocations, excluding previews and queries.
    pub plans: usize,
    pub projections: usize,
    pub bounds: usize,
    pub annotations: usize,
}

#[derive(Clone, Default)]
struct Planner {
    calls: usize,
}
impl Planner {
    fn plan(&mut self, analysis: &Analysis, spec: &Spec) -> Result<Plan, Error> {
        self.calls += 1;
        Ok(model::plan_recovered(analysis, spec)?)
    }
}
struct Evaluation {
    state: State,
    drawing: Drawing,
    projections: Projections,
    planner: Planner,
    rendered: usize,
}

pub struct Session {
    part: Part,
    analysis: Analysis,
    state: State,
    drawing: Drawing,
    font: Font,
    projections: Projections,
    undo: Vec<Spec>,
    redo: Vec<Spec>,
    stats: Stats,
    planner: Planner,
}
impl Session {
    pub fn open(step: &Path, features: Features<'_>, spec: Spec) -> Result<Self, Error> {
        let (part, analysis) = api::analysed(step, features)?;
        Self::new(part, analysis, spec)
    }
    pub fn new(part: Part, analysis: Analysis, spec: Spec) -> Result<Self, Error> {
        let font = Font::new(api::MONO.to_vec()).expect("the bundled font parses");
        let mut planner = Planner::default();
        let plan = planner.plan(&analysis, &spec)?;
        let mut projections = Projections::default();
        let geometry = model::render_geometry(&part, &analysis, &plan, &mut projections)
            .map_err(|e| Error::failed(format!("cannot render session: {e}")))?;
        let annotations = ink(&plan, None);
        let mut state = State {
            problems: plan.problems.clone(),
            plan,
            geometry,
            annotations,
        };
        let drawing = state.render(&part, &analysis, &font, &mut projections)?;
        state.problems.extend(drawing.problems.clone());
        let stats = Stats {
            annotations: state.annotations.len(),
            ..Default::default()
        };
        Ok(Self {
            part,
            analysis,
            state,
            drawing,
            font,
            projections,
            undo: Vec::new(),
            redo: Vec::new(),
            stats,
            planner,
        })
    }
    pub fn part(&self) -> &Part {
        &self.part
    }
    pub fn analysis(&self) -> &Analysis {
        &self.analysis
    }
    pub fn spec(&self) -> &Spec {
        &self.state.plan.spec
    }
    pub fn plan(&self) -> &Plan {
        &self.state.plan
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn drawing(&self) -> &Drawing {
        &self.drawing
    }
    pub fn stats(&self) -> Stats {
        let p = self.projections.stats();
        Stats {
            plans: self.planner.calls,
            projections: p.projections,
            bounds: p.bounds,
            ..self.stats
        }
    }
    /// Validation, one complete plan, then changed annotation ink; commit only after success.
    pub fn apply(&mut self, ops: &[Op], commit: bool) -> Result<Delta, Error> {
        let spec = model::edit(
            self.spec(),
            &serde_json::to_value(ops).expect("ops serialise"),
        )?;
        let evaluated = self.evaluate(&spec)?;
        let delta = difference(
            &self.state,
            &evaluated.state,
            &self.analysis,
            evaluated.planner.calls - self.planner.calls,
        );
        if commit {
            self.undo.push(self.spec().clone());
            self.redo.clear();
            self.install(evaluated);
        }
        Ok(delta)
    }
    pub fn undo(&mut self) -> Result<Delta, Error> {
        self.history(false)
    }
    pub fn redo(&mut self) -> Result<Delta, Error> {
        self.history(true)
    }
    fn history(&mut self, redo: bool) -> Result<Delta, Error> {
        let spec = if redo {
            self.redo.last()
        } else {
            self.undo.last()
        }
        .ok_or_else(|| {
            Error::invalid(if redo {
                "nothing to redo"
            } else {
                "nothing to undo"
            })
        })?
        .clone();
        let evaluated = self.evaluate(&spec)?;
        let delta = difference(
            &self.state,
            &evaluated.state,
            &self.analysis,
            evaluated.planner.calls - self.planner.calls,
        );
        let current = self.spec().clone();
        if redo {
            self.redo.pop();
            self.undo.push(current);
        } else {
            self.undo.pop();
            self.redo.push(current);
        }
        self.install(evaluated);
        Ok(delta)
    }
    fn evaluate(&self, spec: &Spec) -> Result<Evaluation, Error> {
        let mut planner = self.planner.clone();
        let plan = planner.plan(&self.analysis, spec)?;
        let mut cache = self.projections.clone();
        let geometry = model::render_geometry(&self.part, &self.analysis, &plan, &mut cache)
            .map_err(|e| Error::failed(format!("cannot render session: {e}")))?;
        let annotations = ink(&plan, Some(&self.state));
        let rendered = annotations
            .keys()
            .filter(|id| annotation_changed(&self.state.plan, &plan, id))
            .count();
        let mut state = State {
            problems: plan.problems.clone(),
            plan,
            geometry,
            annotations,
        };
        let drawing = state.render(&self.part, &self.analysis, &self.font, &mut cache)?;
        state.problems.extend(drawing.problems.clone());
        Ok(Evaluation {
            state,
            drawing,
            projections: cache,
            planner,
            rendered,
        })
    }
    fn install(&mut self, evaluated: Evaluation) {
        self.state = evaluated.state;
        self.drawing = evaluated.drawing;
        self.projections = evaluated.projections;
        self.planner = evaluated.planner;
        self.stats.annotations += evaluated.rendered;
    }
    pub fn item(&self, id: &str) -> Result<ItemAnswer, Error> {
        answers(self.plan(), &self.analysis)
            .into_iter()
            .find(|i| i.item.id == id)
            .ok_or_else(|| {
                let mut choices: Vec<_> = items::items(self.plan())
                    .into_iter()
                    .map(|i| i.id)
                    .collect();
                choices
                    .sort_by_key(|choice| (crate::dims::levenshtein(id, choice), choice.clone()));
                choices.truncate(5);
                Error {
                    choices,
                    ..Error::invalid(format!("unknown item {id:?}"))
                }
            })
    }
    pub fn items(&self, filter: &ItemFilter) -> Vec<ItemAnswer> {
        let near = filter
            .near
            .as_ref()
            .and_then(|id| self.item(id).ok())
            .and_then(|i| bbox(&i));
        let region = filter.region.or_else(|| {
            near.map(|[a, b, c, d]| {
                let r = filter.radius.unwrap_or(10.0).max(0.0);
                [a - r, b - r, c + r, d + r]
            })
        });
        answers(self.plan(), &self.analysis)
            .into_iter()
            .filter(|i| {
                filter.view.as_ref().is_none_or(|v| {
                    i.item
                        .view
                        .as_ref()
                        .is_some_and(|w| w.trim_start_matches("v:") == v.trim_start_matches("v:"))
                }) && filter
                    .kind
                    .as_ref()
                    .is_none_or(|k| &i.item.kind == k || i.dimension_kind.as_ref() == Some(k))
                    && filter.state.as_ref().is_none_or(|s| &i.item.state == s)
                    && filter
                        .feature
                        .as_ref()
                        .is_none_or(|f| i.features.contains(f))
                    && (filter.near.is_none() || near.is_some())
                    && region.is_none_or(|r| {
                        bbox(i).is_some_and(|b| {
                            b[0] <= r[2] && b[2] >= r[0] && b[1] <= r[3] && b[3] >= r[1]
                        })
                    })
            })
            .collect()
    }
    pub fn problems(&self, filter: &ProblemFilter) -> Vec<model::Problem> {
        self.state
            .problems
            .iter()
            .filter(|p| {
                filter.code.as_ref().is_none_or(|c| &p.code == c)
                    && filter.severity.as_ref().is_none_or(|s| &p.severity == s)
                    && filter
                        .subject
                        .as_ref()
                        .is_none_or(|s| p.subjects.contains(s))
            })
            .cloned()
            .collect()
    }
    pub fn options(&self, id: &str) -> Result<Options, Error> {
        let item = self.item(id)?;
        let placements = if item.operations.contains(&"place".into()) && item.item.kind != "view" {
            let view = crate::placement_intent::carrier(self.plan(), id);
            [Side::Above, Side::Below, Side::Left, Side::Right]
                .into_iter()
                .filter_map(|side| {
                    view.as_ref().map(|view| Place {
                        view: Some(view.trim_start_matches("v:").into()),
                        side: Some(side),
                        lane: None,
                        along: Some(Along::Slot(Slot::Middle)),
                        pinned: false,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(Options {
            id: id.into(),
            operations: item.operations,
            placements,
            view_anchors: if item.item.kind == "view" {
                self.plan()
                    .views
                    .iter()
                    .filter(|v| v.id != id && !depends_on(self.spec(), &v.id, id))
                    .map(|v| v.id.clone())
                    .collect()
            } else {
                Vec::new()
            },
            scales: if id == "sheet" {
                crate::layout::SCALES.map(model::scale_name).to_vec()
            } else {
                Vec::new()
            },
        })
    }
}
fn depends_on(spec: &Spec, view: &str, id: &str) -> bool {
    let mut at = view.trim_start_matches("v:");
    for _ in 0..spec.views.len() {
        let Some(p) = spec.views.get(at).and_then(|v| v.place.as_ref()) else {
            return false;
        };
        if p.relative_to == id.trim_start_matches("v:") {
            return true;
        }
        at = &p.relative_to;
    }
    false
}
fn annotation_changed(old: &Plan, new: &Plan, id: &str) -> bool {
    old.dimensions
        .iter()
        .find(|d| d.id == id)
        .map(|d| (&d.view, &d.kind, &d.terms, &d.measurements, &d.placed))
        != new
            .dimensions
            .iter()
            .find(|d| d.id == id)
            .map(|d| (&d.view, &d.kind, &d.terms, &d.measurements, &d.placed))
        || old
            .gdt
            .iter()
            .find(|d| d.id == id)
            .map(|d| (&d.view, &d.cells, &d.placed))
            != new
                .gdt
                .iter()
                .find(|d| d.id == id)
                .map(|d| (&d.view, &d.cells, &d.placed))
}
fn ink(plan: &Plan, old: Option<&State>) -> BTreeMap<String, RenderedItem> {
    plan.dimensions
        .iter()
        .map(|d| (&d.id, AnnotationItem::Dimension(d)))
        .chain(plan.gdt.iter().map(|g| (&g.id, AnnotationItem::Gdt(g))))
        .map(|(id, item)| {
            let kept = old
                .filter(|s| !annotation_changed(&s.plan, plan, id))
                .and_then(|s| s.annotations.get(id))
                .cloned();
            (id.clone(), kept.unwrap_or_else(|| model::render_item(item)))
        })
        .collect()
}
fn answers(plan: &Plan, analysis: &Analysis) -> Vec<ItemAnswer> {
    items::items(plan)
        .into_iter()
        .map(|item| {
            let id = &item.id;
            let mut a = ItemAnswer {
                value: None,
                dimension_kind: None,
                placement: None,
                anchors: Vec::new(),
                features: Vec::new(),
                basis: String::new(),
                operations: Vec::new(),
                item: item.clone(),
            };
            if id == "sheet" {
                a.basis = plan.sheet.basis.clone();
                a.operations = ["size", "scale", "title"].map(String::from).into();
            }
            if let Some(v) = plan.views.iter().find(|v| &v.id == id) {
                a.placement = Some(Placement::View {
                    at: v.at,
                    area: v.area,
                    scale: v.scale,
                });
                a.basis = v.basis.clone();
                a.operations = ["omit", "place", "pin"].map(String::from).into();
                a.anchors = plan
                    .spec
                    .views
                    .get(id.trim_start_matches("v:"))
                    .and_then(|v| v.place.as_ref())
                    .map(|p| vec![format!("v:{}", p.relative_to)])
                    .unwrap_or_default();
            }
            if let Some(d) = plan.dimensions.iter().find(|d| &d.id == id) {
                a.value = Some(d.value);
                a.dimension_kind = Some(d.kind.clone());
                a.placement = d.placed.clone().map(Placement::Dimension);
                a.basis = d.basis.clone();
                a.features = d.measurements.iter().map(|m| m.feature.clone()).collect();
                a.features.sort();
                a.features.dedup();
                a.anchors = vec![d.view.clone()];
                a.anchors.extend(a.features.clone());
                a.operations = ["omit", "tolerance", "place", "pin"]
                    .map(String::from)
                    .into();
            }
            if let Some(o) = plan.omitted.iter().find(|o| &o.id == id) {
                a.basis = o.basis.clone();
                if let Some((feature, _)) = id.strip_prefix("d:").and_then(|s| s.rsplit_once('/')) {
                    a.features.push(feature.into());
                }
                a.operations = vec!["omit".into()];
                if let ItemState::Covered { by } = &o.state {
                    a.anchors.push(by.clone());
                }
            }
            if let Some(g) = plan.gdt.iter().find(|g| &g.id == id) {
                a.basis = g.basis.clone();
                a.placement = g.placed.clone().map(Placement::Annotation);
                a.anchors = vec![g.view.clone()];
                a.anchors
                    .extend(g.faces.iter().map(|face| format!("face:{face}")));
                a.features = analysis
                    .features
                    .iter()
                    .filter(|f| f.faces.iter().any(|face| g.faces.contains(face)))
                    .map(|f| f.id.clone())
                    .collect();
                a.operations = ["omit", "place", "pin"].map(String::from).into();
            }
            if let Some(n) = plan.notes.iter().find(|n| &n.id == id) {
                a.basis = n.basis.clone();
                a.placement = plan.notes_block.as_ref().map(|b| Placement::Box(b.cells));
            }
            if let Some(r) = plan.requirements.iter().find(|r| &r.id == id) {
                a.basis = r.basis.clone();
                a.anchors = vec!["sheet".into()];
                if let Some(p) = &r.placed {
                    a.placement = Some(Placement::Box(p.glyph));
                }
            }
            if let Some(t) = plan.tables.iter().find(|t| &t.id == id) {
                a.basis = t.basis.clone();
                a.placement = t.placed.as_ref().map(|p| Placement::Box(p.cells));
            }
            if let Some(t) = plan.hole_table.as_ref().filter(|t| &t.id == id) {
                a.basis = t.basis.clone();
                a.features = t.rows.iter().map(|r| r.feature.clone()).collect();
                a.anchors = vec!["v:plan".into()];
            }
            if let Some(b) = plan.balloons.iter().find(|b| &b.id == id) {
                a.features = b.features.clone();
                a.anchors = vec![b.view.clone()];
                a.placement = Some(Placement::Box([
                    b.at[0] - b.radius,
                    b.at[1] - b.radius,
                    b.at[0] + b.radius,
                    b.at[1] + b.radius,
                ]));
            }
            a
        })
        .collect()
}
fn expand_box<'a>(b: [f64; 4], points: impl Iterator<Item = &'a [f64; 2]>) -> [f64; 4] {
    points.fold(b, |b, p| {
        [
            b[0].min(p[0]),
            b[1].min(p[1]),
            b[2].max(p[0]),
            b[3].max(p[1]),
        ]
    })
}
fn bbox(i: &ItemAnswer) -> Option<[f64; 4]> {
    match i.placement.as_ref()? {
        Placement::Dimension(p) => Some(expand_box(
            p.label_box,
            p.lines.iter().chain(&p.arrows).flatten(),
        )),
        Placement::Annotation(p) => Some(expand_box(
            p.label_box,
            p.lines.iter().chain(&p.arrows).flatten(),
        )),
        Placement::View { area, .. } | Placement::Box(area) => Some(*area),
    }
}
fn item_records(plan: &Plan) -> BTreeMap<String, Vec<Value>> {
    let doc = document(plan);
    let mut out = BTreeMap::<String, Vec<Value>>::new();
    out.insert("sheet".into(), vec![doc["sheet"].clone()]);
    for name in COLLECTIONS.into_iter().filter(|name| *name != "problems") {
        for (id, record) in records(&doc[name]) {
            out.entry(id).or_default().push(record);
        }
    }
    if let Some(id) = doc["hole_table"]["id"].as_str() {
        out.entry(id.into())
            .or_default()
            .push(doc["hole_table"].clone());
    }
    out
}
fn difference(old: &State, new: &State, analysis: &Analysis, plans: usize) -> Delta {
    let before: BTreeMap<_, _> = answers(&old.plan, analysis)
        .into_iter()
        .map(|i| (i.item.id.clone(), i))
        .collect();
    let after = answers(&new.plan, analysis);
    let old_records = item_records(&old.plan);
    let new_records = item_records(&new.plan);
    let new_ids: Vec<_> = after.iter().map(|i| i.item.id.clone()).collect();
    let changed: Vec<_> = after
        .iter()
        .filter(|i| {
            before.get(&i.item.id).is_some_and(|b| {
                old_records.get(&i.item.id) != new_records.get(&i.item.id)
                    || serde_json::to_value(b).unwrap() != serde_json::to_value(i).unwrap()
            })
        })
        .cloned()
        .collect();
    let geometry = GeometryLayer {
        views: new
            .geometry
            .views
            .iter()
            .filter(|(id, _)| {
                let a = old.plan.views.iter().find(|v| &v.id == id);
                let b = new
                    .plan
                    .views
                    .iter()
                    .find(|v| &v.id == id)
                    .expect("geometry has a planned view");
                a.is_none_or(|a| {
                    a.at != b.at
                        || a.scale != b.scale
                        || a.direction != b.direction
                        || a.up != b.up
                        || a.cut != b.cut
                })
            })
            .cloned()
            .collect(),
    };
    let view_ids = |s: &State| {
        s.geometry
            .views
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>()
    };
    Delta {
        plans,
        changed,
        added: after
            .into_iter()
            .filter(|i| !before.contains_key(&i.item.id))
            .collect(),
        removed: before
            .keys()
            .filter(|id| !new_ids.contains(id))
            .cloned()
            .collect(),
        problems_added: new
            .problems
            .iter()
            .filter(|p| !old.problems.contains(p))
            .cloned()
            .collect(),
        problems_resolved: old
            .problems
            .iter()
            .filter(|p| !new.problems.contains(p))
            .cloned()
            .collect(),
        plan: patch(&old.plan, &new.plan),
        geometry,
        annotations: new
            .annotations
            .iter()
            .filter(|(id, _)| annotation_changed(&old.plan, &new.plan, id))
            .map(|(id, v)| (id.clone(), v.clone()))
            .collect(),
        geometry_order: (view_ids(old) != view_ids(new)).then(|| view_ids(new)),
        problem_order: (old.problems.iter().map(|p| &p.id).collect::<Vec<_>>()
            != new.problems.iter().map(|p| &p.id).collect::<Vec<_>>())
        .then(|| new.problems.iter().map(|p| p.id.clone()).collect()),
    }
}
