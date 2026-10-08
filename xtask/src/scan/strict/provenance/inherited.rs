use super::super::imports::{self, Catalog, Definition};

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
