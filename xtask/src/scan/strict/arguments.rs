use super::{argument_tokens::nested_arguments, imports::Origin};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::visit::Visit;
use syn::Token;

pub(super) fn inspect(invocation: &syn::Macro, origin: &Origin) -> syn::Result<Vec<syn::Expr>> {
    let tokens = invocation.tokens.clone();
    let Origin::Vendor(path) = origin else {
        return syn::parse2::<RustArguments>(tokens).map(|arguments| arguments.0);
    };
    let name = path
        .rsplit("::")
        .next()
        .ok_or_else(|| syn::Error::new_spanned(&invocation.path, "missing vendor macro name"))?;
    match name {
        "Token" | "stringify" | "concat" | "env" | "option_env" | "include_str"
        | "include_bytes" | "cfg" | "file" | "line" | "column" | "module_path" => Ok(Vec::new()),
        "matches" => syn::parse2::<Matches>(tokens).map(|arguments| arguments.0),
        "vec" => syn::parse2::<Vector>(tokens).map(|arguments| arguments.0),
        "select" | "select_biased" => syn::parse2::<Select>(tokens).map(|arguments| arguments.0),
        "json" => nested_arguments(tokens, true),
        _ => match syn::parse2::<RustArguments>(tokens.clone()) {
            Ok(arguments) => Ok(arguments.0),
            Err(_) => nested_arguments(tokens, false),
        },
    }
}

struct RustArguments(Vec<syn::Expr>);

impl Parse for RustArguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut expressions = Vec::new();
        while !input.is_empty() {
            let ahead = input.fork();
            if let Ok(ty) = ahead.parse::<syn::Type>() {
                if ahead.is_empty() || ahead.peek(Token![,]) {
                    input.advance_to(&ahead);
                    let mut nested = TypeExpressions(Vec::new());
                    nested.visit_type(&ty);
                    expressions.extend(nested.0);
                    if !input.is_empty() {
                        input.parse::<Token![,]>()?;
                    }
                    continue;
                }
            }
            expressions.push(input.parse::<syn::Expr>()?);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self(expressions))
    }
}

struct TypeExpressions(Vec<syn::Expr>);

impl<'ast> Visit<'ast> for TypeExpressions {
    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        self.0.push(expression.clone());
    }
}

struct Matches(Vec<syn::Expr>);

impl Parse for Matches {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut expressions = vec![input.parse()?];
        input.parse::<Token![,]>()?;
        let pattern = syn::Pat::parse_multi_with_leading_vert(input)?;
        let mut nested = TypeExpressions(Vec::new());
        nested.visit_pat(&pattern);
        expressions.extend(nested.0);
        if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            expressions.push(input.parse()?);
        }
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(Self(expressions))
    }
}

struct Vector(Vec<syn::Expr>);

impl Parse for Vector {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut expressions = Vec::new();
        while !input.is_empty() {
            expressions.push(input.parse()?);
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
                expressions.push(input.parse()?);
                break;
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self(expressions))
    }
}

struct Select(Vec<syn::Expr>);

impl Parse for Select {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut expressions = Vec::new();
        if input.peek(syn::Ident) && input.fork().parse::<syn::Ident>()? == "biased" {
            input.parse::<syn::Ident>()?;
            input.parse::<Token![;]>()?;
        }
        while !input.is_empty() {
            if input.peek(Token![else]) {
                input.parse::<Token![else]>()?;
            } else {
                let pattern = syn::Pat::parse_multi_with_leading_vert(input)?;
                let mut nested = TypeExpressions(Vec::new());
                nested.visit_pat(&pattern);
                expressions.extend(nested.0);
            }
            if input.peek(Token![=]) && !input.peek(Token![=>]) {
                input.parse::<Token![=]>()?;
                expressions.push(input.parse()?);
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                    input.parse::<Token![if]>()?;
                    expressions.push(input.parse()?);
                }
            }
            input.parse::<Token![=>]>()?;
            expressions.push(select_handler(input)?);
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self(expressions))
    }
}

fn select_handler(input: ParseStream<'_>) -> syn::Result<syn::Expr> {
    if input.peek(syn::token::Brace) {
        Ok(syn::Expr::Block(syn::ExprBlock {
            attrs: Vec::new(),
            label: None,
            block: input.parse()?,
        }))
    } else {
        input.parse()
    }
}
