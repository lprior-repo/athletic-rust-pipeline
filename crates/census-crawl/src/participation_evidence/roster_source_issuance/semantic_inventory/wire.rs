use crate::milesplit::{SourceRowLocator, TeamRef};
use census_domain::model::{CanonicalSchool, Gender, GradYear, SourceSchoolObservation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inventory {
    pub(super) schema: String,
    pub(super) interpretation: String,
    pub(super) provider_semantic_contract: String,
    pub(super) kernel_document: String,
    pub(super) kernel_digest: String,
    pub(super) kernel_bytes: u64,
    pub(super) packet: String,
    pub(super) parser_manifest: String,
    pub(super) metadata: String,
    pub(super) school: CanonicalSchool,
    pub(super) observation: SourceSchoolObservation,
    pub(super) directory: Directory,
    pub(super) team: TeamRef,
    pub(super) owners: Owners,
    pub(super) members: Vec<Member>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Directory {
    pub(super) capture: String,
    pub(super) acquired_at: chrono::DateTime<chrono::Utc>,
    pub(super) disposition: crate::CollectionDisposition,
    pub(super) parsed_teams: usize,
    pub(super) errors: usize,
    pub(super) unfinished: Vec<String>,
    pub(super) row: SourceRowLocator,
    pub(super) row_digest: String,
    pub(super) name: SourceRowLocator,
    pub(super) city: SourceRowLocator,
    pub(super) anchor: SourceRowLocator,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Member {
    pub(super) roster_name: String,
    pub(super) name: String,
    pub(super) gender: Gender,
    pub(super) grad_year: GradYear,
    pub(super) provider_id: String,
    pub(super) profile_url: String,
    pub(super) sports: [bool; 3],
    pub(super) row: SourceRowLocator,
    pub(super) current: Current,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Current {
    Available {
        owner: String,
        member: String,
        row: SourceRowLocator,
        section: Span,
        container: Span,
        heading: Span,
        heading_digest: String,
        sports: [bool; 3],
        statements: Statements,
    },
    Unavailable {
        reason: TemporalGap,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Statements {
    Available {
        person: Span,
        membership: Span,
        temporal: Span,
    },
    Unavailable {
        reason: WitnessGap,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Owners {
    Available { declarations: Vec<Owner> },
    Unavailable { reason: WitnessGap },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owner {
    pub(super) provider_id: u64,
    pub(super) jurisdiction: Option<census_domain::UsJurisdiction>,
    pub(super) kind: OwnerKind,
    pub(super) span: Span,
    pub(super) termination: OwnerTermination,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum OwnerKind {
    Canonical,
    OpenGraph,
    Twitter,
    JsonLdUrl,
    JsonLdId,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum OwnerTermination {
    Tag,
    ScriptEnd,
    ScriptEof,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum WitnessGap {
    MissingPublishedOwner,
    MissingSourceSpan,
    SourceSpanLimit,
    OwnerInventoryLimit,
    ResourceLimit,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum TemporalGap {
    MissingStatement,
    AmbiguousStatement,
    MalformedMarkup,
    ResourceLimit,
    InactiveMember,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Span {
    pub(super) start: usize,
    pub(super) end: usize,
}
impl From<std::ops::Range<usize>> for Span {
    fn from(value: std::ops::Range<usize>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}
