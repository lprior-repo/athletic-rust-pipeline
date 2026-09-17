use crate::{
    domain::name::{CanonicalName, HtmlIdentity, NameExclusion},
    model::SourceRecord,
    runtime::acquisition::ProfileProbe,
};
use anyhow::{bail, Context, Result};

pub(super) fn verify(
    source: &SourceRecord,
    context: &CanonicalName,
    probe: &ProfileProbe,
) -> Result<NameExclusion> {
    if CanonicalName::from_source(source)?.as_ref() != Some(context) {
        bail!("name exclusion belongs to another original source-name context");
    }
    if !probe.failures.is_empty() || probe.identities.len() != 2 || probe.profiles.len() != 2 {
        bail!("name exclusion lacks successful full initial Bio parses");
    }
    let html = probe
        .html
        .as_ref()
        .context("name exclusion has no HTML evidence")?;
    let witness = NameExclusion::new(
        context.clone(),
        &probe.identities,
        HtmlIdentity {
            athlete_id: html.athlete_id,
            profile_url: &html.profile_url,
            names: &html.identity_hints,
            issues: &html.issues,
            document: &html.document,
        },
    )?;
    if witness.athlete_id() != probe.athlete_id {
        bail!("name exclusion witness belongs to another athlete");
    }
    Ok(witness)
}
