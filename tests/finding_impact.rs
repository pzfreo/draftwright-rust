//! Source inventory, including untaken/refusal paths: a fixture-only survey misses new codes.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::parse::Parser;
use syn::visit::{self, Visit};
use syn::{Expr, FnArg, Item, Lit, Pat};

fn sources(path: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}
fn testing(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("test")
            || (a.path().is_ident("cfg")
                && matches!(&a.meta, syn::Meta::List(l) if l.tokens.to_string() == "test"))
    })
}
fn values(e: &Expr, out: &mut BTreeSet<String>) {
    match e {
        Expr::Lit(e) => {
            if let Lit::Str(s) = &e.lit {
                out.insert(s.value());
            }
        }
        Expr::If(e) => {
            if let Some(syn::Stmt::Expr(e, _)) = e.then_branch.stmts.last() {
                values(e, out);
            }
            if let Some((_, e)) = &e.else_branch {
                values(e, out);
            }
        }
        Expr::Match(e) => {
            for arm in &e.arms {
                values(&arm.body, out);
            }
        }
        Expr::Block(e) => {
            if let Some(syn::Stmt::Expr(e, _)) = e.block.stmts.last() {
                values(e, out);
            }
        }
        Expr::MethodCall(e) => values(&e.receiver, out),
        Expr::Reference(e) => values(&e.expr, out),
        Expr::Paren(e) => values(&e.expr, out),
        Expr::Tuple(e) => {
            if let Some(e) = e.elems.first() {
                values(e, out);
            }
        }
        _ => {}
    }
}
#[derive(Default)]
struct Inventory {
    local_arguments: BTreeMap<String, BTreeSet<usize>>,
    arguments: BTreeMap<String, BTreeSet<usize>>,
    codes: BTreeSet<String>,
    families: BTreeSet<String>,
    states: BTreeSet<String>,
}
impl<'ast> Visit<'ast> for Inventory {
    fn visit_item(&mut self, item: &'ast Item) {
        let attrs = match item {
            Item::Fn(i) => &i.attrs,
            Item::Mod(i) => &i.attrs,
            _ => {
                visit::visit_item(self, item);
                return;
            }
        };
        if !testing(attrs) {
            visit::visit_item(self, item);
        }
    }
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        if testing(&f.attrs) {
            return;
        }
        self.local_arguments
            .entry(f.sig.ident.to_string())
            .or_default();
        for (i, arg) in f.sig.inputs.iter().enumerate() {
            if matches!(arg, FnArg::Typed(a) if matches!(a.pat.as_ref(), Pat::Ident(p) if p.ident == "code"))
            {
                self.local_arguments
                    .entry(f.sig.ident.to_string())
                    .or_default()
                    .insert(i);
                self.arguments
                    .entry(f.sig.ident.to_string())
                    .or_default()
                    .insert(i);
            }
        }
        visit::visit_item_fn(self, f);
    }
    fn visit_expr_struct(&mut self, e: &'ast syn::ExprStruct) {
        if e.path
            .segments
            .last()
            .is_some_and(|s| s.ident == "Problem" || s.ident == "LintIssue")
        {
            for f in &e.fields {
                if matches!(&f.member, syn::Member::Named(n) if n == "code") {
                    values(&f.expr, &mut self.codes);
                }
            }
        }
        visit::visit_expr_struct(self, e);
    }
    fn visit_local(&mut self, l: &'ast syn::Local) {
        let pat = if let Pat::Type(p) = &l.pat {
            p.pat.as_ref()
        } else {
            &l.pat
        };
        let code = match pat {
            Pat::Ident(p) => p.ident == "code",
            Pat::Tuple(p) => matches!(p.elems.first(), Some(Pat::Ident(p)) if p.ident == "code"),
            _ => false,
        };
        if let (Pat::Ident(p), Some(init)) = (pat, &l.init) {
            if let Expr::Closure(c) = init.expr.as_ref() {
                for (i, arg) in c.inputs.iter().enumerate() {
                    let arg = if let Pat::Type(t) = arg {
                        t.pat.as_ref()
                    } else {
                        arg
                    };
                    if matches!(arg, Pat::Ident(p) if p.ident == "code") {
                        self.local_arguments
                            .entry(p.ident.to_string())
                            .or_default()
                            .insert(i);
                    }
                }
            }
        }
        if code {
            if let Some(init) = &l.init {
                values(&init.expr, &mut self.codes);
            }
        }
        visit::visit_local(self, l);
    }
    fn visit_expr_assign(&mut self, e: &'ast syn::ExprAssign) {
        if matches!(e.left.as_ref(), Expr::Field(f) if matches!(&f.member, syn::Member::Named(n) if n == "code"))
        {
            values(&e.right, &mut self.codes);
        }
        visit::visit_expr_assign(self, e);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let name = m.path.segments.last().unwrap().ident.to_string();
        if name == "vec" || name == "problem" {
            if let Ok(args) = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                .parse2(m.tokens.clone())
            {
                if name == "problem" {
                    if let Some(e) = args.first() {
                        values(e, &mut self.codes);
                    }
                }
                for e in &args {
                    self.visit_expr(e);
                }
            }
        }
    }
    fn visit_expr_call(&mut self, e: &'ast syn::ExprCall) {
        if let Expr::Path(p) = e.func.as_ref() {
            let name = p.path.segments.last().unwrap().ident.to_string();
            if let Some(indices) = self
                .local_arguments
                .get(&name)
                .or_else(|| self.arguments.get(&name))
            {
                for i in indices {
                    if let Some(arg) = e.args.iter().nth(*i) {
                        values(arg, &mut self.codes);
                    }
                }
            }
            if name == "Refusal" {
                if let Some(arg) = e.args.first() {
                    values(arg, &mut self.codes);
                }
            }
            if p.path.segments.iter().any(|s| s.ident == "parameters") && name == "coverage" {
                values(e.args.first().unwrap(), &mut self.families);
            }
            // parameters::coverage is the only code template; enumerate its actual states.
            if name == "issue" && e.args.len() == 4 {
                let mut states = BTreeSet::new();
                values(e.args.first().unwrap(), &mut states);
                self.states.extend(
                    states
                        .into_iter()
                        .filter(|s| s == "missing" || s == "unverifiable"),
                );
            }
        }
        visit::visit_expr_call(self, e);
    }
}
#[test]
fn every_emitted_code_has_an_impact_and_a_documented_reason() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    sources(&root.join("src"), &mut paths);
    let files: Vec<_> = paths
        .iter()
        .filter(|p| p.file_name().unwrap() != "finding.rs")
        .map(|p| syn::parse_file(&std::fs::read_to_string(p).unwrap()).unwrap())
        .collect();
    let mut inventory = Inventory::default();
    for file in &files {
        inventory.local_arguments.clear();
        inventory.visit_file(file);
        inventory.visit_file(file);
    }
    // Second pass catches helper calls preceding declarations, including cross-module ones.
    for file in &files {
        inventory.local_arguments.clear();
        inventory.visit_file(file);
        inventory.visit_file(file);
    }
    for family in &inventory.families {
        for state in &inventory.states {
            inventory
                .codes
                .insert(format!("{family}_requirement_{state}"));
        }
    }
    inventory
        .codes
        .retain(|c| c.contains('_') && c.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'));
    let docs = std::fs::read_to_string(root.join("docs/finding-impact.md")).unwrap();
    for code in &inventory.codes {
        let p = draftwright::model::Problem {
            code: code.clone(),
            severity: "warning".into(),
            ..Default::default()
        };
        assert!(
            docs.contains(&format!("| `{code}` |")),
            "undocumented emitted code: {code}"
        );
        assert!(serde_json::to_value(p).unwrap()["impact"].is_string());
    }
    // The production table is checked against this inventory by its own unit test.
    let catalogue: BTreeSet<_> = docs
        .lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split('`').next())
        .map(String::from)
        .collect();
    assert_eq!(
        inventory.codes, catalogue,
        "source inventory and impact catalogue differ"
    );
}
