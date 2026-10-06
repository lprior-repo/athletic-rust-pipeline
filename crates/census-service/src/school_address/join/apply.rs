use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::{CanonicalSchool, ReviewCase, SourceNamespace};
use census_domain::school_directory::{
    AttestedRecord, DirectoryIndex, LinkDecision, LinkRule, ReviewReason, SourceLabel,
    StateRecordId, MAX_CO_OP_MEMBERS,
};
use census_domain::UsJurisdiction;
use census_store::{Store, StoreResult, Table};

use super::lanes::{ASSOCIATION_PREFIX, STATE_ED};
use super::support::{
    allocation, bump, bump_review_reason, parse_key_label, reason_name, row, rule_name,
};
use super::{Counters, Job, JoinError, LaneEvidence, Mode, OutcomeRow, SCHOOL_IDENTITY_FAMILY};

const OUTCOME_RESERVE: usize = 1024;
const WRITE_CHUNK: usize = 100;

impl<'a> Job<'a> {
    pub(super) fn new(
        index: &'a DirectoryIndex,
        lanes: &'a BTreeMap<String, LaneEvidence>,
        mode: Mode,
        store: &'a Store,
    ) -> Result<Self, JoinError> {
        let mut outcomes: Vec<OutcomeRow> = Vec::new();
        outcomes
            .try_reserve(OUTCOME_RESERVE)
            .map_err(|error| allocation(error.to_string()))?;
        let mut pending: Vec<CanonicalSchool> = Vec::new();
        pending
            .try_reserve_exact(WRITE_CHUNK)
            .map_err(|error| allocation(error.to_string()))?;
        let existing_cases: BTreeSet<String> = if mode == Mode::Apply {
            store
                .scan::<ReviewCase>(Table::ReviewCases)?
                .into_iter()
                .map(|case| case.id)
                .collect()
        } else {
            BTreeSet::new()
        };
        let (association, association_problem) = association_lanes(lanes);
        Ok(Self {
            index,
            lanes,
            mode,
            store,
            counters: Counters::default(),
            outcomes,
            pending,
            existing_cases,
            pending_cases: Vec::new(),
            association,
            association_problem,
            attested: Vec::new(),
        })
    }

    pub(super) fn finish(self) -> StoreResult<(Counters, Vec<OutcomeRow>)> {
        if self.mode == Mode::Apply {
            let mut wrote = false;
            if !self.pending_cases.is_empty() {
                self.store
                    .replace_many(Table::ReviewCases, &self.pending_cases)?;
                wrote = true;
            }
            if !self.pending.is_empty() {
                self.store.append_many(Table::Schools, &self.pending)?;
                wrote = true;
            }
            if wrote {
                self.store.flush()?;
            }
        }
        Ok((self.counters, self.outcomes))
    }

    fn file_review(
        &mut self,
        school: &CanonicalSchool,
        reason: &ReviewReason,
        labels: &[String],
    ) -> StoreResult<()> {
        if self.mode != Mode::Apply {
            return Ok(());
        }
        let providers: Vec<&str> = self.lanes.keys().map(String::as_str).collect();
        let detail = format!(
            "{} between candidates {}; providers {}",
            reason_name(reason),
            labels.join(", "),
            providers.join(", ")
        );
        let case = ReviewCase::pending(
            SCHOOL_IDENTITY_FAMILY,
            school.id.as_str(),
            school.name.as_str(),
            detail,
        );
        if self.existing_cases.contains(&case.id) {
            return bump(&mut self.counters.review_present);
        }
        self.pending_cases.push(case);
        bump(&mut self.counters.review_filed)
    }

    pub(super) fn visit(&mut self, school: &CanonicalSchool) -> StoreResult<()> {
        bump(&mut self.counters.scanned)?;
        let Some(state) = school.state else {
            bump(&mut self.counters.missing_state)?;
            let mut row = row(school, None, "missing_state");
            row.reason = Some("missing_state".to_string());
            self.outcomes.push(row);
            return Ok(());
        };
        self.attest(school, state)?;
        if school.co_op {
            return self.visit_co_op(school, state);
        }
        let decision = self.index.link(
            &school.name,
            &school.normalized_name,
            school.city.as_deref(),
            state,
            &school.aliases,
            &self.attested,
        );
        self.decide(school, state, decision, None)
    }

    fn visit_co_op(&mut self, school: &CanonicalSchool, state: UsJurisdiction) -> StoreResult<()> {
        let decision =
            self.index
                .link_name(&school.name, school.city.as_deref(), state, &self.attested);
        self.decide(school, state, decision, None)?;
        for member in school.aliases.iter().take(MAX_CO_OP_MEMBERS) {
            bump(&mut self.counters.co_op_members)?;
            let decision =
                self.index
                    .link_name(member, school.city.as_deref(), state, &self.attested);
            self.decide(school, state, decision, Some(member.as_str()))?;
        }
        Ok(())
    }

    fn decide(
        &mut self,
        school: &CanonicalSchool,
        state: UsJurisdiction,
        decision: LinkDecision,
        member: Option<&str>,
    ) -> StoreResult<()> {
        match decision {
            LinkDecision::Linked(matched) => self.link(school, state, &matched),
            LinkDecision::Review { reason, candidates } => {
                bump(&mut self.counters.review)?;
                bump_review_reason(&mut self.counters, &reason)?;
                let mut labels: Vec<String> = candidates
                    .iter()
                    .map(|candidate| parse_key_label(&candidate.key))
                    .collect();
                labels.sort_unstable();
                labels.dedup();
                let mut row = row(school, Some(state), "review");
                row.reason = Some(reason_name(&reason).to_string());
                row.candidates = labels.clone();
                row.detail = member.map(member_detail);
                self.outcomes.push(row);
                self.file_review(school, &reason, &labels)
            }
            LinkDecision::NoMatch => {
                let Some(member) = member else {
                    bump(&mut self.counters.no_match)?;
                    let mut row = row(school, Some(state), "no_match");
                    row.reason = Some("no_match".to_string());
                    self.outcomes.push(row);
                    return Ok(());
                };
                bump(&mut self.counters.co_op_declined)?;
                let mut row = row(school, Some(state), "co_op_declined");
                row.reason = Some("no_match".to_string());
                row.detail = Some(member_detail(member));
                self.outcomes.push(row);
                Ok(())
            }
        }
    }

    fn attest(&mut self, school: &CanonicalSchool, state: UsJurisdiction) -> StoreResult<()> {
        self.attested.clear();
        for identity in &school.source_identities {
            let label = match &identity.namespace {
                SourceNamespace::SchoolDirectory {
                    provider,
                    state: owner_state,
                } if provider == STATE_ED => SourceLabel::StateEducationAgency {
                    state: *owner_state,
                },
                SourceNamespace::AssociationSchool { association } => {
                    if self.association.as_deref() != Some(association.as_str()) {
                        continue;
                    }
                    SourceLabel::AthleticAssociation { state }
                }
                _ => continue,
            };
            let Ok(id) = StateRecordId::parse(&identity.id) else {
                continue;
            };
            self.attested
                .try_reserve(1)
                .map_err(|error| allocation(error.to_string()))?;
            self.attested.push(AttestedRecord { label, id });
        }
        Ok(())
    }

    pub(super) fn push_change(&mut self, clone: CanonicalSchool) -> StoreResult<()> {
        if self.mode == Mode::Apply {
            self.pending.push(clone);
            if self.pending.len() == WRITE_CHUNK {
                self.store.append_many(Table::Schools, &self.pending)?;
                self.pending.clear();
            }
        }
        Ok(())
    }

    pub(super) fn refuse(
        &mut self,
        school: &CanonicalSchool,
        state: UsJurisdiction,
        rule: &LinkRule,
        detail: String,
    ) -> StoreResult<()> {
        bump(&mut self.counters.refused)?;
        let mut row = row(school, Some(state), "refused");
        row.rule = Some(rule_name(rule).to_string());
        row.detail = Some(detail);
        self.outcomes.push(row);
        Ok(())
    }

    pub(super) fn missing_evidence(
        &mut self,
        school: &CanonicalSchool,
        state: UsJurisdiction,
        rule: &LinkRule,
        detail: String,
    ) -> StoreResult<()> {
        bump(&mut self.counters.evidence_missing)?;
        let mut row = row(school, Some(state), "evidence_missing");
        row.rule = Some(rule_name(rule).to_string());
        row.detail = Some(detail);
        self.outcomes.push(row);
        Ok(())
    }
}

fn member_detail(member: &str) -> String {
    format!("co-op member {member}")
}

fn association_lanes(lanes: &BTreeMap<String, LaneEvidence>) -> (Option<String>, Option<String>) {
    let mut association: Option<String> = None;
    let mut problem: Option<String> = None;
    for key in lanes.keys() {
        let Some(slug) = key.strip_prefix(ASSOCIATION_PREFIX) else {
            continue;
        };
        match &association {
            None => association = Some(slug.to_string()),
            Some(first) if first == slug => {}
            Some(first) => {
                problem = Some(format!(
                    "the generation admits several association lanes ({first}, {slug}); \
                     build one association lane per generation"
                ));
            }
        }
    }
    (association, problem)
}
