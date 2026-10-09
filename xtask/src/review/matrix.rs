#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use indexmap::IndexMap;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;

use crate::paths::repo_root;

const APPLICABILITY_FILE: &str = "crates/census-crawl/src/applicability/table/data.rs";
const APPLICABILITY_TABLE_FILE: &str = "crates/census-crawl/src/applicability/table.rs";
const JURISDICTION_FILE: &str = "crates/census-domain/src/jurisdiction/table.rs";
const TEAMS_FILE: &str = "crates/census-service/src/restate_services/teams_arms.rs";
const MEETS_FILE: &str = "crates/census-service/src/restate_services/meets_arms.rs";
const RESULTS_FILE: &str = "crates/census-service/src/restate_services/results_arms.rs";
const STRATEGIES_FILE: &str = "crates/census-service/src/restate_services/plan/strategies.rs";
const CENSUS_SOURCE_FILE: &str = "crates/census-service/src/census/meets.rs";

struct Arms {
    teams: Vec<(String, String)>,
    meets: Vec<(String, String)>,
    results: Vec<(String, String)>,
}

impl Arms {
    fn parse(root: &Path, source_constant: &str) -> Result<Self> {
        Ok(Self {
            teams: parse_arm_table(root, TEAMS_FILE, source_constant.to_string())?,
            meets: parse_arm_table(root, MEETS_FILE, source_constant.to_string())?,
            results: parse_arm_table(root, RESULTS_FILE, source_constant.to_string())?,
        })
    }

    fn slugs(&self) -> Vec<String> {
        self.teams
            .iter()
            .chain(self.meets.iter())
            .chain(self.results.iter())
            .map(|(slug, _)| slug.clone())
            .collect()
    }
}

pub(super) fn run(out: Option<&Path>) -> Result<()> {
    let root = repo_root();
    let jurisdictions = read_jurisdictions(&root)?;
    let applicability = read_applicability(&root, &jurisdictions)?;
    let arbiter = read_arbiter(&root, &jurisdictions)?;
    let source_constant = read_source_constant(&root)?;
    let arms = Arms::parse(&root, &source_constant)?;
    let strategies = parse_strategies(&root)?;

    let families = build_families(&applicability, &arbiter, &arms, &strategies);

    let registered = families.len();
    let armed = families
        .iter()
        .filter(|f| f["strategy"] != "engineering_gap")
        .count();
    let gap = families
        .iter()
        .filter(|f| f["strategy"] == "engineering_gap")
        .count();

    let jurisdiction_map = jurisdiction_map(&jurisdictions, &families);

    let registered_slugs: Vec<String> = families
        .iter()
        .filter_map(|f| f["slug"].as_str().map(|s| s.to_string()))
        .collect();
    let arms_not_registered: Vec<String> = arms
        .slugs()
        .into_iter()
        .filter(|s| !registered_slugs.contains(s))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let files = vec![
        APPLICABILITY_FILE,
        APPLICABILITY_TABLE_FILE,
        JURISDICTION_FILE,
        TEAMS_FILE,
        MEETS_FILE,
        RESULTS_FILE,
        STRATEGIES_FILE,
        CENSUS_SOURCE_FILE,
    ];

    let output = json!({
        "schema": "census-source-jurisdiction-matrix/v1",
        "extracted_from": files,
        "registered_families": registered,
        "armed_families": armed,
        "gap_families": gap,
        "jurisdiction_count": jurisdictions.len(),
        "families": families,
        "jurisdictions": jurisdiction_map,
        "arms_not_registered": arms_not_registered
    });

    let rendered =
        serde_json::to_string_pretty(&output).with_context(|| "serializing matrix JSON")? + "\n";

    write_or_print(out, &rendered)?;

    Ok(())
}

fn write_or_print(out: Option<&Path>, content: &str) -> Result<()> {
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

fn jurisdiction_map(jurisdictions: &[String], families: &[Value]) -> IndexMap<String, Value> {
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

fn read_applicability(root: &Path, census_scope: &[String]) -> Result<Vec<Value>> {
    let path = root.join(APPLICABILITY_FILE);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", APPLICABILITY_FILE))?;

    let declared = parse_declared_length(&content, "TABLE", "Applicability")?;
    let parsed = parse_applicability_entries(&content, census_scope)?;
    let references = count_referenced_elements(&content)?;

    if parsed.len() + references != declared {
        anyhow::bail!(
            "{}: declared [Applicability; {}] but parsed {} entries and {} referenced elements",
            APPLICABILITY_FILE,
            declared,
            parsed.len(),
            references
        );
    }

    Ok(parsed)
}

fn count_referenced_elements(content: &str) -> Result<usize> {
    let Some(last_block_end) = content.rfind("},") else {
        return Ok(0);
    };
    let tail = &content[last_block_end..];
    let Some(close) = tail.find("];") else {
        anyhow::bail!("{}: applicability array is not closed", APPLICABILITY_FILE);
    };
    let re = Regex::new(r"^\s*[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_:]*,\s*$")
        .with_context(|| "compiling referenced-element regex")?;
    Ok(tail[..close]
        .lines()
        .filter(|line| re.is_match(line))
        .count())
}

fn read_arbiter(root: &Path, census_scope: &[String]) -> Result<Value> {
    let path = root.join(APPLICABILITY_TABLE_FILE);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", APPLICABILITY_TABLE_FILE))?;

    let Some(start) = content.find("const ARBITER:") else {
        anyhow::bail!(
            "{}: ARBITER declaration not found",
            APPLICABILITY_TABLE_FILE
        );
    };
    let rest = &content[start..];
    let Some(end) = rest.find("\n};") else {
        anyhow::bail!(
            "{}: ARBITER declaration not closed",
            APPLICABILITY_TABLE_FILE
        );
    };
    let block = &rest[..end];

    let slug = extract_string(block, "slug:")?;
    let jurisdictions = extract_jurisdictions(block, census_scope)?;

    Ok(json!({
        "slug": slug,
        "applicable": jurisdictions
    }))
}

fn read_jurisdictions(root: &Path) -> Result<Vec<String>> {
    let path = root.join(JURISDICTION_FILE);
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", JURISDICTION_FILE))?;

    let declared = parse_declared_length(&content, "CENSUS_SCOPE", "Self")?;
    let parsed = parse_census_scope(&content)?;

    if parsed.len() != declared {
        anyhow::bail!(
            "{}: declared [Self; {}] but parsed {} entries in CENSUS_SCOPE",
            JURISDICTION_FILE,
            declared,
            parsed.len()
        );
    }

    Ok(parsed)
}

fn parse_declared_length(content: &str, declaration: &str, element: &str) -> Result<usize> {
    let pattern = format!(r"{}\s*:\s*\[{}\s*;\s*(\d+)\]", declaration, element);
    let re = Regex::new(&pattern)
        .with_context(|| format!("compiling length regex for {}", declaration))?;
    let cap = re
        .captures(content)
        .ok_or_else(|| anyhow::anyhow!("{}: could not find declared array length", declaration))?;
    cap[1]
        .parse()
        .with_context(|| format!("{}: parsing declared array length", declaration))
}

fn parse_applicability_entries(content: &str, census_scope: &[String]) -> Result<Vec<Value>> {
    let mut entries = Vec::new();
    let pattern = r#"slug:\s*"([^"]+)"#;
    let re = Regex::new(pattern).with_context(|| "compiling slug regex")?;

    let mut blocks = content.split("Applicability {");
    blocks.next();

    for block in blocks {
        let Some(end) = block_end_offset(block) else {
            break;
        };
        let block = &block[..end];

        let Some(cap) = re.captures(block) else {
            anyhow::bail!("applicability block missing slug field");
        };
        let slug = cap[1].to_string();
        let jurisdictions = extract_jurisdictions(block, census_scope)?;

        entries.push(json!({
            "slug": slug,
            "applicable": jurisdictions
        }));
    }

    Ok(entries)
}
fn block_end_offset(block: &str) -> Option<usize> {
    let bytes = block.as_bytes();
    bytes.iter().enumerate().find_map(|(index, byte)| {
        if *byte != b'}' {
            return None;
        }
        match bytes.get(index + 1) {
            Some(b',') | Some(b'\n') | None => Some(index),
            _ => None,
        }
    })
}

fn parse_census_scope(content: &str) -> Result<Vec<String>> {
    let start = content
        .find("pub const CENSUS_SCOPE:")
        .ok_or_else(|| anyhow::anyhow!("CENSUS_SCOPE array not found"))?;
    let rest = &content[start..];
    let Some(equals) = rest.find("= [") else {
        anyhow::bail!("CENSUS_SCOPE missing opening bracket");
    };
    let body = &rest[equals + 3..];
    let Some(end) = bracket_end(body) else {
        anyhow::bail!("CENSUS_SCOPE missing closing bracket");
    };
    let block = &body[..end];

    let re = Regex::new(r"Self::([A-Za-z][A-Za-z0-9_]*)")
        .with_context(|| "compiling census scope regex")?;
    let names: Vec<String> = re
        .captures_iter(block)
        .map(|cap| cap[1].to_string())
        .collect();

    if names.is_empty() {
        anyhow::bail!("CENSUS_SCOPE parsed 0 entries");
    }

    Ok(names)
}

fn bracket_end(body: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (index, ch) in body.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_jurisdictions(block: &str, census_scope: &[String]) -> Result<Vec<String>> {
    let re = Regex::new(r"UsJurisdiction::([A-Za-z_][A-Za-z0-9_]*)")
        .with_context(|| "compiling jurisdiction regex")?;
    let mut names = Vec::new();

    for cap in re.captures_iter(block) {
        let name = &cap[1];
        if name == "CENSUS_SCOPE" {
            names.extend(census_scope.iter().cloned());
        } else if name.contains('_') {
            anyhow::bail!(
                "unsupported jurisdiction constant `UsJurisdiction::{}`",
                name
            );
        } else {
            names.push(name.to_string());
        }
    }

    Ok(names)
}

fn extract_string(content: &str, prefix: &str) -> Result<String> {
    let re = Regex::new(&format!("{}\\s*\"([^\"]+)\"", prefix))
        .with_context(|| format!("compiling {} regex", prefix))?;
    let cap = re
        .captures(content)
        .ok_or_else(|| anyhow::anyhow!("{}: could not extract string", prefix))?;
    Ok(cap[1].to_string())
}

fn read_source_constant(root: &Path) -> Result<String> {
    let path = root.join(CENSUS_SOURCE_FILE);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", CENSUS_SOURCE_FILE))?;

    let re = Regex::new(r#"pub const SOURCE:\s*&str\s*=\s*"([^"]+)"#)
        .with_context(|| "compiling SOURCE regex")?;
    let cap = re
        .captures(&content)
        .ok_or_else(|| anyhow::anyhow!("{}: could not find SOURCE constant", CENSUS_SOURCE_FILE))?;
    Ok(cap[1].to_string())
}

fn parse_arm_table(
    root: &Path,
    file: &str,
    source_constant: String,
) -> Result<Vec<(String, String)>> {
    let path = root.join(file);
    let content = std::fs::read_to_string(&path).with_context(|| format!("reading {}", file))?;

    let Some(start) = content.find("= &[") else {
        anyhow::bail!("{}: arm table not found", file);
    };
    let body = &content[start + 4..];
    let Some(end) = body.find("\n];") else {
        anyhow::bail!("{}: arm table not closed", file);
    };
    let block = &body[..end];

    let re = Regex::new(
        r#"(?m)^\s*\(\s*("(?:[^"]+)"|[A-Za-z_][A-Za-z0-9_:]*)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)::([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*,"#,
    )
    .with_context(|| format!("compiling arm regex for {}", file))?;

    let mut constants = read_string_constants(&content);
    constants.insert("SOURCE".to_string(), source_constant);
    let census_path = root.join(CENSUS_SOURCE_FILE);
    let census_content = std::fs::read_to_string(&census_path)
        .with_context(|| format!("reading {}", CENSUS_SOURCE_FILE))?;
    for (name, value) in read_string_constants(&census_content) {
        constants.insert(name, value);
    }

    let mut arms = Vec::new();
    for cap in re.captures_iter(block) {
        let slug = resolve_slug(&cap[1], &constants, file)?;
        arms.push((slug, cap[3].to_string()));
    }

    if arms.is_empty() {
        anyhow::bail!("{}: arm table parsed 0 entries", file);
    }

    Ok(arms)
}

fn read_string_constants(content: &str) -> IndexMap<String, String> {
    let Ok(re) = Regex::new(
        r#"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+([A-Z][A-Z0-9_]*)\s*:\s*&str\s*=\s*"([^"]*)""#,
    ) else {
        return IndexMap::new();
    };
    re.captures_iter(content)
        .map(|cap| (cap[1].to_string(), cap[2].to_string()))
        .collect()
}

fn resolve_slug(expr: &str, constants: &IndexMap<String, String>, file: &str) -> Result<String> {
    if expr.len() >= 2 && expr.starts_with('"') && expr.ends_with('"') {
        return Ok(expr[1..expr.len() - 1].to_string());
    }
    let name = expr.rsplit("::").next().unwrap_or(expr);
    constants
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{}: unresolved constant reference `{}`", file, expr))
}

fn parse_strategies(root: &Path) -> Result<IndexMap<String, String>> {
    let path = root.join(STRATEGIES_FILE);
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", STRATEGIES_FILE))?;

    let re =
        Regex::new(r#"([a-z_]+)\s*=>\s*"([^"]+)"#).with_context(|| "compiling strategy regex")?;

    let mut strategies = IndexMap::new();
    for cap in re.captures_iter(&content) {
        let slug = cap[1].to_string();
        let reason = cap[2].to_string();
        strategies.insert(slug, reason);
    }

    let wildcard =
        Regex::new(r#"_ =>\s*"([^"]+)"#).with_context(|| "compiling wildcard strategy regex")?;
    if let Some(cap) = wildcard.captures(&content) {
        strategies.insert("wildcard".to_string(), cap[1].to_string());
    }

    Ok(strategies)
}

fn build_families(
    applicability: &[Value],
    arbiter: &Value,
    arms: &Arms,
    strategies: &IndexMap<String, String>,
) -> Vec<Value> {
    let all_applicability: Vec<&Value> = applicability.iter().chain([arbiter]).collect();
    let mut families = Vec::new();

    for entry in all_applicability {
        let slug = entry["slug"]
            .as_str()
            .map_or_else(|| "unknown".to_string(), |s| s.to_string());
        let applicable = entry["applicable"]
            .as_array()
            .map_or_else(Vec::new, Clone::clone);
        let mut applicable_names: Vec<String> = applicable
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        applicable_names.sort();
        applicable_names.dedup();

        let strategy = determine_strategy(&slug, arms);
        let arm_names = determine_arms(&slug, arms);
        let gap_reason = if strategy == "engineering_gap" {
            strategies
                .get(&slug)
                .map_or_else(
                    || {
                        strategies.get("wildcard").map_or(
                            "registry family has no implemented durable execution strategy",
                            String::as_str,
                        )
                    },
                    String::as_str,
                )
                .to_string()
        } else {
            String::new()
        };

        let mut family = json!({
            "slug": slug,
            "applicable": applicable_names,
            "strategy": strategy,
            "arms": arm_names,
        });

        if strategy == "engineering_gap" {
            family["gap_reason"] = json!(gap_reason);
        }

        families.push(family);
    }

    let mut seen = IndexMap::new();
    for f in families {
        let slug = f["slug"]
            .as_str()
            .map_or_else(|| "unknown".to_string(), |s| s.to_string());
        seen.insert(slug, f);
    }

    let mut result: Vec<Value> = seen.into_values().collect();
    result.sort_by(|a, b| {
        let sa = a["slug"].as_str().map_or("", core::convert::identity);
        let sb = b["slug"].as_str().map_or("", core::convert::identity);
        sa.cmp(sb)
    });

    result
}

fn determine_strategy(slug: &str, arms: &Arms) -> String {
    if arms.teams.iter().any(|(s, _)| s == slug) {
        return "teams".to_string();
    }
    if arms.meets.iter().any(|(s, _)| s == slug) {
        return "meets".to_string();
    }
    if arms.results.iter().any(|(s, _)| s == slug) {
        return "results".to_string();
    }
    "engineering_gap".to_string()
}

fn determine_arms(slug: &str, arms: &Arms) -> Vec<String> {
    let tables = [
        ("teams", &arms.teams),
        ("meets", &arms.meets),
        ("results", &arms.results),
    ];
    tables
        .iter()
        .filter(|(_, table)| table.iter().any(|(s, _)| s == slug))
        .map(|(name, _)| (*name).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_extraction_from_sample() {
        let content = r#"Applicability {
    slug: "aia",
    jurisdictions: &[UsJurisdiction::Arizona],
}"#;
        let entries = parse_applicability_entries(content, &[])
            .map_or_else(|e| panic!("parse failed: {}", e), core::convert::identity);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["slug"], "aia");
        assert_eq!(entries[0]["applicable"][0], "Arizona");
    }

    #[test]
    fn census_scope_expansion_in_applicability() {
        let content = r#"Applicability {
    slug: "milesplit",
    jurisdictions: &UsJurisdiction::CENSUS_SCOPE,
}"#;
        let scope = vec!["Alabama".to_string(), "Wyoming".to_string()];
        let entries = parse_applicability_entries(content, &scope)
            .map_or_else(|e| panic!("parse failed: {}", e), core::convert::identity);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["applicable"][0], "Alabama");
        assert_eq!(entries[0]["applicable"][1], "Wyoming");
    }

    #[test]
    fn unsupported_jurisdiction_constant_rejected() {
        let content = r#"Applicability {
    slug: "mystery",
    jurisdictions: &UsJurisdiction::SOMEWHERE_ELSE,
}"#;
        let result = parse_applicability_entries(content, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn declared_length_mismatch_detected() {
        let content = "pub(crate) const TABLE: [Applicability; 100] = [\n";
        let declared = parse_declared_length(content, "TABLE", "Applicability")
            .map_or_else(|e| panic!("parse failed: {}", e), core::convert::identity);
        assert_eq!(declared, 100);
        let parsed = parse_applicability_entries(content, &[])
            .map_or_else(|e| panic!("parse failed: {}", e), core::convert::identity);
        assert_eq!(parsed.len(), 0);
        assert_ne!(parsed.len(), declared);
    }

    #[test]
    fn unresolved_constant_error() {
        let result = resolve_slug("UNKNOWN_CONSTANT", &IndexMap::new(), "test.rs");
        assert!(result.is_err());
        let Err(error) = result else {
            return;
        };
        assert!(error.to_string().contains("UNKNOWN_CONSTANT"));
    }

    #[test]
    fn strategy_precedence_mapping() {
        let arms = Arms {
            teams: vec![("milesplit".to_string(), "MilesplitIndex".to_string())],
            meets: vec![("wiaa_results".to_string(), "WiaaResults".to_string())],
            results: vec![("athleticnet".to_string(), "AthleticnetMeets".to_string())],
        };

        assert_eq!(determine_strategy("milesplit", &arms), "teams");
        assert_eq!(determine_strategy("wiaa_results", &arms), "meets");
        assert_eq!(determine_strategy("athleticnet", &arms), "results");
        assert_eq!(determine_strategy("foo", &arms), "engineering_gap");
    }

    #[test]
    fn ref_ordering_deterministic() {
        let content = r#"Applicability {
    slug: "z",
    jurisdictions: &[UsJurisdiction::Wyoming, UsJurisdiction::Alabama],
},
Applicability {
    slug: "a",
    jurisdictions: &[UsJurisdiction::Wyoming, UsJurisdiction::Alabama],
},"#;
        let entries = parse_applicability_entries(content, &[])
            .map_or_else(|e| panic!("parse failed: {}", e), core::convert::identity);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["slug"], "z");
        assert_eq!(entries[0]["applicable"][0], "Wyoming");
        assert_eq!(entries[0]["applicable"][1], "Alabama");
        assert_eq!(entries[1]["slug"], "a");
    }
}
