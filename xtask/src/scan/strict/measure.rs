use super::exclusions;
use anyhow::{Context, Result};
use proc_macro2::{LineColumn, Span, TokenStream, TokenTree};
use std::collections::BTreeSet;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

pub(super) struct Source {
    positions: Vec<LineColumn>,
    excluded: exclusions::Exclusions,
}

impl Source {
    pub(super) fn new(
        file: &syn::File,
        source: &str,
        definitions: &super::templates::Definitions<'_>,
    ) -> Result<Self> {
        let mut excluded = exclusions::Exclusions::default();
        excluded.visit_file(file);
        excluded.0.extend_from_slice(definitions.data());
        for bodies in definitions.bodies.values() {
            for body in bodies {
                body.visit(&mut excluded);
            }
        }
        for arguments in definitions.arguments.values() {
            for argument in arguments {
                excluded.visit_expr(argument);
            }
        }
        let start = file.shebang.as_ref().map_or(0, String::len);
        let tokens = source
            .get(start..)
            .context("invalid shebang span")?
            .parse::<TokenStream>()
            .map_err(|error| anyhow::anyhow!("strict Rust token parsing failed: {error}"))?;
        let mut positions = token_positions(tokens);
        positions.retain(|position| !excluded.0.iter().any(|span| within(*position, *span)));
        Ok(Self {
            positions,
            excluded,
        })
    }

    pub(super) fn logical(&self, span: Span, boundaries: Boundaries<'_>) -> usize {
        let lines: BTreeSet<_> = self
            .positions
            .iter()
            .filter(|position| within(**position, span))
            .map(|position| position.line)
            .collect();
        lines.len().max(boundaries.positions.len())
    }

    pub(super) fn boundaries<'a>(
        &'a self,
        span: Span,
        definitions: &'a super::templates::Definitions<'a>,
    ) -> Boundaries<'a> {
        let mut boundaries = Boundaries {
            positions: BTreeSet::new(),
            excluded: &self.excluded,
            definitions,
        };
        boundaries.anchor(span);
        boundaries.positions.insert(position(span.end()));
        boundaries
    }
}

fn token_positions(tokens: TokenStream) -> Vec<LineColumn> {
    let mut positions = Vec::new();
    let mut pending = vec![tokens.into_iter()];
    while let Some(tokens) = pending.last_mut() {
        match tokens.next() {
            Some(TokenTree::Group(group)) => {
                positions.push(group.span_open().start());
                positions.push(group.span_close().start());
                pending.push(group.stream().into_iter());
            }
            Some(token) => positions.push(token.span().start()),
            None => {
                pending.pop();
            }
        }
    }
    positions
}

fn position(at: LineColumn) -> (usize, usize) {
    (at.line, at.column)
}

fn within(at: LineColumn, span: Span) -> bool {
    position(at) >= position(span.start()) && position(at) < position(span.end())
}

pub(super) struct Boundaries<'a> {
    positions: BTreeSet<(usize, usize)>,
    excluded: &'a exclusions::Exclusions,
    definitions: &'a super::templates::Definitions<'a>,
}

impl Boundaries<'_> {
    fn anchor(&mut self, span: Span) {
        if !self
            .excluded
            .0
            .iter()
            .any(|excluded| within(span.start(), *excluded))
        {
            self.positions.insert(position(span.start()));
        }
    }
}

impl<'ast, 'a: 'ast> Visit<'ast> for Boundaries<'a> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if !exclusions::item_excluded(item) {
            visit::visit_item(self, item);
        }
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if !matches!(statement, syn::Stmt::Item(item) if exclusions::item_excluded(item)) {
            self.anchor(statement.span());
            visit::visit_stmt(self, statement);
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.anchor(arm.span());
        visit::visit_arm(self, arm);
    }

    fn visit_field_value(&mut self, field: &'ast syn::FieldValue) {
        self.anchor(field.span());
        visit::visit_field_value(self, field);
    }

    fn visit_macro(&mut self, invocation: &'ast syn::Macro) {
        let start = invocation.span().start();
        if let Some(arguments) = self.definitions.arguments.get(&(start.line, start.column)) {
            for argument in arguments {
                self.visit_expr(argument);
            }
        }
    }
}
