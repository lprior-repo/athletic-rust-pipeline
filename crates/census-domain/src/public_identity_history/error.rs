#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HistoryError {
    #[error("public identity history exceeds {0}")]
    Limit(&'static str),
    #[error("public identity history counter overflow")]
    CounterOverflow,
    #[error("invalid public identity history field: {0}")]
    Invalid(&'static str),
    #[error("public identity history allocation failed")]
    Allocation,
    #[error("public identity history encoding failed: {0}")]
    Encoding(String),
    #[error("public identity history has an unsupported version")]
    Version,
    #[error("public identity history belongs to another installation")]
    Installation,
    #[error("public identity history sequence is not the next committed position")]
    Sequence,
    #[error("public identity history previous or final head differs")]
    Head,
    #[error("public identity history payload digest differs")]
    Payload,
    #[error("duplicate public identity history value: {0}")]
    Duplicate(&'static str),
    #[error("unknown public identity history reference: {0}")]
    Unknown(&'static str),
    #[error("identity operation requires a current canonical public ID")]
    NonCanonicalMember,
    #[error("merge target is not the oldest established public ID")]
    NonCanonicalTarget,
    #[error("candidate decision reference does not match the operation")]
    EvidenceMismatch,
    #[error("candidate has competing public identity bindings")]
    BindingConflict,
    #[error("source identifier reuse requires explicit nonoverlapping supersession")]
    SourceReuse,
    #[error("split partition does not exactly cover the affected bindings")]
    Partition,
    #[error("split would cut an accepted merge that was not explicitly revoked")]
    SurvivingMerge,
    #[error("merge was already revoked or does not belong to the affected component")]
    Revocation,
    #[error("revocation disconnects the component and requires an explicit split")]
    SplitRequired,
}
