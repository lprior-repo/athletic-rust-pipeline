use super::Body;

pub(super) fn validate(body: Body) -> syn::Result<Body> {
    let item = match &body {
        Body::File(file) => file.items.iter().find(|item| changes_namespace(item)),
        Body::Block(block) => block.stmts.iter().find_map(|statement| match statement {
            syn::Stmt::Item(item) if changes_namespace(item) => Some(item),
            _ => None,
        }),
        Body::Expression(_) => None,
    };
    if let Some(item) = item {
        return Err(syn::Error::new_spanned(
            item,
            "macro-emitted namespace declarations require expansion-sensitive provenance",
        ));
    }
    Ok(body)
}

fn changes_namespace(item: &syn::Item) -> bool {
    matches!(
        item,
        syn::Item::Use(_) | syn::Item::ExternCrate(_) | syn::Item::Mod(_)
    ) || matches!(item, syn::Item::Macro(item) if item.ident.is_some())
}
