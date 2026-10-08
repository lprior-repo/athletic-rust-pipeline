use super::super::super::rosters_phase;
use super::super::roster::journal::Journal;
use super::{
    inputs, load, CensusRun, CrawlError, CrawlResult, Disposition, SourceObject, Store,
    UsJurisdiction,
};
use census_crawl::milesplit::Site;
use census_domain::model::SchoolYear;
use std::num::NonZeroU32;

pub(crate) struct RosterIndexEvidence {
    pub(crate) disposition: Disposition,
    pub(crate) owed: u64,
    pub(crate) objects: Vec<SourceObject>,
}

pub(crate) fn inspect_rosters(
    store: &Store,
    run: CensusRun,
    jurisdiction: UsJurisdiction,
) -> CrawlResult<RosterIndexEvidence> {
    let mut evidence = RosterIndexEvidence {
        disposition: Disposition::Unknown,
        owed: 0,
        objects: Vec::new(),
    };
    let Some(receipt) = load(store, jurisdiction)? else {
        evidence
            .objects
            .try_reserve_exact(1)
            .map_err(|_| super::ids::resource(1))?;
        evidence.objects.push(object(
            Site::for_jurisdiction(jurisdiction).teams_url(),
            Disposition::Unknown,
        ));
        return Ok(evidence);
    };
    let capacity = receipt
        .roster_teams
        .len()
        .checked_add(receipt.unfinished.len())
        .and_then(|value| value.checked_add(1))
        .ok_or_else(arithmetic)?;
    evidence
        .objects
        .try_reserve_exact(capacity)
        .map_err(|_| super::ids::resource(capacity))?;
    let revision = NonZeroU32::new(run.revision()).ok_or_else(arithmetic)?;
    let phase = rosters_phase(jurisdiction, run.season(), revision);
    let inputs_resolved = append_teams(
        store,
        jurisdiction,
        (&phase, run.season()),
        &receipt.roster_teams,
        &mut evidence,
    )?;
    append_index(receipt, jurisdiction, inputs_resolved, &mut evidence)?;
    Ok(evidence)
}

fn append_index(
    receipt: super::Receipt,
    jurisdiction: UsJurisdiction,
    inputs_resolved: bool,
    evidence: &mut RosterIndexEvidence,
) -> CrawlResult<()> {
    let index_status = super::measured_disposition(&receipt, inputs_resolved);
    evidence.disposition = if index_status.is_complete() && evidence.owed != 0 {
        Disposition::Partial
    } else {
        index_status
    };
    let mut index = object(
        Site::for_jurisdiction(jurisdiction).teams_url(),
        index_status,
    );
    index.observations = count(receipt.roster_teams.len())?;
    index.windows = u64::from(index_status.is_complete());
    evidence.objects.push(index);
    receipt.unfinished.into_iter().for_each(|locator| {
        evidence.objects.push(object(
            format!("{}/{locator}", jurisdiction.code()),
            Disposition::Partial,
        ))
    });
    Ok(())
}

fn append_teams(
    store: &Store,
    jurisdiction: UsJurisdiction,
    scope: (&str, SchoolYear),
    ids: &[String],
    evidence: &mut RosterIndexEvidence,
) -> CrawlResult<bool> {
    ids.iter().try_fold(true, |resolved, id| {
        let (row, input) = team(store, jurisdiction, scope, id)?;
        if !row.terminal() {
            evidence.owed = evidence.owed.checked_add(1).ok_or_else(arithmetic)?;
        }
        evidence.objects.push(row);
        Ok(resolved && input)
    })
}

fn team(
    store: &Store,
    jurisdiction: UsJurisdiction,
    scope: (&str, SchoolYear),
    id: &str,
) -> CrawlResult<(SourceObject, bool)> {
    let input = inputs::load(store, jurisdiction, id)?;
    let endpoint = input.as_ref().map_or_else(
        || format!("{}/roster-input/{id}/unmeasured", jurisdiction.code()),
        |team| format!("{}/roster", team.url),
    );
    let key = format!("{}:{id}", jurisdiction.code());
    let journal = store
        .journal_payload(scope.0, &key)?
        .map(serde_json::from_value::<Journal>)
        .transpose()
        .map_err(|source| CrawlError::Canonical {
            table: "milesplit_roster".to_string(),
            source: census_domain::model::CanonicalJsonError::Unsupported(source.to_string()),
        })?;
    let Some(journal) = journal else {
        return Ok((object(endpoint, Disposition::Unknown), input.is_some()));
    };
    if !journal.matches_scope(id, scope.1) {
        return Err(CrawlError::Invariant {
            detail: format!("foreign roster receipt at {}:{key}", scope.0),
        });
    }
    let disposition = if journal.is_terminal_for(id, scope.1) && input.is_some() {
        Disposition::Complete
    } else if journal.disposition().is_complete() {
        Disposition::Partial
    } else {
        journal.disposition()
    };
    let mut row = object(endpoint, disposition);
    row.observations = count(journal.accepted_athletes())?;
    row.windows = u64::from(disposition.is_complete());
    Ok((row, input.is_some()))
}

fn object(endpoint: String, disposition: Disposition) -> SourceObject {
    SourceObject {
        endpoint,
        disposition,
        observations: 0,
        windows: 0,
    }
}
fn count(value: usize) -> CrawlResult<u64> {
    u64::try_from(value).map_err(|_| arithmetic())
}
fn arithmetic() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "roster inventory measurement overflow".to_string(),
    }
}
