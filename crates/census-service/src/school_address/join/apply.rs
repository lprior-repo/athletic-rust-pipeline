use census_domain::model::{CanonicalSchool, ReviewCase};
use census_domain::school_directory::{LinkDecision, LinkRule, ReviewReason, MAX_CO_OP_MEMBERS};
use census_domain::UsJurisdiction;
use census_store::StoreResult;

use super::attest::owned_record;
use super::support::{allocation, bump, bump_review_reason, reason_name, row, rule_name};
use super::{Job, Mode, SCHOOL_IDENTITY_FAMILY};

pub(super) mod context;

impl<'a> Job<'a> {
    fn file_review(
        &mut self,
        school: &CanonicalSchool,
        reason: &ReviewReason,
        labels: &[String],
    ) -> StoreResult<()> {
        if self.mode != Mode::Apply {
            return Ok(());
        }
        let providers: Vec<&str> = self.lanes.tokens().map(String::as_str).collect();
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
        self.pending_cases
            .try_reserve(1)
            .map_err(|error| allocation(error.to_string()))?;
        self.pending_cases.push(case);
        bump(&mut self.counters.review_filed)
    }

    pub(super) fn visit_school(&mut self, school: &CanonicalSchool) -> StoreResult<()> {
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
                    .map(|candidate| candidate.key.label())
                    .collect();
                labels.sort_unstable();
                labels.dedup();
                let mut row = row(school, Some(state), "review");
                row.reason = Some(reason_name(&reason).to_string());
                row.candidates = labels.clone();
                row.detail = match member {
                    Some(member) => Some(member_detail(member)),
                    None => match self.gap_reason(school, state, None) {
                        "no_match" => None,
                        reason => Some(reason.to_string()),
                    },
                };
                self.outcomes.push(row);
                self.file_review(school, &reason, &labels)
            }
            LinkDecision::NoMatch => {
                let Some(member) = member else {
                    bump(&mut self.counters.no_match)?;
                    let mut row = row(school, Some(state), "no_match");
                    row.reason = Some(self.gap_reason(school, state, None).to_string());
                    self.outcomes.push(row);
                    return Ok(());
                };
                bump(&mut self.counters.co_op_declined)?;
                let mut row = row(school, Some(state), "co_op_declined");
                row.reason = Some(self.gap_reason(school, state, Some(member)).to_string());
                row.detail = Some(member_detail(member));
                self.outcomes.push(row);
                Ok(())
            }
        }
    }

    fn attest(&mut self, school: &CanonicalSchool, state: UsJurisdiction) -> StoreResult<()> {
        self.attested.clear();
        for identity in &school.source_identities {
            let record =
                owned_record(identity, state, self.association.as_deref()).map_err(|error| {
                    census_store::StoreError::Invariant {
                        detail: format!(
                            "school {} has an invalid directory identity: {error}",
                            school.id.as_str()
                        ),
                    }
                })?;
            let Some(record) = record else {
                continue;
            };
            self.attested
                .try_reserve(1)
                .map_err(|error| allocation(error.to_string()))?;
            self.attested.push(record);
        }
        Ok(())
    }

    pub(super) fn push_change(&mut self, clone: CanonicalSchool) -> StoreResult<()> {
        self.record_postal(&clone);
        if self.mode == Mode::Apply {
            self.pending
                .try_reserve(1)
                .map_err(|error| allocation(error.to_string()))?;
            self.pending.push(clone);
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
