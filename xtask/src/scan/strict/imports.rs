use super::{provenance, templates};
use anyhow::{Context, Result};
use proc_macro2::{LineColumn, Span};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use syn::ext::IdentExt;
use syn::spanned::Spanned;

pub(super) struct Parsed<'a> {
    pub(super) path: &'a Path,
    pub(super) source: &'a str,
    pub(super) file: syn::File,
}

#[derive(Default)]
pub(super) struct Catalog {
    pub(super) scopes: Vec<Scope>,
    pub(super) files: BTreeMap<PathBuf, usize>,
    pub(super) roots: BTreeMap<String, usize>,
    pub(super) templates: BTreeMap<PathBuf, BTreeMap<(usize, usize), Template>>,
    pub(super) calls: Vec<MacroCall>,
    pub(super) arguments: BTreeMap<PathBuf, Arguments>,
}

pub(super) struct Scope {
    pub(super) file: PathBuf,
    pub(super) parent: Option<usize>,
    pub(super) module: usize,
    pub(super) root: usize,
    pub(super) span: Option<Span>,
    pub(super) inherited_at: Option<LineColumn>,
    pub(super) modules: BTreeMap<String, usize>,
    pub(super) names: BTreeMap<String, Binding>,
    pub(super) macros: BTreeMap<String, Vec<Definition>>,
    pub(super) globs: Vec<Vec<String>>,
    pub(super) macro_uses: Vec<(LineColumn, usize)>,
}

#[derive(Clone, Copy)]
pub(super) struct Definition {
    pub(super) at: LineColumn,
    pub(super) owner: usize,
    pub(super) measured: bool,
    pub(super) exported: bool,
}

pub(super) enum Binding {
    Alias(Vec<String>, LineColumn),
    MacroAlias(Vec<String>, LineColumn),
    ValueAlias(Vec<String>, LineColumn),
    External(String),
    Other,
}

pub(super) struct Template {
    pub(super) name: String,
    pub(super) bodies: syn::Result<Vec<templates::Body>>,
}

pub(super) struct MacroCall {
    pub(super) file: PathBuf,
    pub(super) scope: usize,
    pub(super) inline: Vec<String>,
    pub(super) invocation: syn::Macro,
}

#[derive(Default)]
pub(super) struct Arguments {
    pub(super) expressions: BTreeMap<(usize, usize), Vec<syn::Expr>>,
    pub(super) data: Vec<Span>,
    pub(super) unresolved: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum Origin {
    Vendor(String),
    Project(usize, String, (usize, usize), bool),
    Value,
    Unknown,
}

pub(super) struct View<'a> {
    pub(super) catalog: &'a Catalog,
    pub(super) file: &'a Path,
    pub(super) scope: usize,
}

pub(super) fn vendor_root(root: &str) -> bool {
    matches!(
        root,
        "std"
            | "core"
            | "alloc"
            | "anyhow"
            | "tracing"
            | "serde"
            | "serde_json"
            | "restate_sdk"
            | "restate"
            | "tokio"
            | "clap"
            | "syn"
            | "futures"
    )
}

impl Catalog {
    pub(super) fn view<'a>(&'a self, path: &'a Path) -> Result<View<'a>> {
        Ok(View {
            catalog: self,
            file: path,
            scope: *self.files.get(path).context("macro source scope missing")?,
        })
    }

    pub(super) fn take_templates(&mut self, path: &Path) -> BTreeMap<(usize, usize), Template> {
        self.templates
            .remove(path)
            .map_or(BTreeMap::new(), core::convert::identity)
    }

    pub(super) fn take_arguments(&mut self, path: &Path) -> Arguments {
        self.arguments
            .remove(path)
            .map_or(Arguments::default(), core::convert::identity)
    }

    pub(super) fn scope(&self, index: usize) -> Result<&Scope> {
        self.scopes
            .get(index)
            .context("macro scope reference invalid")
    }

    pub(super) fn scope_mut(&mut self, index: usize) -> Result<&mut Scope> {
        self.scopes
            .get_mut(index)
            .context("macro scope reference invalid")
    }

    pub(super) fn add_scope(
        &mut self,
        file: &Path,
        parent: Option<usize>,
        span: Option<Span>,
    ) -> Result<usize> {
        let index = self.scopes.len();
        let (root, module) = match parent {
            Some(parent) => {
                let parent = self.scope(parent)?;
                (parent.root, parent.module)
            }
            None => (index, index),
        };
        self.scopes.try_reserve(1)?;
        self.scopes.push(Scope {
            file: file.to_path_buf(),
            parent,
            module,
            root,
            span,
            inherited_at: None,
            modules: BTreeMap::new(),
            names: BTreeMap::new(),
            macros: BTreeMap::new(),
            globs: Vec::new(),
            macro_uses: Vec::new(),
        });
        Ok(index)
    }
}

impl View<'_> {
    pub(super) fn origin(&self, invocation: &syn::Macro) -> Origin {
        let at = invocation.span().start();
        let scope = self
            .catalog
            .scopes
            .iter()
            .enumerate()
            .filter(|(_, scope)| {
                scope.file == self.file && scope.span.is_some_and(|span| contains(span, at))
            })
            .max_by_key(|(_, scope)| scope.span.map(|span| position(span.start())))
            .map_or(self.scope, |(index, _)| index);
        let path = invocation
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.unraw().to_string())
            .collect();
        provenance::resolve(self.catalog, scope, path, at)
    }
}

fn contains(span: Span, at: LineColumn) -> bool {
    position(span.start()) <= position(at) && position(at) < position(span.end())
}

pub(super) fn position(at: LineColumn) -> (usize, usize) {
    (at.line, at.column)
}
