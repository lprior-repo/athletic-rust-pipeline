use super::super::imports::{self, Catalog, Definition};
use std::collections::BTreeMap;

enum Event {
    Scope(usize, Option<(usize, usize)>),
    Definition(Definition),
}

pub(super) fn lookup(
    catalog: &Catalog,
    scope: usize,
    name: &str,
    at: Option<(usize, usize)>,
) -> Result<Option<Definition>, &'static str> {
    let mut pending = vec![Event::Scope(scope, at)];
    for _ in 0..100_000 {
        match pending.pop() {
            None => return Ok(None),
            Some(Event::Definition(definition)) => return Ok(Some(definition)),
            Some(Event::Scope(index, at)) => {
                let scope = catalog
                    .scopes
                    .get(index)
                    .ok_or("macro lexical scope unavailable")?;
                let mut events = BTreeMap::new();
                for definition in scope.macros.get(name).into_iter().flatten() {
                    let position = imports::position(definition.at);
                    if definition.owner == index && at.is_none_or(|at| position <= at) {
                        events.insert(position, Event::Definition(*definition));
                    }
                }
                for (position, target) in &scope.macro_uses {
                    let position = imports::position(*position);
                    if at.is_none_or(|at| position <= at) {
                        events.insert(position, Event::Scope(*target, None));
                    }
                }
                pending.extend(events.into_values());
            }
        }
    }
    Err("macro lexical definition traversal budget exhausted")
}
