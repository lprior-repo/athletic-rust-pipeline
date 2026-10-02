use crate::cli::export_data::csv::write_csv;
use census_domain::model::{CanonicalSchool, SourceNamespace};
use std::collections::BTreeSet;

fn identity_fields(s: &CanonicalSchool) -> (String, String, usize) {
    let namespaces: BTreeSet<&SourceNamespace> =
        s.source_identities.iter().map(|i| &i.namespace).collect();
    let source_ns = namespaces
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(";");
    let evidence_src = s
        .evidence
        .iter()
        .map(|e| e.source.id.as_str())
        .collect::<BTreeSet<_>>()
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join(";");
    let ident_count = s.source_identities.len();
    (source_ns, evidence_src, ident_count)
}

fn build_school_row(s: &CanonicalSchool) -> anyhow::Result<Vec<String>> {
    let an_team = s
        .source_identities
        .iter()
        .find(|i| matches!(&i.namespace, SourceNamespace::LegacyAthleticNet { kind } | SourceNamespace::AthleticNet { kind } if kind == "team"))
        .map(|i| i.id.clone())
        .unwrap_or_default();
    let ms_school = s
        .source_identities
        .iter()
        .find(|i| matches!(i.namespace, SourceNamespace::MilesplitSchool))
        .map(|i| i.id.clone())
        .unwrap_or_default();
    let aliases = s.aliases.join(";");
    let (source_ns, evidence_src, ident_count) = identity_fields(s);

    let mut row = vec![
        s.id.as_str().to_string(),
        s.name.clone(),
        s.state.map(|j| j.code().to_owned()).unwrap_or_default(),
        s.city.clone().unwrap_or_default(),
        s.association.clone().unwrap_or_default(),
        s.classification.clone().unwrap_or_default(),
        s.enrollment.map(|e| e.to_string()).unwrap_or_default(),
        if s.co_op {
            "true".to_string()
        } else {
            "false".to_string()
        },
        s.athletics_website.clone().unwrap_or_default(),
        s.school_website.clone().unwrap_or_default(),
        an_team,
        ms_school,
        aliases,
        source_ns,
        evidence_src,
        ident_count.to_string(),
    ];
    row.extend(census_report::export::postal::postal_fields([s])?);
    Ok(row)
}

pub fn write_canonical_schools(
    schools: &[CanonicalSchool],
    data: &std::path::Path,
) -> anyhow::Result<()> {
    let mut sorted: Vec<&CanonicalSchool> = schools.iter().collect();
    sorted.sort_by(|a, b| {
        a.state
            .map(|j| j.code())
            .unwrap_or_default()
            .cmp(b.state.map(|j| j.code()).unwrap_or_default())
            .then_with(|| a.name.cmp(&b.name))
    });
    let rows = sorted
        .into_iter()
        .map(build_school_row)
        .collect::<anyhow::Result<Vec<_>>>()?;

    write_csv(
        &data.join("canonical-schools.csv"),
        &[
            "school_id",
            "name",
            "state",
            "city",
            "association",
            "classification",
            "enrollment",
            "co_op",
            "athletics_website",
            "school_website",
            "athleticnet_team_id",
            "milesplit_school_id",
            "aliases",
            "source_namespaces",
            "evidence_sources",
            "identity_count",
            "postal_school_id",
            "postal_street",
            "postal_second_line",
            "postal_city",
            "postal_state",
            "postal_zip",
            "postal_owner_namespace",
            "postal_owner_id",
            "postal_source",
            "postal_source_url",
            "postal_observed_date",
            "postal_capture_sha256",
        ],
        &rows,
    )?;

    Ok(())
}
