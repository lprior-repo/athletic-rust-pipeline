mod inherited;
mod textual;
mod vendor;

use super::imports::{self, Binding, Catalog, Origin};
use proc_macro2::LineColumn;
use std::collections::BTreeSet;
use vendor::STANDARD;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Request {
    scope: usize,
    path: Vec<String>,
    at: Option<(usize, usize)>,
    parents: bool,
    aliases: Vec<(usize, String)>,
    glob: bool,
    searches: Vec<(usize, String)>,
}

pub(super) fn resolve(
    catalog: &Catalog,
    scope: usize,
    path: Vec<String>,
    at: LineColumn,
) -> Origin {
    let standard = path
        .first()
        .filter(|_| path.len() == 1)
        .filter(|name| STANDARD.contains(&name.as_str()))
        .cloned();
    let mut pending = vec![Request {
        scope,
        path,
        at: Some(imports::position(at)),
        parents: true,
        aliases: Vec::new(),
        glob: false,
        searches: Vec::new(),
    }];
    let mut visited = BTreeSet::new();
    let mut origins = BTreeSet::new();
    for _ in 0..1024 {
        let Some(request) = pending.pop() else {
            if origins.len() == 1 {
                return origins
                    .into_iter()
                    .next()
                    .map_or(Origin::Unknown, |origin| origin);
            }
            return if origins.is_empty() {
                standard.map_or(Origin::Unknown, |name| {
                    Origin::Vendor(format!("std::{name}"))
                })
            } else {
                Origin::Unknown
            };
        };
        if !visited.insert(request.clone()) {
            continue;
        }
        if step(catalog, request, &mut pending, &mut origins).is_none() {
            return Origin::Unknown;
        }
    }
    Origin::Unknown
}

fn step(
    catalog: &Catalog,
    request: Request,
    pending: &mut Vec<Request>,
    origins: &mut BTreeSet<Origin>,
) -> Option<()> {
    let scope = catalog.scopes.get(request.scope)?;
    let (name, remaining) = request.path.split_first()?;
    if matches!(name.as_str(), "crate" | "self" | "super") {
        let module = catalog.scopes.get(scope.module)?;
        let target = match name.as_str() {
            "crate" => scope.root,
            "self" => scope.module,
            _ => catalog.scopes.get(module.parent?)?.module,
        };
        descend(target, remaining, &request, pending);
        return Some(());
    }
    if remaining.is_empty() {
        if let Some(origin) = inherited::origin(catalog, &request, name)? {
            origins.insert(origin);
            return Some(());
        }
    }
    if !remaining.is_empty() {
        if let Some(target) = scope.modules.get(name) {
            descend(*target, remaining, &request, pending);
            return Some(());
        }
    }
    if let Some(binding) = binding(catalog, &request, name, remaining) {
        return bind(binding, request, pending, origins);
    }
    glob_requests(catalog, &request, name, pending)?;
    if request.parents {
        if let Some(parent) = scope.parent {
            let at = if catalog.scopes.get(parent)?.file != scope.file {
                scope.inherited_at.map(imports::position)
            } else {
                request.at
            };
            pending.push(Request {
                scope: parent,
                at,
                ..request
            });
        } else if let Some(target) = catalog.roots.get(name) {
            descend(*target, remaining, &request, pending);
        } else if imports::vendor_root(name) && !remaining.is_empty() {
            if vendor::exported(name, remaining) {
                origins.insert(vendor(name, remaining));
            } else if !request.glob {
                origins.insert(Origin::Unknown);
            }
        } else if !remaining.is_empty() && !request.glob {
            origins.insert(Origin::Unknown);
        }
    }
    Some(())
}

fn descend(scope: usize, path: &[String], request: &Request, pending: &mut Vec<Request>) {
    if !path.is_empty() {
        pending.push(Request {
            scope,
            path: path.to_vec(),
            at: None,
            parents: false,
            aliases: request.aliases.clone(),
            glob: request.glob,
            searches: request.searches.clone(),
        });
    }
}

fn bind(
    binding: &Binding,
    request: Request,
    pending: &mut Vec<Request>,
    origins: &mut BTreeSet<Origin>,
) -> Option<()> {
    let (name, remaining) = request.path.split_first()?;
    match binding {
        Binding::Alias(path, at)
        | Binding::MacroAlias(path, at)
        | Binding::ValueAlias(path, at) => {
            let key = (request.scope, name.clone());
            if request.aliases.contains(&key) {
                origins.insert(Origin::Unknown);
                return Some(());
            }
            let mut aliases = request.aliases;
            aliases.push(key);
            pending.push(Request {
                scope: request.scope,
                path: path.iter().chain(remaining).cloned().collect(),
                at: Some(imports::position(*at)),
                parents: true,
                aliases,
                glob: request.glob,
                searches: request.searches,
            });
        }
        Binding::External(root) => {
            if vendor::exported(root, remaining) {
                origins.insert(vendor(root, remaining));
            } else if !request.glob {
                origins.insert(Origin::Unknown);
            }
        }
        Binding::Other => {
            if !remaining.is_empty() {
                origins.insert(Origin::Unknown);
            } else if !request.glob {
                origins.insert(Origin::Value);
            }
        }
    }
    Some(())
}

fn binding<'a>(
    catalog: &'a Catalog,
    request: &Request,
    name: &str,
    remaining: &[String],
) -> Option<&'a Binding> {
    let binding = catalog.scopes.get(request.scope)?.names.get(name)?;
    let external_prefix = request.parents
        && !remaining.is_empty()
        && imports::vendor_root(name)
        && request.aliases.contains(&(request.scope, name.to_string()));
    let macro_prefix = !remaining.is_empty() && matches!(binding, Binding::MacroAlias(_, _));
    if external_prefix
        || macro_prefix
        || (remaining.is_empty()
            && (matches!(binding, Binding::ValueAlias(_, _))
                || (request.parents && matches!(binding, Binding::Other))))
    {
        None
    } else {
        Some(binding)
    }
}

fn glob_requests(
    catalog: &Catalog,
    request: &Request,
    name: &str,
    pending: &mut Vec<Request>,
) -> Option<()> {
    let key = (request.scope, name.to_string());
    if request.searches.contains(&key) {
        return Some(());
    }
    let scope = catalog.scopes.get(request.scope)?;
    let mut searches = request.searches.clone();
    searches.push(key);
    for glob in &scope.globs {
        pending.push(Request {
            scope: request.scope,
            path: glob.iter().chain(&request.path).cloned().collect(),
            at: None,
            parents: true,
            aliases: request.aliases.clone(),
            glob: true,
            searches: searches.clone(),
        });
    }
    Some(())
}

fn vendor(root: &str, path: &[String]) -> Origin {
    if !imports::vendor_root(root) {
        return Origin::Unknown;
    }
    if matches!(root, "std" | "core" | "alloc") {
        if let Some(name) = path.last().filter(|name| STANDARD.contains(&name.as_str())) {
            return Origin::Vendor(format!("std::{name}"));
        }
    }
    Origin::Vendor(
        std::iter::once(root)
            .chain(path.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join("::"),
    )
}
