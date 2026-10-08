use proc_macro2::{TokenStream, TokenTree};

pub(super) fn nested_arguments(tokens: TokenStream, json: bool) -> syn::Result<Vec<syn::Expr>> {
    let mut pending = vec![tokens];
    let mut expressions = Vec::new();
    for _ in 0..100_000 {
        let Some(tokens) = pending.pop() else {
            return Ok(expressions);
        };
        for segment in segments(tokens) {
            if let Ok(expression) = syn::parse2::<syn::Expr>(segment.clone()) {
                expressions.push(expression);
                continue;
            }
            inspect_segment(segment, json, &mut pending, &mut expressions)?;
        }
    }
    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "vendor argument traversal budget exhausted",
    ))
}

fn inspect_segment(
    tokens: TokenStream,
    json: bool,
    pending: &mut Vec<TokenStream>,
    expressions: &mut Vec<syn::Expr>,
) -> syn::Result<()> {
    let mut key = TokenStream::new();
    let mut value = TokenStream::new();
    let mut field = false;
    let mut groups = Vec::new();
    let mut key_groups = Vec::new();
    let mut opaque = None;
    for token in tokens {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == ':' && !field => field = true,
            TokenTree::Group(group) => {
                if field {
                    groups.push(group.stream());
                } else {
                    key_groups.push(group.stream());
                }
                if field {
                    value.extend([TokenTree::Group(group)]);
                } else {
                    key.extend([TokenTree::Group(group)]);
                }
            }
            token => {
                if matches!(&token, TokenTree::Punct(punct) if matches!(punct.as_char(), '|' | ';'))
                    || matches!(&token, TokenTree::Ident(ident) if ident == "fn")
                {
                    opaque = Some(token.span());
                }
                if field {
                    value.extend([token]);
                } else {
                    key.extend([token]);
                }
            }
        }
    }
    if field {
        if json {
            expressions.push(syn::parse2::<syn::Expr>(key)?);
        }
        if let Some(expression) = field_expression(value, json, &groups)? {
            expressions.push(expression);
            return Ok(());
        }
    }
    reject_opaque(opaque)?;
    if !field || !json {
        groups.extend(key_groups);
    }
    pending.extend(groups);
    Ok(())
}

fn field_expression(
    value: TokenStream,
    json: bool,
    groups: &[TokenStream],
) -> syn::Result<Option<syn::Expr>> {
    match syn::parse2::<syn::Expr>(value) {
        Ok(expression) => Ok(Some(expression)),
        Err(error) if groups.is_empty() && json => Err(error),
        Err(_) => Ok(None),
    }
}

fn reject_opaque(span: Option<proc_macro2::Span>) -> syn::Result<()> {
    match span {
        Some(span) => Err(syn::Error::new(
            span,
            "opaque handwritten code in vendor arguments",
        )),
        None => Ok(()),
    }
}

fn segments(tokens: TokenStream) -> Vec<TokenStream> {
    let mut segments = Vec::new();
    let mut segment = TokenStream::new();
    for token in tokens {
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            segments.push(segment);
            segment = TokenStream::new();
        } else {
            segment.extend([token]);
        }
    }
    if !segment.is_empty() {
        segments.push(segment);
    }
    segments
}
