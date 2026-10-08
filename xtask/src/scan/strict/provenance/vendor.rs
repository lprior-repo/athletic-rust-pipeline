pub(super) const STANDARD: &[&str] = &[
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

pub(super) fn exported(root: &str, path: &[String]) -> bool {
    let [name] = path else {
        return false;
    };
    match root {
        "std" | "core" | "alloc" => STANDARD.contains(&name.as_str()) || name == "pin",
        "anyhow" => matches!(name.as_str(), "anyhow" | "bail" | "ensure"),
        "tracing" => tracing(name),
        "serde_json" => name == "json",
        "syn" => name == "Token",
        "tokio" => matches!(name.as_str(), "select" | "join" | "try_join" | "pin"),
        "restate_sdk" | "restate" => name == "select",
        "futures" => matches!(
            name.as_str(),
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
            name.as_str(),
            "arg"
                | "command"
                | "value_parser"
                | "crate_version"
                | "crate_authors"
                | "crate_name"
                | "crate_description"
        ),
        _ => false,
    }
}

fn tracing(name: &str) -> bool {
    matches!(
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
    )
}
