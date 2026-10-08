use super::{imports::Origin, templates, Findings};
use syn::spanned::Spanned;

pub(super) fn trusted(invocation: &syn::Macro, definitions: &templates::Definitions<'_>) -> bool {
    matches!(
        definitions.imports.origin(invocation),
        Origin::Vendor(_) | Origin::Project(_, _, _, true)
    )
}

pub(super) fn invocation(invocation: &syn::Macro, found: &mut Findings) {
    let name = invocation
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    let line = invocation.span().start().line;
    found.unresolved.push(format!(
        "{line} macro {name}! has unresolved handwritten source provenance or expansion"
    ));
}
