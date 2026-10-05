use std::collections::BTreeMap;

use crate::UsJurisdiction;

use super::address::PostalAddress;
use super::ids::StateRecordId;
use super::key::IdentifiedKey;
use super::label::SourceLabel;
use super::name::MatchForm;

const MAX_CANDIDATES: usize = 8;
const MAX_ALIASES: usize = 8;

pub const MAX_CO_OP_MEMBERS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkRule {
    ExactName,
    CoreName,
    Parenthetical,
    ParentheticalInner,
    Alias,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRef {
    pub key: IdentifiedKey,
    pub source: SourceLabel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkMatch {
    pub key: IdentifiedKey,
    pub source: SourceLabel,
    pub address: Option<PostalAddress>,
    pub website: Option<String>,
    pub rule: LinkRule,
    pub matched_form: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedRecord {
    pub label: SourceLabel,
    pub id: StateRecordId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewReason {
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkDecision {
    Linked(Box<LinkMatch>),
    Review {
        reason: ReviewReason,
        candidates: Vec<CandidateRef>,
    },
    NoMatch,
}

struct IndexEntry {
    key: IdentifiedKey,
    source: SourceLabel,
    city: Option<MatchForm>,
    state: UsJurisdiction,
    address: Option<PostalAddress>,
    website: Option<String>,
    middle: bool,
}

pub struct DirectoryIndex {
    entries: Vec<IndexEntry>,
    by_name: BTreeMap<MatchForm, Vec<u32>>,
    by_core: BTreeMap<MatchForm, Vec<u32>>,
}

struct Form {
    rule: LinkRule,
    form: MatchForm,
}

mod attest;
mod forms;
mod index;
