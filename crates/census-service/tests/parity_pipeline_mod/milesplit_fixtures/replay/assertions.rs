use std::collections::BTreeSet;

use anyhow::{Context, Result};
use census_crawl::milesplit;
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, CanonicalSchool, CentiMetres, EventKind,
    ExactSeconds, Gender, Mark, SourceNamespace, SourceObservation, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{ACQUIRED_AT, PHASE};

pub(super) fn assert_retention(store: &Store, body: &[u8], relay: bool) -> Result<()> {
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?, Vec::new());
    check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?,
    Vec::new());
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?, Vec::new());
    assert_observations(store, relay)?;
    assert_physical_observations(store, relay)?;
    let page = super::super::owned_page(body)?;
    check!(eq; page.completeness, milesplit::OwnedCompleteness::Unknown);
    check!(!page.ownership_complete());
    let payloads = store.journal_payloads(milesplit::OWNED_MEET_PHASE)?;
    let summary = payloads
        .iter()
        .find(|row| row.get("published_rows").is_some())
        .context("owned parse summary missing")?;
    check!(eq; summary["published_rows"], 3);
    check!(eq; summary["owned_rows"], if relay { 0 } else { 3 });
    check!(eq; summary["team_relay_rows"], if relay { 3 } else { 0 });
    check!(eq; summary["completeness"], "unknown");
    check!(eq; summary["ownership_complete"], false);
    check!(eq; summary["capture"]["fetched_at"], ACQUIRED_AT);
    check!(eq; summary["capture"]["content_digest"],
    format!("{:x}", Sha256::digest(body)));
    if !relay {
        assert_individuals(&page)?;
        assert_meet(store)?;
    }
    assert_projection_receipts(store, relay)
}

fn assert_observations(store: &Store, relay: bool) -> Result<()> {
    let observations: Vec<SourceObservation> = store.scan(Table::SourceObservations)?;
    check!(eq; observations.len(), if relay { 0 } else { 2 });
    let actual: BTreeSet<_> = observations
        .iter()
        .map(|row| match row {
            SourceObservation::Athlete(row) => (
                row.source_athlete_id.as_str(),
                row.observed_name.as_str(),
                row.observed_on.as_str(),
            ),
            SourceObservation::School(_) => ("unexpected school", "", ""),
        })
        .collect();
    let expected = if relay {
        BTreeSet::new()
    } else {
        BTreeSet::from([
            ("14222592", "Adelyn Spann", ACQUIRED_AT),
            ("11357806", "Payton Ousley", ACQUIRED_AT),
        ])
    };
    check!(eq; actual, expected);
    Ok(())
}

fn assert_physical_observations(store: &Store, relay: bool) -> Result<()> {
    let mut rows = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::SourceObservations, |row| {
            rows.push(row);
            Ok(())
        })?;
    check!(eq; rows.len(), if relay { 0 } else { 3 });
    let mut keys = BTreeSet::new();
    for row in rows {
        let SourceObservation::Athlete(row) = row else {
            anyhow::bail!("retained athlete capture fabricated a school observation");
        };
        check!(eq; row.namespace, SourceNamespace::MilesplitAthlete);
        check!(eq; row.observed_on, ACQUIRED_AT);
        check!(eq; row.gender, Gender::Girls);
        check!(eq; row.observed_grade, None);
        let (name, school, profile) = match row.source_athlete_id.as_str() {
            "14222592" => (
                "Adelyn Spann",
                None,
                "https://www.milesplit.com/athletes/14222592-adelyn-spann",
            ),
            "11357806" => (
                "Payton Ousley",
                Some("Charles Henderson"),
                "https://www.milesplit.com/athletes/11357806-payton-ousley",
            ),
            other => anyhow::bail!("unexpected native participant {other}"),
        };
        check!(eq; row.observed_name, name);
        check!(eq; row.observed_school.as_deref(), school);
        check!(eq; row.profile_url.as_deref(), Some(profile));
        check!(keys.insert((row.source_athlete_id, row.source_row_key)));
    }
    let expected = if relay {
        BTreeSet::new()
    } else {
        BTreeSet::from([
            (
                "14222592".to_string(),
                "milesplit_result:201782263".to_string(),
            ),
            (
                "14222592".to_string(),
                "milesplit_result:201782277".to_string(),
            ),
            (
                "11357806".to_string(),
                "milesplit_result:201782806".to_string(),
            ),
        ])
    };
    check!(eq; keys, expected);
    Ok(())
}

fn assert_individuals(page: &milesplit::OwnedMeetPage) -> Result<()> {
    check!(eq; page.rejected, Vec::new());
    let actual: Vec<_> = page
        .rows
        .iter()
        .map(|row| {
            (
                row.result_id,
                row.source_athlete.id.as_str(),
                row.grad_year.map(|year| year.get()),
                row.event_kind.clone(),
                row.mark.clone(),
            )
        })
        .collect();
    check!(eq; actual,
    vec![
        (
            201782263,
            "14222592",
            Some(2027),
            EventKind::LongJump,
            Mark::FieldImperial {
                feet_mark: "13-9".to_string(),
                metres: CentiMetres::new(419),
            }
        ),
        (
            201782277,
            "14222592",
            Some(2027),
            EventKind::TripleJump,
            Mark::FieldImperial {
                feet_mark: "30-7".to_string(),
                metres: CentiMetres::new(932),
            }
        ),
        (
            201782806,
            "11357806",
            Some(2026),
            EventKind::Track100m,
            Mark::TimeSeconds(ExactSeconds::parse("12.40")?)
        ),
    ]);
    Ok(())
}

fn assert_meet(store: &Store) -> Result<()> {
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    check!(eq; meets.len(), 1);
    let meet = meets.first().context("retained source meet missing")?;
    check!(eq; meet.name, "Troy Invitational #2");
    check!(eq; meet.date, "2026-03-27");
    check!(eq; meet.state, Some(UsJurisdiction::Alabama));
    check!(eq; meet.sports, vec![Sport::OutdoorTrack]);
    Ok(())
}

fn assert_projection_receipts(store: &Store, relay: bool) -> Result<()> {
    let payloads = store.journal_payloads(PHASE)?;
    let receipts: Vec<_> = payloads
        .iter()
        .filter(|row| row.get("source_completeness").is_some())
        .collect();
    check!(eq; receipts.len(), 2);
    for receipt in receipts {
        check!(eq; receipt["source_completeness"], "unknown");
        check!(eq; receipt["ownership_complete"], false);
        check!(eq; receipt["canonical_identity_accepted"], false);
        check!(eq; receipt["lifetime_pr_claimed"], false);
        check!(eq; receipt["census_sealed"], false);
    }
    let retained: Vec<_> = payloads
        .iter()
        .filter(|row| row["disposition"] == "retained_unresolved")
        .collect();
    check!(eq; retained.len(), if relay { 0 } else { 3 });
    retained.into_iter().try_for_each(assert_retained_evidence)
}

fn assert_retained_evidence(row: &Value) -> Result<()> {
    check!(eq; row["cohort"], "published");
    let evidence: Value = serde_json::from_str(
        row["evidence"]["note"]
            .as_str()
            .context("retained evidence note missing")?,
    )?;
    check!(eq; evidence["meet_name"], "Troy Invitational #2");
    check!(eq; evidence["meet_date"], "2026-03-27");
    check!(eq; evidence["sport"], "outdoor_track");
    check!(eq; evidence["school_year"], 2025);
    check!(eq; evidence["published_grad_year"],
    if row["result_id"] == 201782806 {
        2026
    } else {
        2027
    },);
    Ok(())
}
