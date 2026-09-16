use super::{
    evidence, issue, AthleteId, AthleteName, EvidenceDigest, EvidenceIssue, Observed, TreeHint,
};
use anyhow::{bail, Context, Result};
use serde_json::Value;

const MAX_TREE_ITEMS: usize = 4096;
type Hints = (Vec<TreeHint>, Vec<Observed<AthleteName>>);

pub(super) fn parse(
    scripts: &[String],
    requested: AthleteId,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Hints {
    scripts.iter().enumerate().fold(
        (Vec::new(), Vec::new()),
        |mut output, (script_index, script)| {
            match decode(script) {
                Ok(Some(value)) => {
                    if let Err(error) =
                        collect(&value, requested, digest, script_index, &mut output)
                    {
                        issues.push(issue(
                            "embedded_state_shape",
                            &error.to_string(),
                            digest,
                            format!("html/script/{script_index}"),
                        ));
                    }
                }
                Ok(None) => {}
                Err(error) => issues.push(issue(
                    "embedded_state_malformed",
                    &error.to_string(),
                    digest,
                    format!("html/script/{script_index}"),
                )),
            }
            output
        },
    )
}

fn decode(script: &str) -> Result<Option<Value>> {
    let Some(marker) = script.find("anetSiteAppParams") else {
        return Ok(None);
    };
    let rest = script
        .get(marker..)
        .context("invalid embedded-state boundary")?;
    let start = rest
        .find('{')
        .context("embedded profile state has no JSON object")?;
    let json = rest
        .get(start..)
        .context("invalid embedded JSON boundary")?;
    let value = serde_json::Deserializer::from_str(json)
        .into_iter::<Value>()
        .next()
        .context("embedded profile JSON is empty")??;
    Ok(Some(value))
}

fn collect(
    value: &Value,
    requested: AthleteId,
    digest: &EvidenceDigest,
    script_index: usize,
    output: &mut Hints,
) -> Result<()> {
    let tree = value
        .get("tree")
        .and_then(Value::as_array)
        .context("profile state tree is not an array")?;
    if tree.len() > MAX_TREE_ITEMS
        || output
            .0
            .len()
            .checked_add(tree.len())
            .is_none_or(|count| count > MAX_TREE_ITEMS)
    {
        bail!("profile tree exceeds item bound");
    }
    tree.iter().enumerate().try_for_each(|(index, item)| {
        let object = item
            .as_object()
            .context("profile tree node is not an object")?;
        let kind = text(object.get("type")).context("profile tree node has no type")?;
        let id = object.get("id").and_then(identifier);
        let label = text(object.get("title"));
        let reference = evidence(
            digest,
            format!("html/script/{script_index}/anetSiteAppParams/tree/{index}"),
        );
        if kind.eq_ignore_ascii_case("athlete") {
            if id != Some(requested.get()) {
                bail!("embedded athlete identity differs from request");
            }
            let title = label.context("embedded athlete has no title")?;
            output.1.push(Observed {
                value: AthleteName::parse(title)?,
                evidence: reference.clone(),
            });
        }
        output.0.push(TreeHint {
            kind: kind.to_owned(),
            id,
            label: label.map(str::to_owned),
            evidence: reference,
        });
        Ok(())
    })
}

fn identifier(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        .filter(|value| *value > 0)
}

fn text(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
