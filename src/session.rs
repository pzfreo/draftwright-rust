//! A live drawing over the same Spec edits and recovered planner as the batch API.
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[cfg(not(target_arch = "wasm32"))]
use crate::api::Features;
use crate::api::{self, Error};
use crate::haecceity::Part;
use crate::items::{self, Op, State as ItemState};
use crate::model::{
    self, Analysis, AnnotationItem, GeometryLayer, Plan, Projections, RenderedItem, Spec,
};
use crate::output::Drawing;
use crate::placement_intent::{Along, Place, Slot};
use crate::text::Font;

const COLLECTIONS: [&str; 12] = [
    "sheets",
    "views",
    "captions",
    "dimensions",
    "omitted",
    "gdt",
    "requirements",
    "notes",
    "tables",
    "hole_tables",
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<u32>,
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
    pub feature: Option<String>,
}

/// Where a callout's bend sits relative to its automatic leader tip, in paper mm (y up): the
/// origin of a pinned `place.offset` (#12). `offset` is `bend - tip`; pinning it reproduces
/// the bend. To pin a dragged bend `b`, set `place` to `{offset: b - tip, pinned: true}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalloutOffset {
    /// The view the automatic tip is in, `v:plan` etc.
    pub view: String,
    /// The automatic leader tip.
    pub tip: [f64; 2],
    /// The callout's bend as drawn now.
    pub bend: [f64; 2],
    pub offset: [f64; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Options {
    pub id: String,
    pub operations: Vec<String>,
    /// Conservative, valid placements on the current carrier. Collisions remain problems.
    pub placements: Vec<Place>,
    pub view_anchors: Vec<String>,
    pub scales: Vec<String>,
    /// Escaped Spec paths and accepted values/forms; candidates require preview/apply.
    pub settable: BTreeMap<String, Value>,
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
        self.render_sheet(part, analysis, font, cache, 1)
    }
    pub fn render_sheet(
        &self,
        part: &Part,
        analysis: &Analysis,
        font: &Font,
        cache: &mut Projections,
        sheet: u32,
    ) -> Result<Drawing, Error> {
        self.check_sheet(sheet)?;
        model::render_annotations_on_sheet(
            part,
            analysis,
            &self.plan,
            font,
            self.geometry.clone(),
            cache,
            &self.annotations,
            sheet,
        )
        .map_err(|e| Error::failed(format!("cannot render session: {e}")))
    }
    pub fn check_sheet(&self, sheet: u32) -> Result<(), Error> {
        if sheet == 1 || self.plan.sheets.iter().any(|s| s.number == sheet) {
            return Ok(());
        }
        Err(Error {
            choices: std::iter::once(&self.plan.sheet)
                .chain(&self.plan.sheets)
                .map(|s| s.number.to_string())
                .collect(),
            ..Error::invalid(format!("there is no sheet {sheet}"))
        })
    }
}

fn document(p: &Plan) -> Value {
    serde_json::to_value(p).expect("a plan serialises")
}
fn record_id(v: &Value) -> String {
    v["id"].as_str().map(String::from).unwrap_or_else(|| {
        format!(
            "sheet:{}",
            v["number"].as_u64().expect("a sheet has a number")
        )
    })
}
fn records(v: &Value) -> BTreeMap<String, Value> {
    v.as_array()
        .into_iter()
        .flatten()
        .map(|v| (record_id(v), v.clone()))
        .collect()
}
fn order(v: &Value) -> Vec<String> {
    v.as_array().into_iter().flatten().map(record_id).collect()
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
        let plan: Plan = serde_json::from_value(doc)
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
                    // Covered dimensions retain an empty fragment in State; a changed note
                    // owner must not discard it when its geometry is still unchanged.
                    !self.annotations.contains_key(&i.item.id)
                        && !plan.dimensions.iter().any(|d| d.id == i.item.id)
                        && !plan.gdt.iter().any(|g| g.id == i.item.id)
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
    /// Actual recovered sheet-1 runs, summed across committed planner invocations.
    pub sheet_steps: usize,
    pub projections: usize,
    pub bounds: usize,
    pub annotations: usize,
}

#[derive(Clone)]
struct Planner {
    calls: usize,
    sheet_steps: usize,
    main: std::sync::Arc<crate::planning::BoundedMemo<Plan>>,
    seed: std::sync::Arc<crate::planning::BoundedMemo<Plan>>,
}
impl Default for Planner {
    fn default() -> Self {
        Self {
            calls: 0,
            sheet_steps: 0,
            main: std::sync::Arc::new(crate::planning::BoundedMemo::new(32)),
            seed: std::sync::Arc::new(crate::planning::BoundedMemo::new(16)),
        }
    }
}
impl Planner {
    fn plan(&mut self, analysis: &Analysis, spec: &Spec) -> Result<Plan, Error> {
        self.calls += 1;
        Ok(crate::planning::with_session_main(
            self.main.clone(),
            self.seed.clone(),
            || {
                crate::planning::with_retained(analysis, || {
                    crate::sheets::allocate_counted(analysis, spec, &mut self.sheet_steps)
                })
            },
        )?)
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
    render_memo: crate::planning::BoundedMemo<Drawing>,
}
impl Session {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(step: &Path, features: Features<'_>, spec: Spec) -> Result<Self, Error> {
        Self::open_part(step, None, features, spec)
    }
    /// [`Session::open`] of part *part* of an assembly alone, when given
    /// ([`crate::model::analyse_step_part`]).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_part(
        step: &Path,
        part: Option<usize>,
        features: Features<'_>,
        spec: Spec,
    ) -> Result<Self, Error> {
        let (part, analysis) = api::analysed(step, part, features)?;
        Self::new(part, analysis, spec)
    }
    pub fn new(part: Part, analysis: Analysis, spec: Spec) -> Result<Self, Error> {
        Self::with_projections(part, analysis, spec, Projections::default())
    }
    /// Reconstruct a session from immutable documents and a persisted projection cache.
    pub fn with_projections(
        part: Part,
        analysis: Analysis,
        spec: Spec,
        mut projections: Projections,
    ) -> Result<Self, Error> {
        let font = Font::new(api::MONO.to_vec()).expect("the bundled font parses");
        let mut planner = Planner::default();
        let plan = planner.plan(&analysis, &spec)?;
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
            render_memo: crate::planning::BoundedMemo::new(1),
        })
    }
    pub fn projections(&self) -> &Projections {
        &self.projections
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
    /// Exterior elbow rows for pinning a callout in each compatible principal view.
    /// Reads the current frames and canonical widths without planning or projecting.
    pub fn label_rows(
        &self,
        address: &str,
    ) -> Result<BTreeMap<String, crate::dims::LabelRows>, Error> {
        let item = self.item(address)?;
        if item.dimension_kind.as_deref() != Some("callout") {
            return Err(Error::invalid("label rows require a callout"));
        }
        let d = self
            .plan()
            .dimensions
            .iter()
            .find(|d| d.id == item.item.id)
            .unwrap();
        let ink = crate::dims::label_ink(&d.terms, crate::dims::font());
        let width = ink.max[0] - ink.min[0];
        let options = self.options(address)?;
        Ok(self
            .plan()
            .views
            .iter()
            .filter(|v| {
                options
                    .placements
                    .iter()
                    .any(|p| p.view.as_deref() == v.id.strip_prefix("v:"))
            })
            .map(|v| {
                let width = width.max(self.plan().canonical_callout_width(&v.id).unwrap_or(0.0));
                (
                    v.id.clone(),
                    crate::dims::label_rows(&self.analysis, v, width),
                )
            })
            .collect())
    }
    /// The callout's bend against its automatic leader tip, for pinning a drag with
    /// `place.offset`. Reads planning metadata only, without planning or projecting. Refuses a
    /// non-callout, or a callout with no automatic placement or not drawn in that view.
    pub fn callout_offset(&self, address: &str) -> Result<CalloutOffset, Error> {
        let item = self.item(address)?;
        if item.dimension_kind.as_deref() != Some("callout") {
            return Err(Error::invalid("a callout offset requires a callout"));
        }
        let id = item.item.id.as_str();
        let Some((view, tip)) = self.plan().automatic_callout_tip(id) else {
            return Err(Error::invalid(format!(
                "{id} has no automatic placement to measure an offset from"
            )));
        };
        let Some(bend) = self
            .plan()
            .dimensions
            .iter()
            .find(|d| d.id == id && d.view == view)
            .and_then(|d| Some(d.placed.as_ref()?.lines.first()?[1]))
        else {
            return Err(Error::invalid(format!("{id} is not drawn in {view}")));
        };
        Ok(CalloutOffset {
            view: view.into(),
            tip,
            bend,
            offset: [bend[0] - tip[0], bend[1] - tip[1]],
        })
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
            sheet_steps: self.planner.sheet_steps,
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
    /// Replace PMI against the original geometry, transactionally. Geometry caches survive.
    pub fn set_pmi(&mut self, bytes: &[u8]) -> Result<Delta, Error> {
        self.set_pmi_ops(bytes, &[])
    }
    /// Install PMI and its accompanying spec edits as one transaction and one plan.
    pub fn set_pmi_ops(&mut self, bytes: &[u8], ops: &[Op]) -> Result<Delta, Error> {
        let spec = model::edit(
            self.spec(),
            &serde_json::to_value(ops).expect("ops serialise"),
        )?;
        // A part of an assembly read alone takes its own PMI only.
        let reading = crate::pmi::read_part(bytes, &self.part, self.analysis.binding.part_index)
            .map_err(Error::invalid)?;
        let previous = self
            .analysis
            .pmi
            .replace(crate::pmi::Reading::Read(reading));
        let evaluated = match self.evaluate(&spec) {
            Ok(value) => value,
            Err(error) => {
                self.analysis.pmi = previous;
                return Err(error);
            }
        };
        let delta = difference(&self.state, &evaluated.state, &self.analysis, 1);
        self.install(evaluated);
        self.undo.clear();
        self.redo.clear();
        Ok(delta)
    }
    /// SVG fragments in sheet coordinates, sharing the full renderer's styles and text.
    pub fn svg_fragments(&self, delta: Option<&Delta>) -> BTreeMap<String, String> {
        self.svg_fragments_on_sheet(delta, 1)
            .expect("sheet 1 exists")
    }
    pub fn drawing_on_sheet(&self, sheet: u32) -> Result<Drawing, Error> {
        if sheet == 1 {
            return Ok(self.drawing.clone());
        }
        self.state.render_sheet(
            &self.part,
            &self.analysis,
            &self.font,
            &mut self.projections.clone(),
            sheet,
        )
    }
    pub fn svg_fragments_on_sheet(
        &self,
        delta: Option<&Delta>,
        sheet: u32,
    ) -> Result<BTreeMap<String, String>, Error> {
        self.state.check_sheet(sheet)?;
        let selected = (sheet != 1)
            .then(|| self.drawing_on_sheet(sheet))
            .transpose()?;
        let drawing = selected.as_ref().unwrap_or(&self.drawing);
        let mut out = BTreeMap::new();
        let on_sheet = |id: &str| {
            self.plan()
                .views
                .iter()
                .any(|v| v.id == id && v.sheet == sheet)
        };
        let owners = items::items(self.plan());
        let annotation_on_sheet = |id: &str| owners.iter().any(|i| i.id == id && i.sheet == sheet);
        let blank = || Drawing {
            page: drawing.page,
            layers: drawing
                .layers
                .iter()
                .map(|l| crate::output::Layer {
                    name: l.name.clone(),
                    style: l.style.clone(),
                    items: Vec::new(),
                })
                .collect(),
            link: None,
            text: Vec::new(),
            annotations: Vec::new(),
            sheet: crate::output::Sheet::default(),
            problems: Vec::new(),
            spec: None,
        };
        // A thread's or knurl's thin lines move with their view, so they travel in its fragment.
        let thin = drawing.layers.iter().position(|l| l.name == "thin");
        let manufacturing = thin.map(|_| crate::pmi_lowering::manufacturing(self.analysis()));
        let thin_ink = |id: &str| -> Vec<crate::output::Item> {
            let (Some(m), Some(v)) = (
                &manufacturing,
                self.plan().views.iter().find(|v| v.id == id),
            ) else {
                return Vec::new();
            };
            crate::cosmetic::ink(self.analysis(), m, v)
                .into_iter()
                .map(crate::output::Item::Edge)
                .collect()
        };
        for (id, geometry) in &self.state.geometry.views {
            if !on_sheet(id)
                || delta.is_some_and(|d| !d.geometry.views.iter().any(|(v, _)| v == id))
            {
                continue;
            }
            let mut d = blank();
            d.layers[0].items = geometry.visible.clone();
            d.layers[1].items = geometry.hidden.clone();
            if let Some(k) = thin {
                d.layers[k].items = thin_ink(id);
            }
            out.insert(id.clone(), d.svg());
        }
        for (id, item) in &self.state.annotations {
            if !annotation_on_sheet(id) || delta.is_some_and(|d| !d.annotations.contains_key(id)) {
                continue;
            }
            let mut d = blank();
            d.layers[2].items = item.items.clone();
            d.text = item.text.clone();
            out.insert(id.clone(), d.svg());
        }
        // Planned captions participate in the furniture collection diff, including removal.
        let furniture_changed = delta.is_none_or(|d| {
            d.plan.fields.keys().any(|k| k != "spec")
                || d.plan.collections.contains_key("views")
                || d.plan.collections.keys().any(|k| {
                    !matches!(
                        k.as_str(),
                        "dimensions" | "gdt" | "omitted" | "views" | "problems"
                    )
                })
        });
        if furniture_changed {
            let mut d = drawing.clone();
            let visible: usize = self
                .state
                .geometry
                .views
                .iter()
                .filter(|(id, _)| on_sheet(id))
                .map(|(_, g)| g.visible.len())
                .sum();
            let hidden: usize = self
                .state
                .geometry
                .views
                .iter()
                .filter(|(id, _)| on_sheet(id))
                .map(|(_, g)| g.hidden.len())
                .sum();
            let ink: usize = self
                .state
                .annotations
                .iter()
                .filter(|(id, _)| annotation_on_sheet(id))
                .map(|(_, a)| a.items.len())
                .sum();
            let text: usize = self
                .state
                .annotations
                .iter()
                .filter(|(id, _)| annotation_on_sheet(id))
                .map(|(_, a)| a.text.len())
                .sum();
            d.layers[0].items.drain(..visible);
            d.layers[1].items.drain(..hidden);
            d.layers[2].items.drain(..ink);
            if let Some(k) = thin {
                d.layers[k].items.clear();
            }
            d.text.drain(..text);
            out.insert("furniture".into(), d.svg());
        }
        Ok(out)
    }
    pub fn pmi_report(&self) -> Value {
        let mut lowered = Vec::new();
        let mut unresolved = Vec::new();
        if let Some(crate::pmi::Reading::Read(report)) = &self.analysis().pmi {
            let identities = report.identities();
            for record in &report.records {
                let ids: Vec<_> = self
                    .plan()
                    .dimensions
                    .iter()
                    .filter(|d| {
                        d.placed.is_some()
                            && d.measurements.iter().any(|m| {
                                identities
                                    .id(&record.source_id)
                                    .is_ok_and(|id| m.feature == id)
                            })
                    })
                    .map(|d| {
                        let kind = d
                            .measurements
                            .iter()
                            .find(|m| matches!(m.parameter.as_str(), "thread" | "knurl"))
                            .map(|m| m.parameter.clone())
                            .unwrap_or_else(|| d.kind.clone());
                        (d.id.clone(), d.label.clone(), kind)
                    })
                    .chain(
                        self.plan()
                            .gdt
                            .iter()
                            .filter(|g| g.source == record.source_id && g.placed.is_some())
                            .map(|g| (g.id.clone(), g.label.clone(), g.kind.clone())),
                    )
                    .chain(
                        self.plan()
                            .requirements
                            .iter()
                            .filter(|r| {
                                r.source == record.source_id
                                    && matches!(r.state, crate::items::State::Shown)
                            })
                            .map(|r| (r.id.clone(), r.text.clone(), r.kind.clone())),
                    )
                    .collect();
                if ids.is_empty() {
                    let related = identities.id(&record.source_id).ok();
                    let reasons: Vec<_> = self
                        .state
                        .problems
                        .iter()
                        .filter(|p| {
                            p.refs.contains(&record.source_id)
                                || related.is_some_and(|id| p.subjects.iter().any(|s| s == id))
                        })
                        .map(|p| p.message.as_str())
                        .collect();
                    let reason = if reasons.is_empty() {
                        record.blockers.join("; ")
                    } else {
                        reasons.join("; ")
                    };
                    unresolved.push(serde_json::json!({"source_id":record.source_id,"label":record.label,"kind":record.kind,"reason":reason}));
                }
                for (feature_id, label, kind) in ids {
                    lowered.push(serde_json::json!({"source_id":record.source_id,"feature_id":feature_id,"label":label,"kind":kind,"fit":record.fit}));
                }
            }
            if let Some(material) = report.title_material()
                && self
                    .plan()
                    .sheet
                    .title
                    .get("material")
                    .is_some_and(|m| m == material.designation.trim())
            {
                lowered.push(serde_json::json!({"source_id":material.census_id(),"feature_id":"part","label":material.designation.trim(),"kind":"material"}));
            }
            for source in &report.sources {
                if source.outcome == crate::pmi::Outcome::NotExtracted
                    && !lowered.iter().any(|r| r["source_id"] == source.id)
                {
                    unresolved.push(serde_json::json!({"source_id":source.id,"label":source.entity,"kind":source.category,"reason":source.reason}));
                }
            }
            serde_json::json!({"extraction":report,"lowered":lowered,"unresolved":unresolved})
        } else {
            serde_json::json!({"lowered":[],"unresolved":[]})
        }
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
        // Part, font and recognized geometry are fixed for this Session; PMI and
        // Spec are its mutable rendering inputs. Reuse an exact preview's assembled
        // drawing, still planning, placing geometry and computing ink/deltas each time.
        let key: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&(spec, &self.analysis.pmi)).expect("render inputs serialize"),
        )
        .into();
        let drawing = if let Some(drawing) = self.render_memo.get(&key) {
            drawing
        } else {
            let drawing = state.render(&self.part, &self.analysis, &self.font, &mut cache)?;
            self.render_memo.insert(key, drawing.clone());
            drawing
        };
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
    pub fn item(&self, address: &str) -> Result<ItemAnswer, Error> {
        let (sheet, id) = item_address(address)?;
        let item = self.bare_item(id)?;
        if sheet.is_some_and(|n| n != item.item.sheet) {
            return Err(Error {
                choices: vec![format!("sheet:{}/{}", item.item.sheet, id)],
                ..Error::invalid(format!("{id} is on sheet {}", item.item.sheet))
            });
        }
        Ok(item)
    }
    fn bare_item(&self, id: &str) -> Result<ItemAnswer, Error> {
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
            .and_then(|i| item_bounds(&i).map(|bounds| (i.item.sheet, bounds)));
        let region = filter.region.or_else(|| {
            near.map(|(_, [a, b, c, d])| {
                let r = filter.radius.unwrap_or(10.0).max(0.0);
                [a - r, b - r, c + r, d + r]
            })
        });
        answers(self.plan(), &self.analysis)
            .into_iter()
            .filter(|i| {
                filter.sheet.is_none_or(|n| i.item.sheet == n)
                    && near.is_none_or(|(n, _)| i.item.sheet == n)
                    && filter.view.as_ref().is_none_or(|v| {
                        i.item.view.as_ref().is_some_and(|w| {
                            w.trim_start_matches("v:") == v.trim_start_matches("v:")
                        })
                    })
                    && filter
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
                        item_bounds(i).is_some_and(|b| {
                            b[0] <= r[2] && b[2] >= r[0] && b[1] <= r[3] && b[3] >= r[1]
                        })
                    })
            })
            .collect()
    }
    pub fn problems(&self, filter: &ProblemFilter) -> Vec<model::Problem> {
        let subjects: Vec<_> = filter
            .feature
            .as_ref()
            .map(|feature| {
                self.items(&ItemFilter {
                    feature: Some(feature.clone()),
                    ..ItemFilter::default()
                })
                .into_iter()
                .map(|i| i.item.id)
                .chain(std::iter::once(feature.clone()))
                .collect()
            })
            .unwrap_or_default();
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
                    && (filter.feature.is_none() || p.subjects.iter().any(|s| subjects.contains(s)))
            })
            .cloned()
            .collect()
    }
    pub fn options(&self, id: &str) -> Result<Options, Error> {
        if let Some(face) = id.strip_prefix("face:") {
            let count = self.analysis.face_attachments.len();
            let index = face.parse::<usize>().ok().filter(|&i| i < count)
                .ok_or_else(|| Error::invalid(format!("{id:?} is not a face in this Analysis; use face:<index> with 0 <= index < {count}, or re-analyse if face evidence is absent.")))?;
            let fingerprint =
                crate::recognition_corrections::face_fingerprint(&self.analysis, index)?;
            return Ok(Options {
                id: id.into(),
                operations: vec!["add".into()],
                placements: vec![],
                view_anchors: vec![],
                scales: vec![],
                settable: BTreeMap::from([(
                    "feature_declarations".into(),
                    serde_json::json!({"fields":[fingerprint],"forms":["add {id: f:declared:<name>, kind: hole|boss|turned_step|slot|pocket, faces: [fingerprint,…]}"]}),
                )]),
            });
        }
        if id.starts_with("f:") {
            let feature = self.analysis.features.iter().find(|f| f.id == id);
            let pattern = self
                .analysis
                .recognition
                .iter()
                .flat_map(|r| &r.patterns)
                .any(|p| p.id == id);
            if feature.is_none() && !pattern {
                return Err(Error::invalid(format!(
                    "There is no recognised feature or pattern {id:?}; use analyse to read the current feature and pattern IDs."
                )));
            }
            let mut fields = BTreeMap::new();
            let base = format!("features/{}", items::key(id));
            if let Some(f) = feature {
                fields.insert(
                    format!("{base}/suppress"),
                    serde_json::json!({"choices":[false,true]}),
                );
                let kinds = match f.kind.as_str() {
                    "boss" | "turned_step" => vec!["boss", "turned_step"],
                    "slot" | "section_recess" => vec!["slot", "pocket"],
                    _ => vec![],
                };
                if !kinds.is_empty() {
                    fields.insert(
                        format!("{base}/reclassify"),
                        serde_json::json!({"choices":kinds}),
                    );
                }
            }
            if pattern || feature.is_some_and(|f| f.kind == "slot_pattern") {
                fields.insert(
                    format!("{base}/split"),
                    serde_json::json!({"choices":[false,true]}),
                );
            }
            return Ok(Options {
                id: id.into(),
                operations: vec!["set".into(), "remove".into()],
                placements: vec![],
                view_anchors: vec![],
                scales: vec![],
                settable: fields,
            });
        }
        Ok(self.options_for(self.item(id)?, &self.placement_choices()))
    }
    /// All item options, computing the shared geometry choices once and without planning.
    pub fn all_options(&self) -> BTreeMap<String, Options> {
        let choices = self.placement_choices();
        answers(self.plan(), &self.analysis)
            .into_iter()
            .map(|item| (item.item.id.clone(), self.options_for(item, &choices)))
            .collect()
    }
    fn placement_choices(&self) -> BTreeMap<String, Value> {
        let mut choices = crate::dims::placement_options_with_spec(
            &self.analysis,
            &self.plan().views,
            self.spec(),
        );
        // The Spec's datums and controls are placed as the file's PMI is ([`crate::spec_gdt`]).
        let merged = crate::spec_gdt::effective(&self.analysis, &self.plan().spec);
        choices.extend(crate::dims::gdt::placement_options(
            merged.as_deref().unwrap_or(&self.analysis),
            &self.plan().views,
        ));
        for n in &self.plan().notes {
            if n.id.starts_with("n:f:blend:") {
                choices.insert(n.id.clone(), serde_json::json!({"views":self.plan().views.iter().filter(|v| ["v:front","v:plan","v:side"].contains(&v.id.as_str())).map(|v| serde_json::json!({"view":v.id.trim_start_matches("v:"),"sides":["above","below","left","right"]})).collect::<Vec<_>>() }));
            }
        }
        choices
    }
    fn options_for(&self, item: ItemAnswer, choices: &BTreeMap<String, Value>) -> Options {
        let id = item.item.id.as_str();
        let angular = item.dimension_kind.as_deref() == Some("angular");
        let placements: Vec<Place> = choices
            .get(id)
            .into_iter()
            .flat_map(|c| {
                c["views"].as_array().into_iter().flatten().flat_map(|v| {
                    v["sides"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(Some)
                        .chain(angular.then_some(None))
                        .filter_map(move |side| {
                            Some(Place {
                                view: Some(v["view"].as_str()?.into()),
                                side: side
                                    .and_then(|side| serde_json::from_value(side.clone()).ok()),
                                lane: None,
                                along: Some(Along::Slot(Slot::Middle)),
                                offset: None,
                                pinned: false,
                            })
                        })
                })
            })
            .collect();
        let view_anchors: Vec<String> = if item.item.kind == "view" {
            self.plan()
                .views
                .iter()
                .filter(|v| v.id != id && !depends_on(self.spec(), &v.id, id))
                .map(|v| v.id.clone())
                .collect()
        } else {
            Vec::new()
        };
        Options {
            settable: settable(self, &item, &placements, &view_anchors),
            id: id.into(),
            operations: item.operations,
            placements,
            view_anchors,
            scales: if item.item.kind == "sheet" {
                crate::layout::SCALES.map(model::scale_name).to_vec()
            } else {
                Vec::new()
            },
        }
    }
}
fn settable(
    session: &Session,
    item: &ItemAnswer,
    placements: &[Place],
    view_anchors: &[String],
) -> BTreeMap<String, Value> {
    use serde_json::json;
    let mut fields = BTreeMap::new();
    let merged = crate::recognition_corrections::effective(session.analysis(), session.spec()).ok();
    let analysis = merged.as_deref().unwrap_or(session.analysis());
    let id = &item.item.id;
    let base = match item.item.kind.as_str() {
        "dimension" => format!("dimensions/{}", items::key(id)),
        "view" => format!("views/{}", items::key(id.trim_start_matches("v:"))),
        "control_frame" | "datum_feature" | "label" | "surface_finish" | "note"
            if item.item.kind != "note" || id.starts_with("n:f:blend:") =>
        {
            format!("annotations/{}", items::key(id))
        }
        _ => String::new(),
    };
    let choices = |values: Value| json!({"choices":values});
    fields.insert(
        "hole_table".into(),
        choices(json!(["auto", "always", "never"])),
    );
    if id == "sheet" {
        fields.insert("dimension_schemes/<name>".into(), json!({"forms":["{scheme:chain|baseline|ordinate,axis:x|y|z,view?:front|plan|side,targets?:[ID|fingerprint],from?:min|max|reference}"],"consequences":["Requires targets or view; rejects overlapping owners. Remove for auto."]}));
        fields.insert("dimensions/d:authored:<name>".into(), json!({"forms":["{between:[reference,reference],along:x|y|z|true,kind:linear}","{kind:diameter|radius,between:[face reference]}","{kind:angle,between:[reference,reference]}"],"consequences":["References are stable keys from Analysis.edge_ink.references; duplicate quantities are refused with their existing owner ID"]}));
    }
    if !base.is_empty() {
        fields.insert(format!("{base}/omit"), choices(json!([false, true])));
        if item.item.kind == "view" {
            for field in crate::linework::FIELDS {
                fields.insert(format!("{base}/{field}"), choices(json!([false, true])));
            }
            // Preserve default single-sheet answers until the author enables sheet controls.
            if session.spec().sheets.is_some() {
                let name = id.trim_start_matches("v:");
                let max = session
                    .spec()
                    .sheets
                    .as_ref()
                    .and_then(|s| s.max)
                    .unwrap_or(1);
                let mut consequences = Vec::new();
                for group in session
                    .spec()
                    .sheets
                    .iter()
                    .flat_map(|s| &s.keep_together)
                    .filter(|g| g.iter().any(|v| v == name))
                {
                    consequences.push(format!(
                        "Must share a sheet with keep_together group: {}",
                        group.join(", ")
                    ));
                }
                for (view, intent) in &session.spec().views {
                    if let Some(place) = &intent.place
                        && (view == name || place.relative_to == name)
                    {
                        consequences.push(format!(
                        "{view} must share a sheet with {}; remove views/{}/place to separate them",
                        place.relative_to,
                        items::key(view)
                    ));
                    }
                }
                if ["front", "plan", "side"].contains(&name) {
                    consequences.push("Principal views must stay on sheet 1".into());
                }
                fields.insert(format!("{base}/sheet"), json!({
                "choices":(1..=if ["front", "plan", "side"].contains(&name) {1} else {max}).collect::<Vec<_>>(),
                "consequences":consequences,
            }));
            }
            fields.insert(
                format!("{base}/place/relative_to"),
                choices(json!(
                    view_anchors
                        .iter()
                        .map(|v| v.trim_start_matches("v:"))
                        .collect::<Vec<_>>()
                )),
            );
            fields.insert(
                format!("{base}/place/side"),
                choices(json!(["above", "below", "left", "right"])),
            );
            fields.insert(format!("{base}/place/gap"), json!({"range":[0,1000]}));
            fields.insert(
                format!("{base}/place/align"),
                choices(json!(["start", "middle", "end", "outside"])),
            );
        } else {
            let mut views: Vec<_> = placements.iter().filter_map(|p| p.view.clone()).collect();
            views.sort();
            views.dedup();
            let mut sides: Vec<_> = placements
                .iter()
                .filter_map(|p| p.side.map(|s| s.name()))
                .collect();
            sides.sort();
            sides.dedup();
            fields.insert(format!("{base}/place/view"), choices(json!(views)));
            fields.insert(format!("{base}/place/side"), choices(json!(sides)));
            fields.insert(
                format!("{base}/place/lane"),
                json!({"range":[0,1000],"integer":true}),
            );
            fields.insert(
                format!("{base}/place/along"),
                json!({"choices":["start","middle","end","outside"],"range":[0,1]}),
            );
            // #12: a pinned callout's bend from its automatic leader tip, [dx, dy] paper mm.
            if item.dimension_kind.as_deref() == Some("callout") {
                fields.insert(
                    format!("{base}/place/offset"),
                    json!({"range":[-1000,1000]}),
                );
            }
        }
        fields.insert(
            format!("{base}/place/pinned"),
            choices(json!([false, true])),
        );
        if item.item.kind == "dimension"
            || item.item.kind == "label"
            || id.starts_with("n:f:blend:")
        {
            fields.insert(
                format!("{base}/text"),
                json!({"fields":["prefix","suffix","template"],"forms":["{value} AFTER PLATING"]}),
            );
        }
        if item.item.kind == "dimension" {
            fields.insert(
                format!("{base}/style"),
                choices(json!(["reference", "basic"])),
            );
            fields.insert(
                format!("{base}/view"),
                fields[&format!("{base}/place/view")].clone(),
            );
            fields.insert(format!("{base}/tolerance"), json!({"forms":["±T","+U -L"]}));
            if session
                .spec()
                .dimensions
                .get(id)
                .is_some_and(|d| d.between.is_some())
            {
                fields.insert(
                    format!("{base}/kind"),
                    choices(json!(["linear", "diameter", "radius", "angle"])),
                );
                fields.insert(format!("{base}/between"), json!({"forms":["[reference, reference] (linear/angle)","[face reference] (diameter/radius)"],"consequences":["Copy stable keys from Analysis.edge_ink.references; axis:<face key> selects a cylinder axis"]}));
                fields.insert(
                    format!("{base}/along"),
                    choices(json!(["x", "y", "z", "true"])),
                );
            }
        }
    }
    // Recognition edits travel through the same transactional preview/apply and history.
    if id == "sheet" {
        fields.insert("features/f:hole_pattern:<name>/pattern".into(), json!({"fields":["arrangement", "members"], "forms":["{arrangement: linear|grid|bolt_circle, members: [hole id,…]}"]}));
        fields.insert("feature_declarations".into(), json!({
            "forms":["add {id: f:declared:<name>, kind: hole|boss|turned_step|slot|pocket, faces: [fingerprint,…]}",
                format!("Get a face fingerprint with item_options face:<index>, 0 <= index < {}", analysis.face_attachments.len())]
        }));
    }
    // A fit qualifies a feature, including each member of a consolidated hole mark.
    let mut fits: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    for feature in &item.features {
        if let Some(f) = analysis.features.iter().find(|f| &f.id == feature) {
            let classes = crate::dims::fit_choices(
                &f.kind,
                f.parameters
                    .get("diameter")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
            );
            if !classes.is_empty() {
                fits.entry(classes)
                    .or_default()
                    .push(format!("features/{}/fit", items::key(feature)));
            }
        }
    }
    for (classes, paths) in fits {
        let mut descriptor = choices(json!(classes));
        if paths.len() > 1 {
            descriptor["paths"] = json!(paths);
        }
        fields.insert(paths[0].clone(), descriptor);
    }
    if item.item.kind == "sheet" {
        for field in crate::linework::FIELDS {
            fields.insert(format!("sheet/{field}"), choices(json!([false, true])));
        }
    }
    // A datum letter and feature control frames qualify a feature or hole pattern
    // ([`crate::spec_gdt`]); a face takes them at faces/<side or face:n>/….
    let patterns = analysis
        .recognition
        .as_ref()
        .map_or(&[][..], |r| r.patterns.as_slice());
    // The feature the item's ID names (a hole's callout its hole, a hole pattern's its pattern),
    // once: offering every measured feature would outgrow the reply's wire budget.
    let named = item
        .item
        .id
        .strip_prefix("d:")
        .and_then(|s| s.rsplit_once('/'))
        .map(|(f, _)| f.to_string());
    if let Some(feature) = &named {
        let known = analysis.features.iter().any(|f| &f.id == feature)
            || patterns.iter().any(|p| &p.id == feature);
        if known {
            let base = format!("features/{}", items::key(feature));
            fields.insert(format!("{base}/datum"), json!({"forms":["A"]}));
            fields.insert(
                format!("{base}/controls"),
                json!({
                    "fields": ["characteristic", "tolerance", "datums", "diameter", "modifier"],
                }),
            );
            // A finish and a note on any feature; a thread on a hole, hole pattern or turned
            // diameter; a knurl on a turned diameter ([`crate::spec_notes`]).
            fields.insert(format!("{base}/finish"), json!({"fields":["ra","removal"]}));
            fields.insert(format!("{base}/note"), json!({"forms":["text"]}));
            let kind = analysis
                .features
                .iter()
                .find(|f| &f.id == feature)
                .map_or("hole_pattern", |f| f.kind.as_str());
            if matches!(kind, "hole" | "hole_pattern" | "turned_step" | "boss") {
                fields.insert(
                    format!("{base}/thread"),
                    json!({"fields":["designation","length"]}),
                );
            }
            if matches!(kind, "turned_step" | "boss") {
                fields.insert(
                    format!("{base}/knurl"),
                    json!({"fields":["pattern","pitch"],"choices":["straight","diamond"]}),
                );
            }
        }
    }
    if item.item.kind == "sheet" {
        fields.insert(
            "sheet/size".into(),
            choices(json!(crate::layout::PAGES.map(|p| p.0))),
        );
        fields.insert(
            "sheet/scale".into(),
            choices(json!(crate::layout::SCALES.map(model::scale_name))),
        );
        fields.insert(
            "sheet/title".into(),
            json!({"fields":crate::title_block::KEYS.map(|k|k.0),"forms":["text"]}),
        );
        fields.insert(
            "sheet/automatic_details".into(),
            choices(json!([false, true])),
        );
        fields.insert("sheet/fit_limits".into(), choices(json!([false, true])));
        fields.insert("sheet/fit_deviations".into(), choices(json!([false, true])));
        fields.insert(
            "sheet/projection".into(),
            choices(json!(model::PROJECTIONS)),
        );
        fields.insert(
            "sheet/default_finish".into(),
            json!({"fields":["ra","removal"]}),
        );
    }
    fields
}

/// IDs remain unique; a qualifier checks ownership rather than changing identity.
pub(crate) fn item_address(address: &str) -> Result<(Option<u32>, &str), Error> {
    if let Some((prefix, id)) = address.split_once('/')
        && let Some(n) = prefix.strip_prefix("sheet:")
    {
        let sheet = n
            .parse::<u32>()
            .ok()
            .filter(|n| (1..=9).contains(n))
            .ok_or_else(|| Error::invalid("item addresses use sheet:<1..9>/<id>"))?;
        return Ok((Some(sheet), id));
    }
    Ok((None, address))
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
    let carrier_sheet = |p: &Plan| {
        p.dimensions
            .iter()
            .find(|d| d.id == id)
            .map(|d| &d.view)
            .or_else(|| p.gdt.iter().find(|g| g.id == id).map(|g| &g.view))
            .and_then(|id| p.views.iter().find(|v| &v.id == id))
            .map(|v| v.sheet)
    };
    carrier_sheet(old) != carrier_sheet(new)
        || old
            .dimensions
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
    let owners = if plan.sheets.is_empty() {
        Vec::new()
    } else {
        items::items(plan)
    };
    plan.dimensions
        .iter()
        .map(|d| (&d.id, AnnotationItem::Dimension(d)))
        .chain(plan.gdt.iter().map(|g| (&g.id, AnnotationItem::Gdt(g))))
        .map(|(id, item)| {
            let kept = old
                .filter(|s| !annotation_changed(&s.plan, plan, id))
                .and_then(|s| s.annotations.get(id))
                .cloned();
            let mut rendered = kept.unwrap_or_else(|| model::render_item(item));
            rendered.provenance = owners
                .iter()
                .find(|i| &i.id == id)
                .cloned()
                .into_iter()
                .collect();
            (id.clone(), rendered)
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
            if item.kind == "sheet" {
                a.basis = std::iter::once(&plan.sheet)
                    .chain(&plan.sheets)
                    .find(|s| s.number == item.sheet)
                    .expect("a sheet item has a planned sheet")
                    .basis
                    .clone();
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
                a.operations
                    .extend(crate::linework::FIELDS.map(String::from));
                if plan.spec.sheets.is_some() {
                    a.operations.push("move_to_sheet".into());
                }
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
                for m in &d.measurements {
                    if let Some(refs) = m.feature.strip_prefix("geometry:") {
                        a.anchors.extend(refs.split('|').map(String::from));
                    }
                }
                a.operations = ["omit", "tolerance", "text", "style", "place", "pin"]
                    .map(String::from)
                    .into();
                if d.from_spec.iter().any(|field| field == "between") {
                    a.operations
                        .extend(["between", "kind", "along"].map(String::from));
                }
            }
            if let Some(o) = plan.omitted.iter().find(|o| &o.id == id) {
                a.basis = o.basis.clone();
                if let Some(t) = &plan.radius_table {
                    a.features.extend(
                        t.rows
                            .iter()
                            .filter(|r| r.dimension == *id)
                            .map(|r| r.feature.clone()),
                    );
                }
                if let Some((feature, _)) = id.strip_prefix("d:").and_then(|s| s.rsplit_once('/'))
                    && !a.features.iter().any(|f| f == feature)
                {
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
                if g.kind == "label" {
                    a.operations.push("text".into());
                }
            }
            if let Some(c) = plan.captions.iter().find(|c| &c.id == id) {
                a.basis = c.basis.clone();
                a.placement = Some(Placement::Box(c.ink));
                a.anchors = vec![c.view.clone()];
            }
            if let Some(n) = plan.notes.iter().find(|n| &n.id == id) {
                a.basis = n.basis.clone();
                a.placement = n.placed.as_ref().map(|p| Placement::Box(p.cells));
                a.features = n.measurements.iter().map(|m| m.feature.clone()).collect();
                if n.id.starts_with("n:f:blend:") {
                    a.operations = ["omit", "text", "place"].map(String::from).into();
                }
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
            if let Some(t) = plan.radius_table.as_ref().filter(|t| &t.table.id == id) {
                a.basis = t.table.basis.clone();
                a.placement = t.table.placed.as_ref().map(|p| Placement::Box(p.cells));
                a.features = t.rows.iter().map(|r| r.feature.clone()).collect();
            }
            if let Some(t) = plan
                .hole_table
                .iter()
                .chain(&plan.hole_tables)
                .find(|t| &t.id == id)
            {
                a.basis = t.basis.clone();
                a.features = t.rows.iter().map(|r| r.feature.clone()).collect();
                a.anchors = vec![t.view.clone().unwrap_or_else(|| "v:plan".into())];
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
pub fn item_bounds(i: &ItemAnswer) -> Option<[f64; 4]> {
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
    if let Some(id) = doc["radius_table"]["table"]["id"].as_str() {
        out.entry(id.into())
            .or_default()
            .push(doc["radius_table"].clone());
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
                    a.sheet != b.sheet
                        || a.at != b.at
                        || a.scale != b.scale
                        || a.direction != b.direction
                        || a.up != b.up
                        || a.cut != b.cut
                        || a.line_style != b.line_style
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
            .filter(|(id, v)| {
                annotation_changed(&old.plan, &new.plan, id)
                    || old.annotations.get(*id).map(|a| &a.provenance) != Some(&v.provenance)
            })
            .map(|(id, v)| (id.clone(), v.clone()))
            .collect(),
        geometry_order: (view_ids(old) != view_ids(new)).then(|| view_ids(new)),
        problem_order: (old.problems.iter().map(|p| &p.id).collect::<Vec<_>>()
            != new.problems.iter().map(|p| &p.id).collect::<Vec<_>>())
        .then(|| new.problems.iter().map(|p| p.id.clone()).collect()),
    }
}
