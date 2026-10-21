use super::*;
use census_crawl::milesplit::{LocatedRosterAthlete, RosterRowStatementsV1};
use census_domain::affiliation_candidates::OwnerSourceSpan;
use census_domain::model::{CanonicalSchool, SourceCapture, SourceNamespace, SourceRef};
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use census_store::ReviewedRosterSourceClaimV1;

pub(super) struct Capture {
    pub(super) proposal: LocallyCheckedRosterIssuanceV1,
    pub(super) school: CanonicalSchool,
    pub(super) body: String,
    pub(super) metadata: SourceCapture,
    pub(super) source: SourceRef,
    pub(super) manifest: Vec<u8>,
    pub(super) interpretation: Vec<u8>,
    pub(super) qualification: String,
    pub(super) activation: String,
    pub(super) owner_span: Result<Vec<OwnerSourceSpan>, AuthorityGap>,
    pub(super) members: Vec<Member>,
}

pub(super) struct Member {
    pub(super) located: LocatedRosterAthlete,
    pub(super) person: SourceSubject,
    pub(super) join: identity::PersonJoin,
    pub(super) id: String,
    pub(super) spans: Result<RosterRowStatementsV1, AuthorityGap>,
}

mod input;
pub(super) use input::read;

fn member_record(
    people: &identity::PersonIndex,
    proposal: &LocallyCheckedRosterIssuanceV1,
    member: &LocatedRosterAthlete,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Member> {
    charge(grant, member.athlete().athlete_id.len(), 1)?;
    let person = checked(SourceSubject::new(
        SourceNamespace::MilesplitAthlete,
        member.athlete().athlete_id.clone(),
    ))?;
    let join = identity::join(people, &person, grant)?;
    let id = census_domain::model::serialized_digest(&(proposal, &person, member.locator()))?;
    let spans = member
        .current_roster()
        .ok_or(AuthorityGap::MissingCurrentStatement)
        .and_then(|statement| {
            statement
                .source_statements_v1()
                .cloned()
                .map_err(|error| AuthorityGap::RowStatement(error.to_string()))
        });
    Ok(Member {
        located: member.clone(),
        person,
        join,
        id,
        spans,
    })
}

impl Capture {
    pub(super) fn inventory(&self) -> impl serde::Serialize + '_ {
        (
            &self.proposal,
            &self.qualification,
            &self.activation,
            format!("{:?}", self.owner_span),
            self.members
                .iter()
                .map(|member| {
                    (
                        &member.id,
                        &member.person,
                        &member.join,
                        member.located.locator(),
                        format!("{:?}", member.spans),
                    )
                })
                .collect::<Vec<_>>(),
        )
    }

    pub(super) fn evidence(&self) -> StoreResult<LocallyCheckedEvidence<'_>> {
        let capture = self.proposal.capture();
        let parser = self.proposal.parser();
        checked(LocallyCheckedEvidence::check_roster_interpretation(
            RawEvidence {
                occurrence: object(capture.occurrence().as_str())?,
                manifest_key: object(capture.manifest().digest().as_str())?,
                manifest_bytes: &self.manifest,
                metadata: &self.metadata,
                owner: self.proposal.scope().owner(),
                source: &self.source,
                requested_url: capture.requested_url(),
                final_url: match capture.observed_final_url() {
                    Some(url) => FinalUrl::Observed(url),
                    None => FinalUrl::Unknown,
                },
                acquired_at: checked(chrono::DateTime::parse_from_rfc3339(capture.acquired_at()))?
                    .to_utc(),
                body: self.body.as_bytes(),
                interpretation_key: object(capture.interpretation().digest().as_str())?,
                interpretation_bytes: &self.interpretation,
                parser_artifact: object(parser.artifact().digest().as_str())?,
                extraction_schema: object(parser.schema().digest().as_str())?,
                extraction_config: object(parser.configuration().digest().as_str())?,
                reference_dependencies: object(parser.dependencies().digest().as_str())?,
            },
            &self.proposal,
        ))
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) enum AuthorityGap {
    PublishedOwner(String),
    RowStatement(String),
    MissingCurrentStatement,
}

fn owners(
    packet: &census_crawl::participation_evidence::provider_school::CheckedProviderSchoolPacketV1,
    proposal: &LocallyCheckedRosterIssuanceV1,
) -> StoreResult<Result<Vec<OwnerSourceSpan>, AuthorityGap>> {
    let owners = packet.published_roster_owners_v1();
    Ok(match owners {
        Ok(owners) => {
            if owners.declarations().iter().any(|owner| {
                owner.provider_id().to_string() != proposal.scope().owner().provider_id()
            }) {
                return Err(invalid("reviewed published roster owner differs"));
            }
            Ok(owners
                .declarations()
                .iter()
                .map(|owner| owner.span())
                .collect())
        }
        Err(error) => Err(AuthorityGap::PublishedOwner(error.to_string())),
    })
}
