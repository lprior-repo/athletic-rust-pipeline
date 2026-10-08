use proc_macro2::{Group, Ident, Span, TokenStream, TokenTree};
use std::collections::BTreeMap;
use syn::ext::IdentExt;

pub(super) fn normalize(matcher: &Group, output: Group) -> syn::Result<Group> {
    let fragments = fragments(matcher.stream());
    let mut frames = vec![Frame::new(output, false)];
    for _ in 0..100_000 {
        let Some(frame) = frames.last_mut() else {
            break;
        };
        match frame.tokens.next() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == '$' => {
                let token = frame
                    .tokens
                    .next()
                    .ok_or_else(|| syn::Error::new(punct.span(), "missing macro fragment"))?;
                match token {
                    TokenTree::Ident(ident) => frame.output.extend([fragment(ident, &fragments)?]),
                    TokenTree::Group(group) => {
                        repetition(frame, group.span())?;
                        let mut repeated = Frame::new(group, true);
                        repeated.repeated = true;
                        frames.push(repeated);
                    }
                    token => {
                        return Err(syn::Error::new(
                            token.span(),
                            "unsupported macro substitution",
                        ))
                    }
                }
            }
            Some(TokenTree::Group(group)) => {
                let arguments = matches!(frame.previous.as_ref(), Some(TokenTree::Punct(punct)) if punct.as_char() == '!');
                frame.previous = Some(TokenTree::Group(group.clone()));
                frames.push(Frame::new(group, arguments));
            }
            Some(token) => {
                frame.previous = Some(token.clone());
                frame.output.extend([token]);
            }
            None => {
                let group = frames
                    .pop()
                    .ok_or_else(|| syn::Error::new(Span::call_site(), "missing template frame"))?
                    .finish()?;
                match frames.last_mut() {
                    Some(parent) => parent.output.extend([TokenTree::Group(group)]),
                    None => return Ok(group),
                }
            }
        }
    }
    Err(syn::Error::new(
        Span::call_site(),
        "macro template traversal budget exhausted",
    ))
}

struct Frame {
    tokens: proc_macro2::token_stream::IntoIter,
    output: TokenStream,
    previous: Option<TokenTree>,
    span: Span,
    delimiter: proc_macro2::Delimiter,
    arguments: bool,
    repeated: bool,
}

impl Frame {
    fn new(group: Group, arguments: bool) -> Self {
        Self {
            tokens: group.stream().into_iter(),
            output: TokenStream::new(),
            previous: None,
            span: group.span(),
            delimiter: group.delimiter(),
            arguments,
            repeated: false,
        }
    }

    fn finish(self) -> syn::Result<Group> {
        if self.repeated {
            syn::parse2::<syn::Expr>(self.output.clone())?;
        }
        let delimiter = if self.repeated {
            proc_macro2::Delimiter::None
        } else {
            self.delimiter
        };
        let mut group = Group::new(delimiter, self.output);
        group.set_span(self.span);
        Ok(group)
    }
}

fn repetition(frame: &mut Frame, span: Span) -> syn::Result<()> {
    if !frame.arguments {
        return Err(syn::Error::new(
            span,
            "callable template repetition outside macro arguments cannot be measured",
        ));
    }
    let separator = frame
        .tokens
        .next()
        .ok_or_else(|| syn::Error::new(span, "missing repetition operator"))?;
    let operator = if matches!(&separator, TokenTree::Punct(punct) if punct.as_char() == ',') {
        frame
            .tokens
            .next()
            .ok_or_else(|| syn::Error::new(span, "missing repetition operator"))?
    } else {
        separator
    };
    if !matches!(operator, TokenTree::Punct(punct) if matches!(punct.as_char(), '*' | '+' | '?')) {
        return Err(syn::Error::new(
            span,
            "unsupported macro repetition separator",
        ));
    }
    Ok(())
}

fn fragment(ident: Ident, fragments: &BTreeMap<String, String>) -> syn::Result<TokenTree> {
    let name = ident.unraw().to_string();
    if name == "crate" {
        return Ok(TokenTree::Ident(ident));
    }
    match fragments.get(&name).map(String::as_str) {
        Some("ty" | "expr" | "path") => Ok(TokenTree::Ident(Ident::new("_", ident.span()))),
        _ => Err(syn::Error::new(
            ident.span(),
            "opaque handwritten macro fragment cannot be measured",
        )),
    }
}

fn fragments(tokens: TokenStream) -> BTreeMap<String, String> {
    let mut pending = vec![tokens.into_iter()];
    let mut fragments = BTreeMap::new();
    while let Some(tokens) = pending.last_mut() {
        match tokens.next() {
            Some(TokenTree::Group(group)) => pending.push(group.stream().into_iter()),
            Some(TokenTree::Punct(punct)) if punct.as_char() == '$' => match tokens.next() {
                Some(TokenTree::Group(group)) => pending.push(group.stream().into_iter()),
                Some(TokenTree::Ident(name)) => {
                    if let (Some(TokenTree::Punct(colon)), Some(TokenTree::Ident(kind))) =
                        (tokens.next(), tokens.next())
                    {
                        if colon.as_char() == ':' {
                            fragments.insert(name.unraw().to_string(), kind.to_string());
                        }
                    }
                }
                _ => {}
            },
            Some(_) => {}
            None => {
                pending.pop();
            }
        }
    }
    fragments
}
