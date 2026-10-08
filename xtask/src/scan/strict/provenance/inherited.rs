use super::super::imports::{self, Catalog, Definition, Origin};

pub(super) fn lookup(
    catalog: &Catalog,
    mut scope: usize,
    name: &str,
    mut at: Option<(usize, usize)>,
) -> Result<Option<Definition>, &'static str> {
    for _ in 0..100_000 {
        if let Some(definition) = super::textual::lookup(catalog, scope, name, at)? {
            return Ok(Some(definition));
        }
        let current = catalog
            .scopes
            .get(scope)
            .ok_or("macro lexical scope unavailable")?;
        let Some(parent) = current.parent else {
            return Ok(None);
        };
        let parent_scope = catalog
            .scopes
            .get(parent)
            .ok_or("macro parent scope unavailable")?;
        if current.file != parent_scope.file {
            at = current.inherited_at.map(imports::position);
        }
        scope = parent;
    }
    Err("macro parent definition traversal budget exhausted")
}

pub(super) fn origin(
    catalog: &Catalog,
    request: &super::Request,
    name: &str,
) -> Option<Option<Origin>> {
    let definition = if request.parents {
        lookup(catalog, request.scope, name, request.at).ok()?
    } else {
        catalog
            .scopes
            .get(request.scope)?
            .macros
            .get(name)
            .and_then(|definitions| {
                definitions
                    .iter()
                    .rev()
                    .find(|definition| definition.exported)
            })
            .copied()
    };
    Some(definition.map(|definition| {
        Origin::Project(
            definition.owner,
            name.to_string(),
            imports::position(definition.at),
            definition.measured,
        )
    }))
}
