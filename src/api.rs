//! The operations every adapter calls (ADR R1): the command line, the JSON-lines service
//! (`draftwright serve`) and the MCP server (`draftwright-mcp`) are thin layers over these.
//!
//! - [`analyse`], [`plan`], [`edit`], [`render`] (with [`write`]), [`spec`] and [`review`],
//!   typed, over the documents of [`crate::model`];
//! - [`call`]: the same operations by name with JSON arguments, as the service and the MCP server
//!   receive them. Requests carry their documents and name the STEP file by path, so nothing is
//!   kept between calls.
//!
//! An [`Error`] is `invalid` (the request cannot be met as asked; it says what would be accepted)
//! or `failed` (something went wrong; nothing was written in its place).

use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};

use crate::lint::{self, LintIssue};
use crate::model::{self, Analysis, Plan, Problem, Spec};
use crate::output::Drawing;
use crate::text::Font;

const MONO: &[u8] = include_bytes!("../fonts/IBMPlexMono-Regular.ttf");

/// The formats [`render`] writes.
pub const FORMATS: [&str; 4] = ["svg", "dxf", "pdf", "png"];

/// The formats written when none are named: the drawing's vector files.
pub const DEFAULT_FORMATS: [&str; 3] = ["svg", "dxf", "pdf"];

/// The PNG's resolution when none is given, pixels per inch (draftwright's).
pub use crate::png::DPI;

/// Rendered files: each format's name and its bytes.
pub type Files = Vec<(String, Vec<u8>)>;

/// The operations [`call`] answers, in the order they are usually used.
pub const OPERATIONS: [&str; 6] = ["analyse", "plan", "edit", "render", "review", "spec"];

/// Why an operation did not complete.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Error {
    /// `invalid` or `failed`.
    pub code: &'static str,
    pub message: String,
    /// What would be accepted, where the request chose from a list.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

impl Error {
    pub fn invalid(message: impl Into<String>) -> Error {
        Error {
            code: "invalid",
            message: message.into(),
            choices: Vec::new(),
        }
    }

    pub fn failed(message: impl Into<String>) -> Error {
        Error {
            code: "failed",
            message: message.into(),
            choices: Vec::new(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if !self.choices.is_empty() {
            write!(f, " Choose one of: {}.", self.choices.join(", "))?;
        }
        Ok(())
    }
}

impl From<model::Unanalysed> for Error {
    fn from(e: model::Unanalysed) -> Error {
        if e.failed {
            Error::failed(e.message)
        } else {
            Error::invalid(e.message)
        }
    }
}

impl From<model::Invalid> for Error {
    fn from(e: model::Invalid) -> Error {
        Error {
            code: "invalid",
            message: e.message,
            choices: e.choices,
        }
    }
}

/// The Analysis of the STEP file at *step*, with the features in *features*, Python quiddity's
/// recognition JSON for the same file ([`crate::bridge`]), when given.
pub fn analyse(step: &Path, features: Option<&Path>) -> Result<Analysis, Error> {
    Ok(model::analyse_with(step, features)?.1)
}

/// The Plan for the STEP file at *step* (with *features*, as for [`analyse`]) under *spec*.
pub fn plan(step: &Path, features: Option<&Path>, spec: &Spec) -> Result<Plan, Error> {
    let (_, analysis) = model::analyse_with(step, features)?;
    Ok(model::plan(&analysis, spec)?)
}

/// *spec* with the edit operations *ops* applied.
pub fn edit(spec: &Spec, ops: &Value) -> Result<Spec, Error> {
    Ok(model::edit(spec, ops)?)
}

/// The analysis, the plan and the drawing of the STEP file at *step* under *spec*. The plan's
/// problems include the drawing's: what could not be drawn as planned.
fn drawn(
    step: &Path,
    features: Option<&Path>,
    spec: &Spec,
) -> Result<(Analysis, Plan, Drawing), Error> {
    let (part, analysis) = model::analyse_with(step, features)?;
    let mut plan = model::plan(&analysis, spec)?;
    let font = Font::new(MONO.to_vec()).expect("the bundled font parses");
    let drawing = model::render(&part, &analysis, &plan, &font).map_err(|e| {
        Error::failed(format!(
            "cannot draw {}: the kernel did not resolve its {e} on reading",
            step.display()
        ))
    })?;
    plan.problems.extend(drawing.problems.iter().cloned());
    Ok((analysis, plan, drawing))
}

/// The plan (its problems with the drawing's), and the drawing in each of *formats* (from
/// [`FORMATS`]) as bytes, each carrying the spec; a PNG at [`DPI`].
pub fn render(
    step: &Path,
    features: Option<&Path>,
    spec: &Spec,
    formats: &[&str],
) -> Result<(Plan, Files), Error> {
    render_at(step, features, spec, formats, DPI)
}

/// [`render`], with a PNG at *dpi* pixels per inch: refused, as draftwright refuses it, when a
/// PNG is asked for and *dpi* is not positive.
pub fn render_at(
    step: &Path,
    features: Option<&Path>,
    spec: &Spec,
    formats: &[&str],
    dpi: f64,
) -> Result<(Plan, Files), Error> {
    if let Some(other) = formats.iter().find(|f| !FORMATS.contains(f)) {
        return Err(Error {
            choices: FORMATS.map(String::from).to_vec(),
            ..Error::invalid(format!("{other:?} is not a format."))
        });
    }
    if formats.contains(&"png") && !(dpi > 0.0 && dpi.is_finite()) {
        return Err(Error::invalid(format!(
            "png export needs dpi > 0, got {dpi}"
        )));
    }
    let (_, plan, drawing) = drawn(step, features, spec)?;
    let files = formats
        .iter()
        .map(|&format| {
            let bytes = match format {
                "svg" => drawing.svg().into_bytes(),
                "dxf" => drawing.dxf(),
                "png" => drawing.png(dpi).map_err(Error::invalid)?,
                _ => {
                    drawing.pdf(&|name| (name == "IBMPlexMono-Regular.ttf").then(|| MONO.to_vec()))
                }
            };
            Ok((format.to_string(), bytes))
        })
        .collect::<Result<_, Error>>()?;
    Ok((plan, files))
}

/// Writes each rendered file at *out* with its format's extension appended (`part.rev2` →
/// `part.rev2.svg`; an *out* already ending in a drawing format's extension has it replaced, as
/// draftwright's `_output_prefix` does), and returns the paths written. Each is written beside its
/// place and then moved in, so a failure leaves any earlier file as it was.
pub fn write(out: &Path, files: &[(String, Vec<u8>)]) -> Result<Vec<String>, Error> {
    let stem = match out.extension().and_then(|e| e.to_str()) {
        Some(e) if FORMATS.contains(&e) => out.with_extension(""),
        _ => out.to_path_buf(),
    };
    let named = |suffix: &str| {
        let mut name = stem.clone().into_os_string();
        name.push(format!(".{suffix}"));
        std::path::PathBuf::from(name)
    };
    let mut written = Vec::new();
    for (format, bytes) in files {
        let path = named(format);
        let staged = named(&format!("{format}.part"));
        std::fs::write(&staged, bytes)
            .map_err(|e| Error::failed(format!("cannot write {}: {e}", staged.display())))?;
        std::fs::rename(&staged, &path)
            .map_err(|e| Error::failed(format!("cannot write {}: {e}", path.display())))?;
        written.push(path.display().to_string());
    }
    Ok(written)
}

/// A drawing to look at: the plan rendered as SVG, with its problems and lint's findings on the
/// drawing.
#[derive(Clone, Debug, Serialize)]
pub struct Review {
    pub svg: String,
    /// The plan's problems, and the drawing's: what could not be drawn as planned.
    pub problems: Vec<Problem>,
    /// What lint finds on the rendered drawing, judged against the part's analysis alone
    /// ([`crate::lint`]): e.g. a hole whose sizes the sheet does not state.
    pub lint: Vec<LintIssue>,
}

/// The drawing of the STEP file at *step* under *spec*, as SVG text, with its problems and
/// lint's findings.
pub fn review(step: &Path, features: Option<&Path>, spec: &Spec) -> Result<Review, Error> {
    let (analysis, plan, drawing) = drawn(step, features, spec)?;
    Ok(Review {
        svg: drawing.svg(),
        problems: plan.problems,
        lint: lint::lint(&analysis, &drawing),
    })
}

/// The spec embedded in the drawing (SVG, DXF, PDF or PNG) at *drawing*.
pub fn spec(drawing: &Path) -> Result<Spec, Error> {
    let bytes = std::fs::read(drawing)
        .map_err(|e| Error::invalid(format!("cannot read {}: {e}", drawing.display())))?;
    model::embedded_spec(&bytes)
        .map_err(|e| {
            Error::invalid(format!(
                "cannot read the spec in {}: {e}",
                drawing.display()
            ))
        })?
        .ok_or_else(|| Error::invalid(format!("{} carries no draftwright spec", drawing.display())))
}

/// Operation *op* with JSON arguments *args*, its result as JSON:
///
/// | op | arguments | result |
/// | --- | --- | --- |
/// | `analyse` | `step`, `features`? | the Analysis |
/// | `plan` | `step`, `features`?, `spec`? | the Plan |
/// | `edit` | `spec`, `ops` | the edited Spec |
/// | `render` | `step`, `features`?, `spec`?, `output`, `formats`?, `dpi`? | `{written, problems}` |
/// | `review` | `step`, `features`?, `spec`? | `{svg, problems, lint}` |
/// | `spec` | `drawing` | the Spec embedded in the drawing |
///
/// `step`, `features`, `output` and `drawing` are file paths; `features` is Python quiddity's
/// recognition JSON for the STEP file ([`crate::bridge`]). An absent `spec` is the default one;
/// absent `formats` are [`DEFAULT_FORMATS`], and an absent `dpi` (the PNG's) is [`DPI`]. A panic
/// inside an operation is answered as `failed`, so a long-lived adapter outlives one bad part.
pub fn call(op: &str, args: &Value) -> Result<Value, Error> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dispatch(op, args))).unwrap_or_else(
        |panic| {
            let why = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            Err(Error::failed(format!("{op} stopped: {why}")))
        },
    )
}

fn dispatch(op: &str, args: &Value) -> Result<Value, Error> {
    let text = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(Path::new)
            .ok_or_else(|| Error::invalid(format!("{op} needs {key:?}, a file path")))
    };
    let features = match args.get("features") {
        None | Some(Value::Null) => None,
        Some(Value::String(path)) => Some(Path::new(path.as_str())),
        Some(_) => {
            return Err(Error::invalid(format!(
                "{op}'s \"features\" is a file path"
            )));
        }
    };
    let given = || -> Result<Spec, Error> {
        match args.get("spec") {
            None | Some(Value::Null) => Ok(Spec::default()),
            Some(s) => serde_json::from_value(s.clone())
                .map_err(|e| Error::invalid(format!("the spec is not valid: {e}"))),
        }
    };
    match op {
        "analyse" => Ok(json_of(&analyse(text("step")?, features)?)),
        "plan" => Ok(json_of(&plan(text("step")?, features, &given()?)?)),
        "edit" => {
            let ops = args
                .get("ops")
                .ok_or_else(|| Error::invalid("edit needs \"ops\", a list of operations"))?;
            Ok(json_of(&edit(&given()?, ops)?))
        }
        "render" => {
            let formats: Vec<&str> = match args.get("formats") {
                None | Some(Value::Null) => DEFAULT_FORMATS.to_vec(),
                Some(Value::Array(list)) => list
                    .iter()
                    .map(|f| {
                        f.as_str()
                            .ok_or_else(|| Error::invalid("\"formats\" is a list of format names"))
                    })
                    .collect::<Result<_, _>>()?,
                Some(_) => return Err(Error::invalid("\"formats\" is a list of format names")),
            };
            let dpi = match args.get("dpi") {
                None | Some(Value::Null) => DPI,
                Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
                Some(_) => return Err(Error::invalid("\"dpi\" is a number of pixels per inch")),
            };
            let output = text("output")?;
            let (plan, files) = render_at(text("step")?, features, &given()?, &formats, dpi)?;
            Ok(json!({ "written": write(output, &files)?, "problems": plan.problems }))
        }
        "review" => Ok(json_of(&review(text("step")?, features, &given()?)?)),
        "spec" => Ok(json_of(&spec(text("drawing")?)?)),
        other => Err(Error {
            choices: OPERATIONS.map(String::from).to_vec(),
            ..Error::invalid(format!("{other:?} is not an operation."))
        }),
    }
}

fn json_of(document: &impl Serialize) -> Value {
    serde_json::to_value(document).expect("a document serialises")
}
