use serde_json::{json, Value};

pub(in crate::review) fn latest_sample(progress: &str) -> Option<Value> {
    let block = progress.rsplit("=== ").next()?;
    let sweeps: Vec<u64> =
        find_line(block, "jurisdiction sweeps owed: ").map_or_else(Vec::new, |line| {
            line.split(" of ")
                .filter_map(|part| part.trim().parse().ok())
                .collect()
        });
    let invoations = block
        .lines()
        .find(|line| line.trim_start().starts_with('{'))
        .and_then(|line| serde_json::from_str::<Value>(line).ok());
    let mut owed = Vec::new();
    for line in block.lines() {
        if let Some(rest) = line.trim().split_once("x owes: ") {
            let count: u64 = rest.0.parse().map_or(0, core::convert::identity);
            owed.push(json!({"count": count, "stages": rest.1}));
        }
    }
    let source_objects = find_line(block, "source objects owed: ")
        .and_then(|value| value.trim().parse::<u64>().ok());
    Some(json!({
        "sampled_at": block
            .lines()
            .next()
            .map_or("", str::trim)
            .trim_matches(|character| matches!(character, '=' | ' ' | '"')),
        "sweeps_owed": sweeps.first().copied().map_or(Value::Null, |n| json!(n)),
        "sweeps_total": sweeps.get(1).copied().map_or(Value::Null, |n| json!(n)),
        "source_objects_owed": source_objects.map_or(Value::Null, |n| json!(n)),
        "invocations": invoations.map_or(Value::Null, core::convert::identity),
        "owed_sets": owed
    }))
}

fn find_line<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(prefix))
}
