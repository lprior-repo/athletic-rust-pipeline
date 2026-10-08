use super::exclusions;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::ext::IdentExt;
use syn::visit::{self, Visit};

struct Edge {
    parent: PathBuf,
    target: PathBuf,
    test: bool,
}

pub(crate) fn test_files(
    sources: &[(PathBuf, String)],
    roots: &[PathBuf],
) -> Result<BTreeSet<PathBuf>> {
    let mut edges = Vec::new();
    for (path, source) in sources {
        edges.extend(declarations(path, source, roots)?);
    }
    let incoming = incoming_edges(&edges);
    let mut tests = BTreeSet::new();
    for _ in 0..=sources.len() {
        let selected = select_tests(&incoming, &tests, roots);
        if selected == tests {
            return Ok(tests);
        }
        tests = selected;
    }
    anyhow::bail!("test module classification did not converge")
}

fn incoming_edges(edges: &[Edge]) -> BTreeMap<PathBuf, Vec<&Edge>> {
    let mut incoming: BTreeMap<PathBuf, Vec<&Edge>> = BTreeMap::new();
    for edge in edges {
        incoming.entry(edge.target.clone()).or_default().push(edge);
    }
    incoming
}

fn select_tests(
    incoming: &BTreeMap<PathBuf, Vec<&Edge>>,
    tests: &BTreeSet<PathBuf>,
    roots: &[PathBuf],
) -> BTreeSet<PathBuf> {
    incoming
        .iter()
        .filter(|(target, edges)| {
            !roots.contains(target)
                && edges
                    .iter()
                    .all(|edge| edge.test || tests.contains(&edge.parent))
        })
        .map(|(target, _)| target.clone())
        .collect()
}

pub(crate) fn references(path: &Path, source: &str, roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    Ok(declarations(path, source, roots)?
        .into_iter()
        .map(|edge| edge.target)
        .collect())
}

pub(super) fn target_for(
    path: &Path,
    roots: &[PathBuf],
    inline: &[String],
    module: &syn::ItemMod,
) -> Result<PathBuf> {
    Declarations {
        path,
        base: module_base(path, roots)?,
        inline: inline.to_vec(),
        test: false,
        edges: Vec::new(),
        error: None,
    }
    .target(module)
}

fn declarations(path: &Path, source: &str, roots: &[PathBuf]) -> Result<Vec<Edge>> {
    let file = syn::parse_file(source).context("module declaration AST parsing failed")?;
    let mut declarations = Declarations {
        path,
        base: module_base(path, roots)?,
        inline: Vec::new(),
        test: false,
        edges: Vec::new(),
        error: None,
    };
    declarations.visit_file(&file);
    match declarations.error {
        Some(error) => Err(error),
        None => Ok(declarations.edges),
    }
}

struct Declarations<'a> {
    path: &'a Path,
    base: PathBuf,
    inline: Vec<String>,
    test: bool,
    edges: Vec<Edge>,
    error: Option<anyhow::Error>,
}

impl<'ast> Visit<'ast> for Declarations<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let previous = self.test;
        self.test |= exclusions::item_excluded(item);
        visit::visit_item(self, item);
        self.test = previous;
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if module.content.is_some() {
            self.inline.push(module.ident.unraw().to_string());
            visit::visit_item_mod(self, module);
            self.inline.pop();
        } else {
            match self.target(module) {
                Ok(target) => self.edges.push(Edge {
                    parent: self.path.to_path_buf(),
                    target,
                    test: self.test,
                }),
                Err(error) => self.error = Some(error),
            }
        }
    }
}

impl Declarations<'_> {
    fn target(&self, module: &syn::ItemMod) -> Result<PathBuf> {
        let explicit = path_attribute(&module.attrs)?;
        let mut base = if explicit.is_some() && self.inline.is_empty() {
            self.path
                .parent()
                .context("module source has no parent")?
                .to_path_buf()
        } else {
            self.base.clone()
        };
        for name in &self.inline {
            base.push(name);
        }
        match explicit {
            Some(relative) => Ok(normalize(&base.join(relative))),
            None => default_target(&base, &module.ident.unraw().to_string()),
        }
    }
}

fn module_base(path: &Path, roots: &[PathBuf]) -> Result<PathBuf> {
    let mut base = path
        .parent()
        .context("module source has no parent")?
        .to_path_buf();
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("module source name invalid")?;
    if stem != "mod" && !roots.iter().any(|root| root == path) {
        base.push(stem);
    }
    Ok(base)
}

fn default_target(base: &Path, name: &str) -> Result<PathBuf> {
    let direct = base.join(format!("{name}.rs"));
    let nested = base.join(name).join("mod.rs");
    anyhow::ensure!(
        direct.is_file() != nested.is_file(),
        "missing or ambiguous module {name} in {}",
        base.display()
    );
    Ok(if direct.is_file() { direct } else { nested })
}

fn path_attribute(attrs: &[syn::Attribute]) -> Result<Option<String>> {
    let mut path = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("path")) {
        anyhow::ensure!(path.is_none(), "duplicate module path attribute");
        if let syn::Meta::NameValue(value) = &attr.meta {
            if let syn::Expr::Lit(value) = &value.value {
                if let syn::Lit::Str(value) = &value.lit {
                    path = Some(value.value());
                    continue;
                }
            }
        }
        anyhow::bail!("module path needs a Rust string literal");
    }
    Ok(path)
}

fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            _ => result.push(component.as_os_str()),
        }
    }
    result
}
