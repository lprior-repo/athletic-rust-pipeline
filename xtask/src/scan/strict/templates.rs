mod scope;

use super::Findings;
use proc_macro2::{Group, Span, TokenStream, TokenTree};
use std::collections::BTreeMap;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::Token;

pub(super) enum Body {
    File(syn::File),
    Block(syn::Block),
    Expression(syn::Expr),
}

impl Body {
    pub(super) fn span(&self) -> Span {
        match self {
            Self::File(file) => file.span(),
            Self::Block(block) => block.span(),
            Self::Expression(expression) => expression.span(),
        }
    }

    pub(super) fn visit<'ast>(&'ast self, visitor: &mut impl Visit<'ast>) {
        match self {
            Self::File(file) => visitor.visit_file(file),
            Self::Block(block) => visitor.visit_block(block),
            Self::Expression(expression) => visitor.visit_expr(expression),
        }
    }
}

struct Rules(Vec<Group>);

impl Parse for Rules {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut outputs = Vec::new();
        while !input.is_empty() {
            let matcher: TokenTree = input.parse()?;
            let TokenTree::Group(matcher) = matcher else {
                return Err(input.error("macro matcher group required"));
            };
            input.parse::<Token![=>]>()?;
            let output: TokenTree = input.parse()?;
            let TokenTree::Group(group) = output else {
                return Err(input.error("macro output group required"));
            };
            outputs.push(super::syntax::normalize(&matcher, group)?);
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
            } else if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else if !input.is_empty() {
                return Err(input.error("macro rule separator required"));
            }
        }
        Ok(Self(outputs))
    }
}

fn parse_body(group: Group) -> syn::Result<Body> {
    let tokens = group.stream();
    if let Ok(file) = syn::parse2::<syn::File>(tokens.clone()) {
        return Ok(Body::File(file));
    }
    if let Ok(expression) = syn::parse2::<syn::Expr>(tokens) {
        return Ok(Body::Expression(expression));
    }
    syn::parse2::<syn::Block>(TokenStream::from(TokenTree::Group(group))).map(Body::Block)
}

pub(super) fn bodies(invocation: &syn::Macro) -> syn::Result<Vec<Body>> {
    let rules = syn::parse2::<Rules>(invocation.tokens.clone())?;
    rules
        .0
        .into_iter()
        .map(|group| parse_body(group).and_then(scope::validate))
        .collect()
}

pub(super) struct Definitions<'a> {
    pub(super) bodies: BTreeMap<(usize, usize), Vec<Body>>,
    pub(super) unresolved: Vec<String>,
    pub(super) imports: super::imports::View<'a>,
    pub(super) arguments: BTreeMap<(usize, usize), Vec<syn::Expr>>,
    data: Vec<Span>,
}

impl<'a> Definitions<'a> {
    pub(super) fn collect(
        imports: super::imports::View<'a>,
        templates: BTreeMap<(usize, usize), super::imports::Template>,
        arguments: super::imports::Arguments,
        found: &mut Findings,
    ) -> Self {
        let mut definitions = Self {
            imports,
            bodies: BTreeMap::new(),
            unresolved: arguments.unresolved,
            arguments: arguments.expressions,
            data: arguments.data,
        };
        for (at, template) in templates {
            match template.bodies {
                Ok(bodies) => {
                    definitions.bodies.insert(at, bodies);
                }
                Err(error) => definitions.unresolved.push(format!(
                    "{} handwritten macro {} template cannot be measured as Rust AST: {error}",
                    at.0, template.name
                )),
            }
        }
        found.unresolved.append(&mut definitions.unresolved);
        definitions
    }

    pub(super) fn take(&mut self, span: Span) -> Option<Vec<Body>> {
        let start = span.start();
        self.bodies.remove(&(start.line, start.column))
    }
}

impl Definitions<'_> {
    pub(super) fn take_arguments(&mut self, span: Span) -> Option<Vec<syn::Expr>> {
        let start = span.start();
        self.arguments.remove(&(start.line, start.column))
    }
}

pub(super) fn visit_expressions(
    expressions: &[syn::Expr],
    visitor: &mut impl for<'ast> Visit<'ast>,
) {
    for expression in expressions {
        visitor.visit_expr(expression);
    }
}

impl Definitions<'_> {
    pub(super) fn data(&self) -> &[Span] {
        &self.data
    }
}
