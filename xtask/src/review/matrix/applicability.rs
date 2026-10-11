use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;

use super::{APPLICABILITY_FILE, APPLICABILITY_TABLE_FILE, JURISDICTION_FILE};

pub(super) fn read_applicability(root: &Path, census_scope: &[String]) -> Result<Vec<Value>> {
    let path = root.join(APPLICABILITY_FILE);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", APPLICABILITY_FILE))?;

    let declared = parse_declared_length(&content, "TABLE", "Applicability")?;
    let parsed = parse_applicability_entries(&content, census_scope)?;
    let references = count_referenced_elements(&content)?;

    let total = parsed
        .len()
        .checked_add(references)
        .context("applicability count overflow")?;
    if total != declared {
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
    let tail = content
        .get(last_block_end..)
        .context("invalid applicability tail offset")?;
    let Some(close) = tail.find("];") else {
        anyhow::bail!("{}: applicability array is not closed", APPLICABILITY_FILE);
    };
    let re = Regex::new(r"^\s*[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_:]*,\s*$")
        .with_context(|| "compiling referenced-element regex")?;
    Ok(tail
        .get(..close)
        .context("invalid applicability closing offset")?
        .lines()
        .filter(|line| re.is_match(line))
        .count())
}

pub(super) fn read_arbiter(root: &Path, census_scope: &[String]) -> Result<Value> {
    let path = root.join(APPLICABILITY_TABLE_FILE);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", APPLICABILITY_TABLE_FILE))?;

    let Some(start) = content.find("const ARBITER:") else {
        anyhow::bail!(
            "{}: ARBITER declaration not found",
            APPLICABILITY_TABLE_FILE
        );
    };
    let rest = content
        .get(start..)
        .context("invalid ARBITER start offset")?;
    let Some(end) = rest.find("\n};") else {
        anyhow::bail!(
            "{}: ARBITER declaration not closed",
            APPLICABILITY_TABLE_FILE
        );
    };
    let block = rest.get(..end).context("invalid ARBITER end offset")?;

    let slug = extract_string(block, "slug:")?;
    let jurisdictions = extract_jurisdictions(block, census_scope)?;

    Ok(json!({
        "slug": slug,
        "applicable": jurisdictions
    }))
}

pub(super) fn read_jurisdictions(root: &Path) -> Result<Vec<String>> {
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

pub(super) fn parse_declared_length(
    content: &str,
    declaration: &str,
    element: &str,
) -> Result<usize> {
    let pattern = format!(r"{}\s*:\s*\[{}\s*;\s*(\d+)\]", declaration, element);
    let re = Regex::new(&pattern)
        .with_context(|| format!("compiling length regex for {}", declaration))?;
    let cap = re
        .captures(content)
        .ok_or_else(|| anyhow::anyhow!("{}: could not find declared array length", declaration))?;
    let captured = cap
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("{}: declared array length is absent", declaration))?;
    captured
        .as_str()
        .parse()
        .with_context(|| format!("{}: parsing declared array length", declaration))
}

pub(super) fn parse_applicability_entries(
    content: &str,
    census_scope: &[String],
) -> Result<Vec<Value>> {
    let mut entries = Vec::new();
    let pattern = r#"slug:\s*"([^"]+)"#;
    let re = Regex::new(pattern).with_context(|| "compiling slug regex")?;

    let mut blocks = content.split("Applicability {");
    blocks.next();

    for block in blocks {
        let Some(end) = block_end_offset(block) else {
            break;
        };
        let block = block
            .get(..end)
            .context("invalid applicability block end")?;

        let Some(cap) = re.captures(block) else {
            anyhow::bail!("applicability block missing slug field");
        };
        let Some(slug) = cap.get(1) else {
            anyhow::bail!("applicability block missing slug field");
        };
        let slug = slug.as_str().to_string();
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
        match bytes.get(index.saturating_add(1)) {
            Some(b',') | Some(b'\n') | None => Some(index),
            _ => None,
        }
    })
}

fn parse_census_scope(content: &str) -> Result<Vec<String>> {
    let start = content
        .find("pub const CENSUS_SCOPE:")
        .ok_or_else(|| anyhow::anyhow!("CENSUS_SCOPE array not found"))?;
    let rest = content
        .get(start..)
        .context("invalid CENSUS_SCOPE start offset")?;
    let Some(equals) = rest.find("= [") else {
        anyhow::bail!("CENSUS_SCOPE missing opening bracket");
    };
    let body = rest
        .get(equals.saturating_add(3)..)
        .context("invalid CENSUS_SCOPE body offset")?;
    let Some(end) = bracket_end(body) else {
        anyhow::bail!("CENSUS_SCOPE missing closing bracket");
    };
    let block = body.get(..end).context("invalid CENSUS_SCOPE end offset")?;

    let re = Regex::new(r"Self::([A-Za-z][A-Za-z0-9_]*)")
        .with_context(|| "compiling census scope regex")?;
    let names: Vec<String> = re
        .captures_iter(block)
        .map(|cap| {
            cap.get(1)
                .map(|name| name.as_str().to_string())
                .ok_or_else(|| anyhow::anyhow!("CENSUS_SCOPE entry has no name"))
        })
        .collect::<Result<Vec<String>>>()?;

    if names.is_empty() {
        anyhow::bail!("CENSUS_SCOPE parsed 0 entries");
    }

    Ok(names)
}

fn bracket_end(body: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (index, ch) in body.char_indices() {
        match ch {
            '[' => depth = depth.checked_add(1)?,
            ']' => {
                depth = depth.checked_sub(1)?;
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
        let Some(name) = cap.get(1) else {
            anyhow::bail!("jurisdiction entry carries no name");
        };
        let name = name.as_str();
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
    let value = cap
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("{}: could not extract string", prefix))?;
    Ok(value.as_str().to_string())
}
