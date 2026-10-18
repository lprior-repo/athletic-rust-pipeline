use super::audit::RetainedAudit;
use super::{
    capture::Captures, invalid, note::RowNote, CurrentAffiliationResolution as Resolution,
    UnavailableAffiliation as Gap,
};
use crate::export::public_identity::population::{IdentityAdmission, IdentityPopulation};
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalSchool, Evidence, QueryProfileV1, SourceNamespace,
};
use census_store::StoreResult;
use std::collections::BTreeMap;
#[cfg(test)]
use std::path::Path;

mod member;
mod temporal;
use member::{selected_sport, validate_member, validate_owner, validate_source};

type Outcomes = BTreeMap<AthleteId, Resolution>;

#[derive(Clone, Copy)]
enum Interpretation {
    HistoricalCaptureDay,
    ScopedCurrentStatement,
}

pub(super) struct Reader<'a> {
    identities: IdentityPopulation<'a>,
    interpretation: Interpretation,
    profile: &'a QueryProfileV1,
    people: BTreeMap<&'a str, &'a AthleteId>,
    schools: BTreeMap<&'a str, Option<&'a CanonicalSchool>>,
    captures: Captures,
    rows: Outcomes,
    audit: RetainedAudit,
}

impl<'a> Reader<'a> {
    #[cfg(test)]
    pub(super) fn new(
        dataset: &'a ExportDataset,
        profile: &'a QueryProfileV1,
        root: &Path,
    ) -> ReportResult<Self> {
        Self::with_captures(dataset, profile, Captures::new(root)?)
    }

    pub(super) fn with_captures(
        dataset: &'a ExportDataset,
        profile: &'a QueryProfileV1,
        captures: Captures,
    ) -> ReportResult<Self> {
        if dataset.athletes.len() > 200_000 || dataset.schools.len() > 100_000 {
            return Err(invalid("current affiliation entity inventory exceeds its ceiling").into());
        }
        let people = dataset
            .athletes
            .iter()
            .map(|row| (row.id.as_str(), &row.id))
            .collect();
        let mut schools: BTreeMap<&str, Option<&CanonicalSchool>> = BTreeMap::new();
        for school in dataset.schools.values() {
            for owner in school
                .source_identities
                .iter()
                .filter(|owner| owner.namespace == SourceNamespace::MilesplitSchool)
            {
                match schools.entry(owner.id.as_str()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(Some(school));
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry) => {
                        if entry.get().is_some_and(|previous| previous.id != school.id) {
                            entry.insert(None);
                        }
                    }
                }
            }
        }
        Ok(Self {
            identities: IdentityPopulation::of(dataset)?,
            interpretation: if dataset.is_historical_selected_replay() {
                Interpretation::HistoricalCaptureDay
            } else {
                Interpretation::ScopedCurrentStatement
            },
            profile,
            people,
            schools,
            captures,
            rows: BTreeMap::new(),
            audit: RetainedAudit::default(),
        })
    }

    pub(super) fn accept(&mut self, athlete: CanonicalAthlete) -> StoreResult<()> {
        let canonical = self
            .identities
            .canonical_candidate(athlete.id.as_str())
            .ok_or_else(|| invalid("pinned observation has no retained identity subject"))?;
        let id = self
            .people
            .get(canonical)
            .ok_or_else(|| invalid("pinned observation has no canonical dataset subject"))?;
        self.rows
            .entry((*id).clone())
            .or_insert(Resolution::Unknown);
        if self.identities.admission(athlete.id.as_str()) != IdentityAdmission::Accepted
            || self.identities.admission(canonical) != IdentityAdmission::Accepted
        {
            self.audit.gap((*id).clone(), Gap::IdentityUnaccepted)?;
            return Ok(());
        }
        let id = (*id).clone();
        let mut recognized = false;
        for evidence in &athlete.evidence {
            let Some(note) = RowNote::read(evidence)? else {
                continue;
            };
            recognized = true;
            self.row(&id, &athlete, evidence, &note)?;
        }
        if !recognized {
            self.audit.gap(id, Gap::NoRetainedRow)?;
        }
        Ok(())
    }

    fn row(
        &mut self,
        id: &AthleteId,
        athlete: &CanonicalAthlete,
        evidence: &Evidence,
        note: &RowNote,
    ) -> StoreResult<()> {
        validate_owner(athlete, note)?;
        let day = note.acquired_at()?.date_naive().to_string();
        let Some(school) = self
            .schools
            .get(note.provider_team_id.as_str())
            .copied()
            .flatten()
        else {
            self.unavailable(id, &day, Gap::SchoolUnresolved)?;
            return Ok(());
        };
        if !school.retained_conflicts.is_empty() || school.id != athlete.school {
            self.unavailable(id, &day, Gap::SchoolUnresolved)?;
            return Ok(());
        }
        let Some(capture) = self.captures.get(school, note)? else {
            self.unavailable(id, &day, Gap::ArchiveUnavailable)?;
            return Ok(());
        };
        validate_source(athlete, evidence, school)?;
        let member = validate_member(athlete, note, capture)?;
        let selected_sport = selected_sport(member, self.profile);
        self.audit.context.retain(&athlete.id, evidence)?;
        observe_pinned(
            &mut self.audit,
            &mut self.rows,
            self.interpretation,
            self.profile,
            id,
            athlete,
            evidence,
            note,
            member,
            capture,
            school,
            &day,
            selected_sport,
        )
    }

    fn unavailable(&mut self, id: &AthleteId, day: &str, gap: Gap) -> StoreResult<()> {
        let gap = if day == self.profile.input().as_of.as_str() {
            gap
        } else {
            Gap::OutsideCaptureDay
        };
        self.audit.gap(id.clone(), gap)
    }

    pub(super) fn finish(self) -> (Outcomes, RetainedAudit) {
        let (rows, audit, _) = self.finish_with_retention();
        (rows, audit)
    }

    pub(super) fn finish_with_retention(
        mut self,
    ) -> (Outcomes, RetainedAudit, Option<super::frozen::assets::Sink>) {
        for (id, gaps) in self.audit.unavailable() {
            if gaps.iter().any(|gap| {
                matches!(
                    gap,
                    Gap::IdentityUnaccepted
                        | Gap::ArchiveUnavailable
                        | Gap::SchoolUnresolved
                        | Gap::MissingTemporalAuthority
                )
            }) {
                if let Some(outcome) = self.rows.get_mut(id) {
                    outcome.withhold();
                }
            }
        }
        (self.rows, self.audit, self.captures.retention())
    }
}

fn observe_pinned(
    audit: &mut RetainedAudit,
    rows: &mut Outcomes,
    interpretation: Interpretation,
    profile: &QueryProfileV1,
    id: &AthleteId,
    athlete: &CanonicalAthlete,
    evidence: &Evidence,
    note: &RowNote,
    member: &census_crawl::milesplit::LocatedRosterAthlete,
    capture: &super::capture::CapturedRoster,
    school: &CanonicalSchool,
    day: &str,
    selected_sport: bool,
) -> StoreResult<()> {
    if day != profile.input().as_of.as_str() {
        audit.gap(id.clone(), Gap::OutsideCaptureDay)?;
        return Ok(());
    }
    let mut evidence_refs = vec![note.source_row_key.clone(), capture.metadata_ref.clone()];
    evidence_refs.sort();
    evidence_refs.dedup();
    for reference in &evidence_refs {
        audit.reference(id, reference)?;
    }
    if !selected_sport {
        audit.gap(id.clone(), Gap::OutsideSelectedSport)?;
        return Ok(());
    }
    if matches!(interpretation, Interpretation::ScopedCurrentStatement) {
        if let temporal::Outcome::Unavailable(diagnostic) = temporal::retain(
            &mut audit.context,
            &athlete.id,
            evidence,
            note,
            member,
            profile,
        )? {
            audit.gap(id.clone(), Gap::MissingTemporalAuthority)?;
            if let Some(reason) = diagnostic {
                audit.gap(id.clone(), reason)?;
            }
            return Ok(());
        }
    }
    let current = rows.entry(id.clone()).or_insert(Resolution::Unknown);
    current.observe_school(school, evidence_refs);
    Ok(())
}
