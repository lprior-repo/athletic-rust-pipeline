use super::super::{
    attributes, exclusions,
    imports::{Binding, Catalog, Definition, MacroCall, Origin, Template},
    modules, provenance, templates,
};
use anyhow::{Context, Result};
use proc_macro2::LineColumn;
use std::path::{Path, PathBuf};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

pub(super) struct Collector<'a> {
    pub(super) catalog: &'a mut Catalog,
    pub(super) path: &'a Path,
    pub(super) roots: &'a [PathBuf],
    pub(super) scope: usize,
    pub(super) inline: Vec<String>,
    pub(super) pending: &'a mut Vec<(PathBuf, usize)>,
    pub(super) error: Option<anyhow::Error>,
}

impl Collector<'_> {
    fn binding(&mut self, name: String, binding: Binding) -> Result<()> {
        let binding = match binding {
            Binding::Alias(path, at) => {
                match provenance::resolve(self.catalog, self.scope, path.clone(), at) {
                    Origin::Vendor(_) | Origin::Project(_, _, _, _) => {
                        Binding::MacroAlias(path, at)
                    }
                    Origin::Value => Binding::ValueAlias(path, at),
                    Origin::Unknown => Binding::Alias(path, at),
                }
            }
            binding => binding,
        };
        let names = &mut self.catalog.scope_mut(self.scope)?.names;
        if !matches!(binding, Binding::Other) || !names.contains_key(&name) {
            names.insert(name, binding);
        }
        Ok(())
    }

    fn tree(&mut self, tree: &syn::UseTree, prefix: &[String], at: LineColumn) -> Result<()> {
        let mut pending = vec![(tree, prefix.to_vec())];
        while let Some((tree, mut prefix)) = pending.pop() {
            match tree {
                syn::UseTree::Path(path) => {
                    prefix.push(path.ident.unraw().to_string());
                    pending.push((&path.tree, prefix));
                }
                syn::UseTree::Name(name) => {
                    if name.ident != "self" {
                        prefix.push(name.ident.unraw().to_string());
                    }
                    let name = prefix.last().context("empty macro import path")?.clone();
                    self.binding(name, Binding::Alias(prefix, at))?;
                }
                syn::UseTree::Rename(rename) => {
                    if rename.ident != "self" {
                        prefix.push(rename.ident.unraw().to_string());
                    }
                    self.binding(
                        rename.rename.unraw().to_string(),
                        Binding::Alias(prefix, at),
                    )?;
                }
                syn::UseTree::Group(group) => {
                    pending.extend(group.items.iter().map(|tree| (tree, prefix.clone())))
                }
                syn::UseTree::Glob(_) => self.catalog.scope_mut(self.scope)?.globs.push(prefix),
            }
        }
        Ok(())
    }

    fn module(&mut self, module: &syn::ItemMod) -> Result<()> {
        let previous = self.scope;
        if module.content.is_some() {
            self.scope = self
                .catalog
                .add_scope(self.path, Some(previous), Some(module.span()))?;
            self.catalog.scope_mut(self.scope)?.module = self.scope;
            self.catalog
                .scope_mut(previous)?
                .modules
                .insert(module.ident.unraw().to_string(), self.scope);
            self.inline.push(module.ident.unraw().to_string());
            visit::visit_item_mod(self, module);
            if module
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("macro_use"))
            {
                self.catalog
                    .scope_mut(previous)?
                    .macro_uses
                    .push((module.span().end(), self.scope));
            }
            self.inline.pop();
            self.scope = previous;
        } else {
            let target = modules::target_for(self.path, self.roots, &self.inline, module)?;
            let scope = self.catalog.file_scope(&target, Some(previous))?;
            self.catalog.scope_mut(scope)?.inherited_at = Some(module.span().start());
            self.catalog
                .scope_mut(previous)?
                .modules
                .insert(module.ident.unraw().to_string(), scope);
            self.pending.push((target, scope));
            if module
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("macro_use"))
            {
                self.catalog
                    .scope_mut(previous)?
                    .macro_uses
                    .push((module.span().end(), scope));
            }
        }
        Ok(())
    }

    fn definition(&mut self, item: &syn::ItemMacro) -> Result<()> {
        let Some(name) = &item.ident else {
            return Ok(());
        };
        if !item.mac.path.is_ident("macro_rules") {
            return Ok(());
        }
        let name = name.unraw().to_string();
        let bodies = templates::bodies(&item.mac);
        let at = item.mac.span().start();
        let exported = item
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("macro_export"));
        let definition = Definition {
            at,
            owner: self.scope,
            measured: bodies.is_ok(),
            exported,
        };
        self.catalog
            .scope_mut(self.scope)?
            .macros
            .entry(name.clone())
            .or_default()
            .push(definition);
        if exported {
            let root = self.catalog.scope(self.scope)?.root;
            if root != self.scope {
                self.catalog
                    .scope_mut(root)?
                    .macros
                    .entry(name.clone())
                    .or_default()
                    .push(definition);
            }
        }
        if let Ok(bodies) = &bodies {
            let previous = self.scope;
            for body in bodies {
                self.scope =
                    self.catalog
                        .add_scope(self.path, Some(previous), Some(body.span()))?;
                body.visit(self);
            }
            self.scope = previous;
        }
        self.catalog
            .templates
            .entry(self.path.to_path_buf())
            .or_default()
            .insert((at.line, at.column), Template { name, bodies });
        Ok(())
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if !exclusions::item_excluded(item) {
            visit::visit_item(self, item);
        }
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if let Err(error) = self.tree(&item.tree, &[], item.span().start()) {
            self.error = Some(error);
        }
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if let Err(error) = self.module(module) {
            self.error = Some(error);
        }
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if item.ident.is_some() && item.mac.path.is_ident("macro_rules") {
            if let Err(error) = self.definition(item) {
                self.error = Some(error);
            }
        } else {
            self.visit_macro(&item.mac);
        }
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        let name = item.rename.as_ref().map_or_else(
            || item.ident.unraw().to_string(),
            |(_, name)| name.unraw().to_string(),
        );
        if let Err(error) = self.binding(name, Binding::External(item.ident.unraw().to_string())) {
            self.error = Some(error);
        }
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let previous = self.scope;
        match self
            .catalog
            .add_scope(self.path, Some(previous), Some(block.span()))
        {
            Ok(scope) => {
                self.scope = scope;
                visit::visit_block(self, block);
                self.scope = previous;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if let Err(error) = self.binding(item.sig.ident.unraw().to_string(), Binding::Other) {
            self.error = Some(error);
        }
        visit::visit_item_fn(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        if let Err(error) = self.binding(item.ident.unraw().to_string(), Binding::Other) {
            self.error = Some(error);
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        if let Err(error) = self.binding(item.ident.unraw().to_string(), Binding::Other) {
            self.error = Some(error);
        }
        visit::visit_item_type(self, item);
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        self.catalog.calls.push(MacroCall {
            file: self.path.to_path_buf(),
            scope: self.scope,
            inline: self.inline.clone(),
            invocation: invocation.clone(),
        });
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if !exclusions::test_only(&local.attrs) {
            visit::visit_local(self, local);
        }
    }

    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if !exclusions::test_only(attributes::expression(expression)) {
            visit::visit_expr(self, expression);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        if !exclusions::test_only(exclusions::impl_attrs(item)) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        if !exclusions::test_only(exclusions::trait_attrs(item)) {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_foreign_item(&mut self, item: &'ast syn::ForeignItem) {
        if !exclusions::test_only(exclusions::foreign_attrs(item)) {
            visit::visit_foreign_item(self, item);
        }
    }
}
