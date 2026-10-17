use super::*;

pub(super) fn assertion<'a>(
    capture: &'a material::Capture,
    member: &'a material::Member,
    evidence: &'a LocallyCheckedEvidence<'a>,
    known: KnownBounds<'a>,
    published: &'a [&'a SourceSubject],
    owner_statement: SourceSpan,
    spans: &census_crawl::milesplit::RosterRowStatementsV1,
) -> StoreResult<LocallyCheckedAssertion<'a>> {
    checked(LocallyCheckedAssertion::check(RawAssertion {
        id: object(&member.id)?,
        evidence,
        roster_qualification_claim: object(&capture.qualification)?,
        source_activation_claim: object(&capture.activation)?,
        row_locator: object(&member.id)?,
        person: person(member, known)?,
        owner: ownership(capture, published, known)?,
        relationship: relationship(capture),
        temporal: TemporalScope::ExactDay(capture.proposal.capture().observed_day()),
        sport: SportScope::Sport(capture.proposal.scope().sport()),
        known,
        membership_statement: spans.membership(),
        person_statement: spans.person(),
        owner_statement,
        temporal_statement: spans.temporal(),
    }))
}

fn person<'a>(
    member: &'a material::Member,
    known: KnownBounds<'a>,
) -> StoreResult<PersonCandidate<'a>> {
    Ok(match &member.join {
        identity::PersonJoin::Accepted {
            athlete, decision, ..
        } => PersonCandidate::Mapped {
            person: &member.person,
            athlete,
            decision: object(decision)?,
            known,
        },
        identity::PersonJoin::Unresolved { reason, .. } => PersonCandidate::Unbound {
            person: &member.person,
            gap: match reason {
                identity::JoinGap::MissingAcceptedBinding => SubjectGap::UnacceptedBinding,
                identity::JoinGap::ConflictingCandidateOwnership => SubjectGap::ProviderIdReused,
            },
        },
    })
}

fn relationship(capture: &material::Capture) -> RelationshipCandidate<'_> {
    if capture.school.co_op {
        RelationshipCandidate::RepresentsCoopProgram {
            program: capture.proposal.binding().program(),
            member: CoopMemberCandidate::Unknown,
        }
    } else {
        RelationshipCandidate::RepresentsSchoolProgram {
            school: &capture.school.id,
            program: capture.proposal.binding().program(),
        }
    }
}

fn ownership<'a>(
    capture: &'a material::Capture,
    published: &'a [&'a SourceSubject],
    known: KnownBounds<'a>,
) -> StoreResult<RawOwnership<'a>> {
    let owner = capture.proposal.scope().owner();
    let target = if capture.school.co_op {
        OwnerTarget::Program(capture.proposal.binding().program())
    } else {
        OwnerTarget::School(&capture.school.id)
    };
    Ok(RawOwnership {
        requested: owner,
        observed: ObservedOwner::Known(owner),
        published,
        governing: owner,
        target,
        binding_decision: object(&capture.activation)?,
        known,
    })
}
