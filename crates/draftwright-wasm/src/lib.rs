//! A wasm-bindgen front end over draftwright's live Session and operation schema.
pub use draftwright::api::Error;
pub use draftwright::session::Session as CoreSession;
use draftwright::{api, items::Op, model, session};

pub fn error_json(error: &Error) -> String {
    serde_json::to_string(error).expect("errors serialise")
}

pub struct Session(pub CoreSession);
impl Session {
    pub fn open(name: &str, bytes: &[u8]) -> Result<Self, Error> {
        let (part, analysis) = model::analyse_native_bytes(name, bytes)?;
        CoreSession::new(part, analysis, model::Spec::default()).map(Self)
    }
    pub fn apply(&mut self, ops: &[Op], commit: bool) -> Result<session::Delta, Error> {
        self.0.apply(ops, commit)
    }
    pub fn apply_json(&mut self, json: &str, commit: bool) -> Result<String, Error> {
        let ops: Vec<Op> =
            serde_json::from_str(json).map_err(|e| Error::invalid(format!("invalid ops: {e}")))?;
        Ok(serde_json::to_string(&self.apply(&ops, commit)?).expect("delta serialises"))
    }
    pub fn apply_fragments_json(&mut self, json: &str) -> Result<String, Error> {
        let ops: Vec<Op> = serde_json::from_str(json).map_err(|e| Error::invalid(e.to_string()))?;
        let delta = self.apply(&ops, true)?;
        let annotations: std::collections::BTreeMap<_, _> =
            delta.annotations.keys().map(|id| (id, true)).collect();
        let changed: Vec<_> = delta.changed.iter().map(|i| serde_json::json!({"item":{"id":i.item.id},"placement":i.placement.as_ref().map(|_|true)})).collect();
        let mut packet = serde_json::json!({"delta":{"plans":delta.plans,"plan":delta.plan,"removed":delta.removed,"changed":changed,"annotations":annotations,"problems_added":delta.problems_added,"problems_resolved":delta.problems_resolved,"problem_order":delta.problem_order},"fragments":self.0.svg_fragments(Some(&delta)),"pmi":self.0.pmi_report()});
        let plan = serde_json::to_value(self.0.plan()).unwrap();
        for (name, patch) in packet["delta"]["plan"]["collections"]
            .as_object_mut()
            .unwrap()
        {
            patch["present"] = serde_json::Value::Bool(plan.get(name.as_str()).is_some());
        }
        if ops.iter().any(|op| {
            let path = op.path.trim_start_matches('/');
            path.starts_with("views") || path.starts_with("dimension_schemes")
        }) {
            packet["placementOptions"] =
                serde_json::from_str(&self.placement_options_json()?).unwrap();
        }
        Ok(packet.to_string())
    }
    pub fn plans(&self) -> usize {
        self.0.stats().plans
    }
    pub fn source_sha256(&self) -> &str {
        &self.0.analysis().binding.source_sha256
    }
    pub fn face_count(&self) -> usize {
        self.0.part().faces.len()
    }
    pub fn feature_count(&self) -> usize {
        self.0.analysis().features.len()
    }
    pub fn analysis_json(&self) -> String {
        serde_json::to_string(self.0.analysis()).unwrap()
    }
    pub fn provenance_json(&self) -> String {
        serde_json::json!({"source_sha256": self.source_sha256(), "face_entities": (0..self.face_count()).map(|i| self.0.part().face_source(i).map(|s| s.entity)).collect::<Vec<_>>()}).to_string()
    }
    pub fn spec_json(&self) -> String {
        serde_json::to_string(self.0.spec()).unwrap()
    }
    pub fn plan_json(&self) -> Result<String, Error> {
        Ok(serde_json::to_string(self.0.plan()).unwrap())
    }
    pub fn render_svg(&self) -> Result<String, Error> {
        Ok(self.0.drawing().svg())
    }
    /// The drawing as a PDF, from the engine's own writer with its bundled fonts embedded.
    pub fn render_pdf(&self) -> Vec<u8> {
        self.0.drawing().pdf(&|name| match name {
            "IBMPlexMono-Regular.ttf" => Some(api::MONO.to_vec()),
            draftwright::title_block::FONT => Some(draftwright::title_block::FONT_DATA.to_vec()),
            _ => None,
        })
    }
    pub fn set_pmi(&mut self, source: &str, bytes: &[u8]) -> Result<(), Error> {
        if source != self.source_sha256() {
            return Err(Error::invalid("PMI was generated from another source file"));
        }
        self.0.set_pmi(bytes).map(|_| ())
    }
    pub fn set_pmi_ops(&mut self, source: &str, bytes: &[u8], json: &str) -> Result<(), Error> {
        if source != self.source_sha256() {
            return Err(Error::invalid("PMI was generated from another source file"));
        }
        let ops: Vec<Op> = serde_json::from_str(json).map_err(|e| Error::invalid(e.to_string()))?;
        self.0.set_pmi_ops(bytes, &ops).map(|_| ())
    }
    pub fn set_spec(&mut self, json: &str) -> Result<(), Error> {
        let spec: model::Spec =
            serde_json::from_str(json).map_err(|e| Error::invalid(format!("invalid spec: {e}")))?;
        let spec = model::migrate(&spec, self.0.analysis())?;
        let old = serde_json::to_value(self.0.spec()).unwrap();
        let new = serde_json::to_value(spec).unwrap();
        let mut ops = Vec::new();
        for (key, value) in new.as_object().unwrap() {
            if old.get(key) != Some(value) {
                ops.push(Op::set(key, value.clone()));
            }
        }
        for key in old.as_object().unwrap().keys() {
            if new.get(key).is_none() {
                ops.push(Op::remove(key));
            }
        }
        if !ops.is_empty() {
            self.apply(&ops, true)?;
        }
        Ok(())
    }
    pub fn placement_options_json(&self) -> Result<String, Error> {
        let mut out = serde_json::Map::new();
        let mut choices = self.0.all_options();
        for answer in self.0.items(&session::ItemFilter::default()) {
            let options = choices.remove(&answer.item.id).expect("item has options");
            if !options.operations.contains(&"omit".into()) {
                continue;
            }
            let mut views = std::collections::BTreeMap::<String, Vec<String>>::new();
            for place in options.placements {
                if let (Some(view), Some(side)) = (place.view, place.side) {
                    views
                        .entry(view)
                        .or_default()
                        .push(serde_json::to_value(side).unwrap().as_str().unwrap().into());
                }
            }
            out.insert(answer.item.id, serde_json::json!({"label": answer.item.label, "views": views.into_iter().map(|(view,sides)| serde_json::json!({"view":view,"sides":sides})).collect::<Vec<_>>(), "operations": options.operations}));
        }
        Ok(serde_json::Value::Object(out).to_string())
    }
    pub fn pmi_json(&self) -> String {
        self.0.pmi_report().to_string()
    }
    pub fn snapshot(&self) -> Result<String, Error> {
        Ok(serde_json::json!({"problems":self.0.state().problems,"plan":self.0.plan(),"spec":self.0.spec(),"svg":self.0.drawing().svg(),"placementOptions":serde_json::from_str::<serde_json::Value>(&self.placement_options_json()?).unwrap(),"provenance":serde_json::from_str::<serde_json::Value>(&self.provenance_json()).unwrap(),"analysis":self.0.analysis(),"pmi":serde_json::from_str::<serde_json::Value>(&self.pmi_json()).unwrap()}).to_string())
    }
    pub fn preview(&mut self, json: &str) -> Result<String, Error> {
        let ops: Vec<Op> =
            serde_json::from_str(json).map_err(|e| Error::invalid(format!("invalid ops: {e}")))?;
        let delta = self.apply(&ops, false)?;
        let mut state = self.0.state().clone();
        delta.apply_to(&mut state)?;
        let drawing = state.render(
            self.0.part(),
            self.0.analysis(),
            &draftwright::text::Font::new(api::MONO.to_vec()).unwrap(),
            &mut model::Projections::default(),
        )?;
        let mut snapshot: serde_json::Value = serde_json::from_str(&self.snapshot()?).unwrap();
        snapshot["problems"] = serde_json::to_value(&state.problems).unwrap();
        snapshot["plan"] = serde_json::to_value(&state.plan).unwrap();
        snapshot["spec"] = serde_json::to_value(&state.plan.spec).unwrap();
        snapshot["svg"] = serde_json::Value::String(drawing.svg());
        Ok(snapshot.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use super::{Session, error_json};
    use wasm_bindgen::prelude::*;
    fn js(e: super::Error) -> JsValue {
        JsValue::from_str(&error_json(&e))
    }
    #[wasm_bindgen(start)]
    pub fn start() {
        console_error_panic_hook::set_once();
    }
    #[wasm_bindgen]
    pub struct Engine(Session);
    #[wasm_bindgen]
    impl Engine {
        #[wasm_bindgen(constructor)]
        pub fn open(name: &str, bytes: &[u8]) -> Result<Engine, JsValue> {
            Session::open(name, bytes).map(Engine).map_err(js)
        }
        pub fn apply(&mut self, ops: &str, commit: bool) -> Result<String, JsValue> {
            self.0.apply_json(ops, commit).map_err(js)
        }
        pub fn fragments(&self) -> String {
            serde_json::to_string(&self.0.0.svg_fragments(None)).unwrap()
        }
        #[wasm_bindgen(js_name = applyFragments)]
        pub fn apply_fragments(&mut self, ops: &str) -> Result<String, JsValue> {
            self.0.apply_fragments_json(ops).map_err(js)
        }
        pub fn preview(&mut self, ops: &str) -> Result<String, JsValue> {
            self.0.preview(ops).map_err(js)
        }
        pub fn snapshot(&self) -> Result<String, JsValue> {
            self.0.snapshot().map_err(js)
        }
        pub fn undo(&mut self) -> Result<String, JsValue> {
            self.0
                .0
                .undo()
                .map(|d| serde_json::to_string(&d).unwrap())
                .map_err(js)
        }
        pub fn redo(&mut self) -> Result<String, JsValue> {
            self.0
                .0
                .redo()
                .map(|d| serde_json::to_string(&d).unwrap())
                .map_err(js)
        }
        #[wasm_bindgen(js_name = setPmi)]
        pub fn set_pmi(&mut self, source: &str, bytes: &[u8]) -> Result<(), JsValue> {
            self.0.set_pmi(source, bytes).map_err(js)
        }
        #[wasm_bindgen(js_name = setPmiOps)]
        pub fn set_pmi_ops(
            &mut self,
            source: &str,
            bytes: &[u8],
            ops: &str,
        ) -> Result<(), JsValue> {
            self.0.set_pmi_ops(source, bytes, ops).map_err(js)
        }
        #[wasm_bindgen(js_name = setSpec)]
        pub fn set_spec(&mut self, json: &str) -> Result<(), JsValue> {
            self.0.set_spec(json).map_err(js)
        }
        #[wasm_bindgen(js_name = planCount)]
        pub fn plans(&self) -> usize {
            self.0.plans()
        }
        #[wasm_bindgen(js_name = faceCount)]
        pub fn face_count(&self) -> usize {
            self.0.face_count()
        }
        #[wasm_bindgen(js_name = featureCount)]
        pub fn feature_count(&self) -> usize {
            self.0.feature_count()
        }
        pub fn plan(&self) -> Result<String, JsValue> {
            self.0.plan_json().map_err(js)
        }
        pub fn spec(&self) -> String {
            self.0.spec_json()
        }
        pub fn pmi(&self) -> String {
            self.0.pmi_json()
        }
        #[wasm_bindgen(js_name = placementOptions)]
        pub fn placement_options(&self) -> Result<String, JsValue> {
            self.0.placement_options_json().map_err(js)
        }
        #[wasm_bindgen(js_name = renderSvg)]
        pub fn render_svg(&self) -> Result<Vec<u8>, JsValue> {
            self.0.render_svg().map(String::into_bytes).map_err(js)
        }
        #[wasm_bindgen(js_name = renderPdf)]
        pub fn render_pdf(&self) -> Vec<u8> {
            self.0.render_pdf()
        }
    }
}
