use super::{attributes, exclusions, measure, obligations, templates, Findings};
use anyhow::{Context, Result};
use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

pub(super) fn inspect(
    file: &syn::File,
    source: &str,
    imports: super::imports::View<'_>,
    templates: std::collections::BTreeMap<(usize, usize), super::imports::Template>,
    arguments: super::imports::Arguments,
) -> Result<Findings> {
    if exclusions::test_only(&file.attrs) {
        return Ok(Findings::default());
    }
    let mut found = Findings {
        files: 1,
        ..Findings::default()
    };
    let definitions = templates::Definitions::collect(imports, templates, arguments, &mut found);
    let mut scan = Scan {
        source: measure::Source::new(file, source, &definitions)?,
        found,
        definitions,
        error: None,
    };
    scan.visit_file(file);
    match scan.error {
        Some(error) => Err(error),
        None => Ok(scan.found),
    }
}

struct Scan<'a> {
    source: measure::Source,
    found: Findings,
    error: Option<anyhow::Error>,
    definitions: templates::Definitions<'a>,
}

impl Scan<'_> {
    fn function(&mut self, signature: &syn::Signature, end: Span, body: Option<&syn::Block>) {
        let result = signature
            .span()
            .join(end)
            .context("callable source span unavailable")
            .and_then(|span| {
                let mut boundaries = self.source.boundaries(span, &self.definitions);
                if let Some(body) = body {
                    boundaries.visit_block(body);
                }
                let logical = self.source.logical(span, boundaries);
                self.record(
                    span,
                    &signature.ident.to_string(),
                    signature.inputs.len(),
                    logical,
                )
            });
        if let Err(error) = result {
            self.error = Some(error);
        }
    }

    fn record(&mut self, span: Span, name: &str, arity: usize, logical: usize) -> Result<()> {
        let line = span.start().line;
        if logical > 60 {
            self.found
                .over_60
                .push(format!("{line} {name} ({logical} logical lines > 60)"));
        }
        if logical > 25 {
            self.found
                .lines
                .push(format!("{line} {name} ({logical} logical lines > 25)"));
        }
        if arity > 5 {
            self.found
                .parameters
                .push(format!("{line} {name} ({arity} parameters > 5)"));
        }
        self.found.callables = self
            .found
            .callables
            .checked_add(1)
            .context("callable count overflow")?;
        Ok(())
    }
}

impl<'ast> Visit<'ast> for Scan<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if exclusions::item_excluded(item) {
            return;
        }
        if matches!(item, syn::Item::Verbatim(_)) {
            self.found.unresolved.push(format!(
                "{} opaque handwritten item AST",
                item.span().start().line
            ));
        } else {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        if exclusions::test_only(exclusions::impl_attrs(item)) {
            return;
        }
        if matches!(item, syn::ImplItem::Verbatim(_)) {
            self.found.unresolved.push(format!(
                "{} opaque handwritten impl item AST",
                item.span().start().line
            ));
        } else {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        if exclusions::test_only(exclusions::trait_attrs(item)) {
            return;
        }
        if matches!(item, syn::TraitItem::Verbatim(_)) {
            self.found.unresolved.push(format!(
                "{} opaque handwritten trait item AST",
                item.span().start().line
            ));
        } else {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_foreign_item(&mut self, item: &'ast syn::ForeignItem) {
        if exclusions::test_only(exclusions::foreign_attrs(item)) {
            return;
        }
        if matches!(item, syn::ForeignItem::Verbatim(_)) {
            self.found.unresolved.push(format!(
                "{} opaque handwritten foreign item AST",
                item.span().start().line
            ));
        } else {
            visit::visit_foreign_item(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.function(&item.sig, item.block.span(), Some(&item.block));
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !exclusions::test_only(&item.attrs) {
            self.function(&item.sig, item.block.span(), Some(&item.block));
            visit::visit_impl_item_fn(self, item);
        }
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if !exclusions::test_only(&item.attrs) {
            let end = item.default.as_ref().map_or_else(
                || item.semi_token.map_or(item.sig.span(), |semi| semi.span()),
                Spanned::span,
            );
            self.function(&item.sig, end, item.default.as_ref());
            visit::visit_trait_item_fn(self, item);
        }
    }

    fn visit_foreign_item_fn(&mut self, item: &'ast syn::ForeignItemFn) {
        if !exclusions::test_only(&item.attrs) {
            self.function(&item.sig, item.semi_token.span(), None);
            visit::visit_foreign_item_fn(self, item);
        }
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        if exclusions::test_only(&closure.attrs) {
            return;
        }
        let mut boundaries = self.source.boundaries(closure.span(), &self.definitions);
        boundaries.visit_expr(&closure.body);
        let logical = self.source.logical(closure.span(), boundaries);
        if let Err(error) = self.record(closure.span(), "<closure>", closure.inputs.len(), logical)
        {
            self.error = Some(error);
        }
        visit::visit_expr_closure(self, closure);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if !exclusions::test_only(&local.attrs) {
            visit::visit_local(self, local);
        }
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        if obligations::trusted(invocation, &self.definitions) {
            match self.definitions.take_arguments(invocation.span()) {
                Some(arguments) => templates::visit_expressions(&arguments, self),
                None => self.found.unresolved.push(format!(
                    "{} handwritten macro argument measurement unavailable",
                    invocation.span().start().line
                )),
            }
        } else {
            obligations::invocation(invocation, &mut self.found);
        }
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if item.ident.is_some() && item.mac.path.is_ident("macro_rules") {
            if let Some(bodies) = self.definitions.take(item.mac.span()) {
                for body in bodies {
                    body.visit(self);
                }
            }
        } else {
            visit::visit_item_macro(self, item);
        }
    }

    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if !exclusions::test_only(attributes::expression(expression)) {
            if matches!(expression, syn::Expr::Verbatim(_)) {
                self.found.unresolved.push(format!(
                    "{} opaque handwritten expression AST",
                    expression.span().start().line
                ));
            } else {
                visit::visit_expr(self, expression);
            }
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if !exclusions::test_only(&arm.attrs) {
            visit::visit_arm(self, arm);
        }
    }

    fn visit_field_value(&mut self, field: &'ast syn::FieldValue) {
        if !exclusions::test_only(&field.attrs) {
            visit::visit_field_value(self, field);
        }
    }
}
