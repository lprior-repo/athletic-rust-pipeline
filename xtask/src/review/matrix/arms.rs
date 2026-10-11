use super::{CENSUS_SOURCE_FILE, MEETS_FILE, RESULTS_FILE, TEAMS_FILE};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use regex::Regex;
use std::path::Path;

pub(super) struct Arms {
    pub(super) teams: Vec<(String, String)>,
    pub(super) meets: Vec<(String, String)>,
    pub(super) results: Vec<(String, String)>,
}

impl Arms {
    pub(super) fn parse(root: &Path, source_constant: &str) -> Result<Self> {
        Ok(Self {
            teams: parse_arm_table(root, TEAMS_FILE, source_constant.to_string())?,
            meets: parse_arm_table(root, MEETS_FILE, source_constant.to_string())?,
            results: parse_arm_table(root, RESULTS_FILE, source_constant.to_string())?,
        })
    }

    pub(super) fn slugs(&self) -> Vec<String> {
        self.teams
            .iter()
            .chain(self.meets.iter())
            .chain(self.results.iter())
            .map(|(slug, _)| slug.clone())
            .collect()
    }
}

pub(super) fn read_source_constant(root: &Path) -> Result<String> {
    let path = root.join(CENSUS_SOURCE_FILE);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", CENSUS_SOURCE_FILE))?;

    let re = Regex::new(r#"pub const SOURCE:\s*&str\s*=\s*"([^"]+)"#)
        .with_context(|| "compiling SOURCE regex")?;
    let cap = re
        .captures(&content)
        .ok_or_else(|| anyhow::anyhow!("{}: could not find SOURCE constant", CENSUS_SOURCE_FILE))?;
    let value = cap
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("{}: could not find SOURCE constant", CENSUS_SOURCE_FILE))?;
    Ok(value.as_str().to_string())
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
    let body = content
        .get(start.saturating_add(4)..)
        .with_context(|| format!("{}: invalid arm table start", file))?;
    let Some(end) = body.find("\n];") else {
        anyhow::bail!("{}: arm table not closed", file);
    };
    let block = body
        .get(..end)
        .with_context(|| format!("{}: invalid arm table end", file))?;

    let re = Regex::new(
        r#"(?m)^\s*\(\s*("(?:[^"]+)"|[A-Za-z_][A-Za-z0-9_:]*)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)::([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*,"#,
    )
    .with_context(|| format!("compiling arm regex for {}", file))?;

    let mut constants = read_string_constants(&content)?;
    constants.insert("SOURCE".to_string(), source_constant);
    let census_path = root.join(CENSUS_SOURCE_FILE);
    let census_content = std::fs::read_to_string(&census_path)
        .with_context(|| format!("reading {}", CENSUS_SOURCE_FILE))?;
    for (name, value) in read_string_constants(&census_content)? {
        constants.insert(name, value);
    }

    let mut arms = Vec::new();
    for cap in re.captures_iter(block) {
        let Some(slug_expr) = cap.get(1) else {
            anyhow::bail!("{}: arm entry carries no slug", file);
        };
        let Some(variant) = cap.get(3) else {
            anyhow::bail!("{}: arm entry carries no variant", file);
        };
        let slug = resolve_slug(slug_expr.as_str(), &constants, file)?;
        arms.push((slug, variant.as_str().to_string()));
    }

    if arms.is_empty() {
        anyhow::bail!("{}: arm table parsed 0 entries", file);
    }

    Ok(arms)
}

fn read_string_constants(content: &str) -> Result<IndexMap<String, String>> {
    let Ok(re) = Regex::new(
        r#"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+([A-Z][A-Z0-9_]*)\s*:\s*&str\s*=\s*"([^"]*)""#,
    ) else {
        return Ok(IndexMap::new());
    };
    re.captures_iter(content)
        .map(|cap| {
            let name = cap
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("constant entry carries no name"))?;
            let value = cap
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("constant entry carries no value"))?;
            Ok((name.as_str().to_string(), value.as_str().to_string()))
        })
        .collect::<Result<IndexMap<String, String>>>()
}

pub(super) fn resolve_slug(
    expr: &str,
    constants: &IndexMap<String, String>,
    file: &str,
) -> Result<String> {
    if let Some(slug) = expr
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
    {
        return Ok(slug.to_string());
    }
    let name = expr.rsplit_once("::").map_or(expr, |(_, tail)| tail);
    constants
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{}: unresolved constant reference `{}`", file, expr))
}
