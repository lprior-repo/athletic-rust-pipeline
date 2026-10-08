mod publication;

use super::obligation;
use crate::restate_services::{wire::SourceObjectOpen, JobError};
use census_crawl::CollectionDisposition as Disposition;
use census_domain::model::{
    CanonicalSchool, CensusRun, ContactResearch, ContactResearchOutcome, GradYear,
    SchoolMailboxPurpose, SchoolYear,
};
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
use census_report::export::store_identity;
use census_store::{Store, StoreError, StoreResult, Table};

pub(crate) struct StoreEvidence {
    season: SchoolYear,
    revision: Revision,
    pub(super) bound: bool,
    pub(super) objects: Vec<SourceObjectOpen>,
    pub(super) publication: Disposition,
    contacts: Vec<ContactTally>,
    rosters: Vec<RosterTally>,
}

struct ContactTally {
    jurisdiction: UsJurisdiction,
    schools: u64,
    owed: u64,
}
struct RosterTally {
    jurisdiction: UsJurisdiction,
    disposition: Disposition,
    owed: u64,
}

impl StoreEvidence {
    pub(super) fn matches(&self, season: SchoolYear, revision: Revision) -> bool {
        self.season == season && self.revision == revision
    }
    pub(super) fn contacts(&self, jurisdiction: UsJurisdiction) -> Disposition {
        self.contacts
            .iter()
            .find(|row| row.jurisdiction == jurisdiction)
            .map_or(Disposition::Unknown, |row| {
                if row.schools > 0 && row.owed == 0 && self.bound {
                    Disposition::Complete
                } else {
                    Disposition::Unknown
                }
            })
    }
    pub(super) fn rosters(&self, jurisdiction: UsJurisdiction) -> (Disposition, u64) {
        self.rosters
            .iter()
            .find(|row| row.jurisdiction == jurisdiction)
            .map_or((Disposition::Unknown, 0), |row| {
                (
                    if self.bound {
                        row.disposition
                    } else {
                        Disposition::Unknown
                    },
                    row.owed,
                )
            })
    }
}

pub(crate) fn inspect_store(
    store: &Store,
    season: SchoolYear,
    revision: Revision,
) -> Result<StoreEvidence, JobError> {
    let run = CensusRun::new(season, revision.get()).ok_or_else(|| JobError::Terminal {
        message: "invalid census run revision".to_string(),
    })?;
    let bound = bound_store(store, run)?;
    let mut contacts = Vec::new();
    contacts
        .try_reserve_exact(UsJurisdiction::CENSUS_SCOPE.len())
        .map_err(|_| capacity())?;
    contacts.extend(
        UsJurisdiction::CENSUS_SCOPE
            .into_iter()
            .map(|jurisdiction| ContactTally {
                jurisdiction,
                schools: 0,
                owed: 0,
            }),
    );
    let mut evidence = StoreEvidence {
        season,
        revision,
        bound,
        objects: Vec::new(),
        publication: Disposition::Unknown,
        contacts,
        rosters: Vec::new(),
    };
    roster_obligations(store, run, &mut evidence)?;
    let snapshot = store.snapshot();
    snapshot.for_each_merged(Table::Schools, |school: CanonicalSchool| {
        school_obligations(&mut evidence, &school)
    })?;
    unmeasured_school_inventories(&mut evidence)?;
    if bound {
        evidence.publication = publication::inspect(store, run);
    }
    append(
        &mut evidence.objects,
        obligation(
            "publication/current-generation".to_string(),
            evidence.publication,
        ),
    )?;
    append(
        &mut evidence.objects,
        obligation("store/run-binding".to_string(), super::complete(bound)),
    )?;
    Ok(evidence)
}

fn unmeasured_school_inventories(evidence: &mut StoreEvidence) -> StoreResult<()> {
    evidence
        .contacts
        .iter()
        .filter(|row| row.schools == 0)
        .try_for_each(|row| {
            append(
                &mut evidence.objects,
                obligation(
                    format!(
                        "{}/contact-research/school-inventory-unmeasured",
                        row.jurisdiction.code()
                    ),
                    Disposition::Unknown,
                ),
            )
        })
}

fn roster_obligations(
    store: &Store,
    run: CensusRun,
    evidence: &mut StoreEvidence,
) -> Result<(), JobError> {
    evidence
        .rosters
        .try_reserve_exact(UsJurisdiction::CENSUS_SCOPE.len())
        .map_err(|_| capacity())?;
    UsJurisdiction::CENSUS_SCOPE
        .into_iter()
        .try_for_each(|jurisdiction| {
            let roster: crate::census::RosterIndexEvidence =
                crate::census::inspect_rosters(store, run, jurisdiction)
                    .map_err(crate::restate_services::jobs::collect_error)?;
            roster.objects.into_iter().try_for_each(|row| {
                append(
                    &mut evidence.objects,
                    SourceObjectOpen {
                        endpoint: row.endpoint,
                        observations: row.observations,
                        windows: row.windows,
                        unreadable: false,
                        disposition: row.disposition,
                    },
                )
            })?;
            evidence.rosters.push(RosterTally {
                jurisdiction,
                disposition: roster.disposition,
                owed: roster.owed,
            });
            Ok::<_, JobError>(())
        })
}

fn bound_store(store: &Store, run: CensusRun) -> Result<bool, JobError> {
    let Some(manifest) = store.run_manifest()? else {
        return Ok(false);
    };
    if manifest.run != run
        || manifest.cohort != GradYear::CO2027
        || manifest.jurisdictions.len() != UsJurisdiction::CENSUS_SCOPE.len()
    {
        return Ok(false);
    }
    let identity = store_identity(store).map_err(|error| JobError::Terminal {
        message: format!("store identity check failed: {error}"),
    })?;
    Ok(manifest.store_identity == identity
        && UsJurisdiction::CENSUS_SCOPE.iter().all(|jurisdiction| {
            manifest
                .jurisdictions
                .iter()
                .filter(|selected| *selected == jurisdiction)
                .count()
                == 1
        }))
}

fn school_obligations(evidence: &mut StoreEvidence, school: &CanonicalSchool) -> StoreResult<()> {
    let Some(jurisdiction) = school.state else {
        return append(
            &mut evidence.objects,
            obligation(
                format!("{}/contact/jurisdiction-unmeasured", school.id),
                Disposition::Unknown,
            ),
        );
    };
    let Some(tally) = evidence
        .contacts
        .iter_mut()
        .find(|row| row.jurisdiction == jurisdiction)
    else {
        return Ok(());
    };
    tally.schools = tally
        .schools
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    ContactResearch::programs()
        .into_iter()
        .try_for_each(|program| {
            let outcome =
                census_domain::model::school_contact_research(school, &program, evidence.season);
            record(
                &mut evidence.objects,
                tally,
                format!("{}/contact/{program:?}", school.id),
                outcome,
            )
        })?;
    [SchoolMailboxPurpose::SchoolOffice, SchoolMailboxPurpose::AthleticsOffice].into_iter().try_for_each(|purpose| {
        let outcome = census_domain::model::school_mailbox_research(school, purpose, evidence.season);
        let outcome = match census_domain::model::school_mailbox(school, purpose, evidence.season) {
            Ok(None) if outcome == ContactResearchOutcome::CompletedClaims => ContactResearchOutcome::Partial,
            Ok(_) => outcome,
            Err(error) => { tracing::warn!(%error, school = %school.id, ?purpose, "mailbox research is not admissible"); ContactResearchOutcome::Conflict }
        };
        record(&mut evidence.objects, tally, format!("{}/contact/{purpose:?}", school.id), outcome)
    })
}

fn record(
    rows: &mut Vec<SourceObjectOpen>,
    tally: &mut ContactTally,
    endpoint: String,
    outcome: ContactResearchOutcome,
) -> StoreResult<()> {
    let disposition = research_disposition(outcome);
    if !disposition.is_complete() {
        tally.owed = tally
            .owed
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
    }
    append(rows, obligation(endpoint, disposition))
}

fn research_disposition(outcome: ContactResearchOutcome) -> Disposition {
    match outcome {
        ContactResearchOutcome::CompletedEmpty | ContactResearchOutcome::CompletedClaims => {
            Disposition::Complete
        }
        ContactResearchOutcome::Blocked => Disposition::Blocked,
        ContactResearchOutcome::Failed => Disposition::Failed,
        ContactResearchOutcome::Exhausted => Disposition::Exhausted,
        ContactResearchOutcome::Partial
        | ContactResearchOutcome::Ambiguous
        | ContactResearchOutcome::Conflict => Disposition::Partial,
        ContactResearchOutcome::Unattempted | ContactResearchOutcome::Stale => Disposition::Unknown,
    }
}

fn append(rows: &mut Vec<SourceObjectOpen>, row: SourceObjectOpen) -> StoreResult<()> {
    if rows.len() >= super::MAX_OBJECTS {
        return Err(capacity());
    }
    rows.try_reserve(1).map_err(|_| capacity())?;
    rows.push(row);
    Ok(())
}

fn capacity() -> StoreError {
    StoreError::Refused {
        detail: "open-work store evidence capacity exhausted; completion is unmeasured".to_string(),
    }
}
