mod inherited;
mod textual;

use super::imports::{self, Binding, Catalog, Origin};
use proc_macro2::LineColumn;
use std::collections::BTreeSet;

const STANDARD: &[&str] = &[
    "format",
    "format_args",
    "println",
    "eprintln",
    "print",
    "eprint",
    "write",
    "writeln",
    "vec",
    "matches",
    "concat",
    "stringify",
    "env",
    "option_env",
    "file",
    "line",
    "column",
    "module_path",
    "include_str",
    "include_bytes",
    "cfg",
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "panic",
    "todo",
    "unreachable",
    "unimplemented",
];

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Request {
    scope: usize,
    path: Vec<String>,
    at: Option<(usize, usize)>,
    parents: bool,
    aliases: Vec<(usize, String)>,
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
        descend(target, remaining, &request.aliases, pending);
        return Some(());
    }
    if remaining.is_empty() {
        let definition = if request.parents {
            match inherited::lookup(catalog, request.scope, name, request.at) {
                Ok(definition) => definition,
                Err(_) => return None,
            }
        } else {
            scope
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
        if let Some(definition) = definition {
            origins.insert(Origin::Project(
                definition.owner,
                name.clone(),
                imports::position(definition.at),
                definition.measured,
            ));
            return Some(());
        }
    }
    if let Some(binding) = scope.names.get(name) {
        if !remaining.is_empty() || !matches!(binding, Binding::Other) {
            return bind(binding, request, pending, origins);
        }
    }
    for glob in &scope.globs {
        pending.push(Request {
            scope: request.scope,
            path: glob.iter().chain(&request.path).cloned().collect(),
            at: None,
            parents: true,
            aliases: request.aliases.clone(),
        });
    }
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
            descend(*target, remaining, &request.aliases, pending);
        } else if imports::vendor_root(name) && !remaining.is_empty() {
            if vendor_macro(name, remaining.last()?) {
                origins.insert(vendor(name, remaining));
            }
        } else if !remaining.is_empty() {
            origins.insert(Origin::Unknown);
        }
    }
    Some(())
}

fn descend(scope: usize, path: &[String], aliases: &[(usize, String)], pending: &mut Vec<Request>) {
    if !path.is_empty() {
        pending.push(Request {
            scope,
            path: path.to_vec(),
            at: None,
            parents: false,
            aliases: aliases.to_vec(),
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
        Binding::Module(target) => descend(*target, remaining, &request.aliases, pending),
        Binding::Alias(path, at) => {
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
            });
        }
        Binding::External(root) => {
            origins.insert(vendor(root, remaining));
        }
        Binding::Other => {
            if !remaining.is_empty() {
                origins.insert(Origin::Unknown);
            }
        }
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

fn vendor_macro(root: &str, name: &str) -> bool {
    match root {
        "std" | "core" | "alloc" => STANDARD.contains(&name) || name == "pin",
        "anyhow" => matches!(name, "anyhow" | "bail" | "ensure"),
        "tracing" => matches!(
            name,
            "info"
                | "warn"
                | "error"
                | "debug"
                | "trace"
                | "event"
                | "span"
                | "info_span"
                | "warn_span"
                | "error_span"
                | "debug_span"
                | "trace_span"
                | "enabled"
                | "event_enabled"
                | "span_enabled"
        ),
        "serde_json" => name == "json",
        "syn" => name == "Token",
        "tokio" => matches!(name, "select" | "join" | "try_join" | "pin"),
        "futures" => matches!(
            name,
            "pin_mut"
                | "select"
                | "select_biased"
                | "join"
                | "try_join"
                | "pending"
                | "poll"
                | "ready"
        ),
        "clap" => matches!(
            name,
            "arg"
                | "command"
                | "value_parser"
                | "crate_version"
                | "crate_authors"
                | "crate_name"
                | "crate_description"
        ),
        _ => true,
    }
}
