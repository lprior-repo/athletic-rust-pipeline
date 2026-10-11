use serde_json::{json, Value};

pub(in crate::review) fn latest_sample(progress: &str) -> Option<Value> {
    let block = progress.rsplit("=== ").next()?;
    let (sweeps_owed, sweeps_total) =
        find_line(block, "jurisdiction sweeps owed: ").map_or((None, None), sweep_pair);
    let invoations = block
        .lines()
        .find(|line| line.trim_start().starts_with('{'))
        .and_then(|line| serde_json::from_str::<Value>(line).ok());
    let mut owed = Vec::new();
    for line in block.lines() {
        if let Some(rest) = line.trim().split_once("x owes: ") {
            let count = rest.0.trim().parse::<u64>().ok();
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
        "sweeps_owed": sweeps_owed.map_or(Value::Null, |n| json!(n)),
        "sweeps_total": sweeps_total.map_or(Value::Null, |n| json!(n)),
        "source_objects_owed": source_objects.map_or(Value::Null, |n| json!(n)),
        "invocations": invoations.map_or(Value::Null, core::convert::identity),
        "owed_sets": owed
    }))
}

fn sweep_pair(line: &str) -> (Option<u64>, Option<u64>) {
    let mut parts = line.splitn(2, " of ");
    (
        parts.next().and_then(parse_trimmed),
        parts.next().and_then(parse_trimmed),
    )
}

fn parse_trimmed(part: &str) -> Option<u64> {
    part.trim().parse().ok()
}

fn find_line<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(prefix))
}

#[cfg(test)]
mod tests {
    use super::latest_sample;
    use serde_json::Value;

    const PARSEABLE: &str = "=== 2026-10-09T15:41:21Z ===\nseason 2026-27 revision 1\njurisdiction sweeps owed: 49 of 50\nsource objects owed: 56227\n";

    #[test]
    fn a_parsable_sweeps_line_supplies_both_bounds() {
        let Some(sample) = latest_sample(PARSEABLE) else {
            return;
        };
        assert_eq!(sample.get("sweeps_owed").and_then(Value::as_u64), Some(49));
        assert_eq!(sample.get("sweeps_total").and_then(Value::as_u64), Some(50));
    }

    #[test]
    fn an_unparsable_sweeps_line_stays_unknown_without_losing_the_sample() {
        let broken = PARSEABLE.replace("49 of 50", "many of lots");
        let Some(sample) = latest_sample(&broken) else {
            return;
        };
        assert_eq!(sample.get("sweeps_owed"), Some(&Value::Null));
        assert_eq!(sample.get("sweeps_total"), Some(&Value::Null));
        assert_eq!(
            sample.get("source_objects_owed").and_then(Value::as_u64),
            Some(56227)
        );
    }

    #[test]
    fn a_missing_sweeps_line_leaves_both_bounds_null() {
        let sweeps_removed = PARSEABLE
            .lines()
            .filter(|line| !line.contains("sweeps owed"))
            .collect::<Vec<&str>>()
            .join("\n");
        let Some(sample) = latest_sample(&sweeps_removed) else {
            return;
        };
        assert_eq!(sample.get("sweeps_owed"), Some(&Value::Null));
        assert_eq!(sample.get("sweeps_total"), Some(&Value::Null));
    }
}
