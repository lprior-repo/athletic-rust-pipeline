mod collector;

use super::{
    exclusions,
    imports::{Catalog, Origin, Parsed},
    templates,
};
use anyhow::{Context, Result};
use collector::Collector;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;

impl Catalog {
    pub(super) fn collect(sources: &[Parsed<'_>], roots: &[PathBuf]) -> Result<Self> {
        let mut catalog = Self::default();
        let mut pending = catalog.root_scopes(roots)?;
        let mut visited = std::collections::BTreeSet::new();
        for _ in 0..100_000 {
            let next = pending
                .pop()
                .map(|(path, scope)| (path, Some(scope)))
                .or_else(|| {
                    sources
                        .iter()
                        .find(|source| !visited.contains(source.path))
                        .map(|source| (source.path.to_path_buf(), None))
                });
            let Some((path, scope)) = next else {
                catalog.classify_aliases()?;
                catalog.collect_arguments(roots)?;
                return Ok(catalog);
            };
            if !visited.insert(path.clone()) {
                continue;
            }
            let source = sources
                .iter()
                .find(|source| source.path == path)
                .context("declared macro module source unavailable")?;
            let scope = match scope {
                Some(scope) => scope,
                None => catalog.file_scope(&path, None)?,
            };
            let mut collector = Collector {
                catalog: &mut catalog,
                path: &path,
                roots,
                scope,
                inline: Vec::new(),
                pending: &mut pending,
                error: None,
            };
            if !exclusions::test_only(&source.file.attrs) {
                collector.visit_file(&source.file);
            }
            if let Some(error) = collector.error {
                return Err(error);
            }
        }
        anyhow::bail!("macro module traversal budget exhausted")
    }

    fn root_scopes(&mut self, roots: &[PathBuf]) -> Result<Vec<(PathBuf, usize)>> {
        let mut pending = Vec::new();
        for root in roots {
            let scope = self.file_scope(root, None)?;
            pending.push((root.clone(), scope));
            if root.file_name().is_some_and(|name| name == "lib.rs") {
                let name = root
                    .parent()
                    .and_then(Path::parent)
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .context("macro crate root name unavailable")?;
                self.roots.insert(name.replace('-', "_"), scope);
            }
        }
        Ok(pending)
    }

    fn file_scope(&mut self, path: &Path, parent: Option<usize>) -> Result<usize> {
        if let Some(scope) = self.files.get(path) {
            return Ok(*scope);
        }
        let scope = self.add_scope(path, parent, None)?;
        self.scope_mut(scope)?.module = scope;
        self.files.insert(path.to_path_buf(), scope);
        Ok(scope)
    }

    fn classify_aliases(&mut self) -> Result<()> {
        let mut selected = Vec::new();
        for (scope, current) in self.scopes.iter().enumerate() {
            for (name, binding) in &current.names {
                let super::imports::Binding::Alias(path, at) = binding else {
                    continue;
                };
                let origin = super::provenance::resolve(self, scope, path.clone(), *at);
                let binding = match origin {
                    Origin::Vendor(_) | Origin::Project(_, _, _, _) => {
                        Some(super::imports::Binding::MacroAlias(path.clone(), *at))
                    }
                    Origin::Value => Some(super::imports::Binding::ValueAlias(path.clone(), *at)),
                    Origin::Unknown => None,
                };
                if let Some(binding) = binding {
                    selected.push((scope, name.clone(), binding));
                }
            }
        }
        for (scope, name, binding) in selected {
            self.scope_mut(scope)?.names.insert(name, binding);
        }
        Ok(())
    }

    fn collect_arguments(&mut self, roots: &[PathBuf]) -> Result<()> {
        for _ in 0..100_000 {
            let Some(call) = self.calls.pop() else {
                return Ok(());
            };
            let origin = self.view(&call.file)?.origin(&call.invocation);
            if !matches!(origin, Origin::Vendor(_) | Origin::Project(_, _, _, true)) {
                continue;
            }
            let at = call.invocation.span().start();
            let mut pending = Vec::new();
            let parsed = super::arguments::inspect(&call.invocation, &origin);
            if let Ok(arguments) = &parsed {
                let mut collector = Collector {
                    catalog: self,
                    path: &call.file,
                    roots,
                    scope: call.scope,
                    inline: call.inline,
                    pending: &mut pending,
                    error: None,
                };
                templates::visit_expressions(arguments, &mut collector);
                if let Some(error) = collector.error {
                    return Err(error);
                }
            }
            anyhow::ensure!(
                pending.is_empty(),
                "external module in handwritten macro arguments was not admitted"
            );
            let cached = self.arguments.entry(call.file).or_default();
            if matches!(&origin, Origin::Vendor(name) if name == "std::stringify") {
                cached.data.push(call.invocation.tokens.span());
            }
            match parsed {
                Ok(arguments) => {
                    cached.expressions.insert((at.line, at.column), arguments);
                }
                Err(error) => cached.unresolved.push(format!(
                    "{} handwritten macro arguments cannot be measured as Rust AST: {error}",
                    at.line
                )),
            }
        }
        anyhow::bail!("handwritten macro argument traversal budget exhausted")
    }
}
