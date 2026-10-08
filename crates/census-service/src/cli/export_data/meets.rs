use crate::cli::export_data::csv::write_csv;
use census_domain::model::{CanonicalMeet, CompetitionLevel};
use census_report::export::athletic_net_meet_identity;
use std::collections::BTreeSet;

fn meet_identity_fields(m: &CanonicalMeet) -> (String, String, String, usize) {
    let identity = athletic_net_meet_identity(m);
    let an_meet = identity
        .map(|identity| identity.id.clone())
        .map_or(Default::default(), core::convert::identity);
    let an_url = identity
        .and_then(|identity| identity.url.clone())
        .map_or(Default::default(), core::convert::identity);
    let sports = m
        .sports
        .iter()
        .map(|s| s.stable_key())
        .collect::<Vec<_>>()
        .join(";");
    let ident_count = m.source_identities.len();
    (an_meet, an_url, sports, ident_count)
}

fn build_meet_row(m: &CanonicalMeet) -> Vec<String> {
    let (an_meet, an_url, sports, ident_count) = meet_identity_fields(m);
    let evidence_src = m
        .evidence
        .iter()
        .map(|e| e.source.id.as_str())
        .collect::<BTreeSet<_>>()
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join(";");

    vec![
        m.id.as_str().to_string(),
        m.state
            .map(|j| j.code().to_owned())
            .map_or(Default::default(), core::convert::identity),
        m.date.clone(),
        m.end_date
            .clone()
            .map_or(Default::default(), core::convert::identity),
        m.name.clone(),
        m.location
            .clone()
            .map_or(Default::default(), core::convert::identity),
        level_label(m.level).to_owned(),
        sports,
        an_meet,
        an_url,
        ident_count.to_string(),
        evidence_src,
    ]
}

fn level_label(level: CompetitionLevel) -> &'static str {
    match level {
        CompetitionLevel::Invitational => "invitational",
        CompetitionLevel::Dual => "dual",
        CompetitionLevel::Conference => "conference",
        CompetitionLevel::District => "district",
        CompetitionLevel::Regional => "regional",
        CompetitionLevel::Sectional => "sectional",
        CompetitionLevel::State => "state",
        CompetitionLevel::National => "national",
        CompetitionLevel::Unknown => "unknown",
    }
}

pub fn write_canonical_meets(
    meets: &[CanonicalMeet],
    data: &std::path::Path,
) -> anyhow::Result<()> {
    let rows: Vec<Vec<String>> = meets.iter().map(build_meet_row).collect();

    write_csv(
        &data.join("canonical-meets.csv"),
        [
            "meet_id",
            "state",
            "date",
            "end_date",
            "name",
            "location",
            "level",
            "sports",
            "athleticnet_meet_id",
            "athleticnet_url",
            "source_identities",
            "evidence_sources",
        ],
        &rows,
    )?;

    Ok(())
}
