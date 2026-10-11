use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn write_or_print(out: Option<&Path>, content: &str) -> Result<()> {
    match out {
        Some(path) => {
            std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))
        }
        None => {
            print!("{}", content);
            Ok(())
        }
    }
}

pub(super) fn jurisdiction_map(
    jurisdictions: &[String],
    families: &[Value],
) -> IndexMap<String, Value> {
    let mut map = IndexMap::new();
    for jurisdiction in jurisdictions {
        let applicable = slugs_where(families, jurisdiction, |_| true);
        let armed = slugs_where(families, jurisdiction, |f| {
            f["strategy"] != "engineering_gap"
        });
        let gap = slugs_where(families, jurisdiction, |f| {
            f["strategy"] == "engineering_gap"
        });
        map.insert(
            jurisdiction.clone(),
            json!({
                "applicable_families": applicable,
                "armed_families": armed,
                "gap_families": gap
            }),
        );
    }
    map
}

fn slugs_where(
    families: &[Value],
    jurisdiction: &str,
    predicate: impl Fn(&Value) -> bool,
) -> Vec<Value> {
    families
        .iter()
        .filter(|family| {
            predicate(family)
                && family["applicable"]
                    .as_array()
                    .is_some_and(|names| names.contains(&json!(jurisdiction)))
        })
        .map(|family| family["slug"].clone())
        .collect()
}
