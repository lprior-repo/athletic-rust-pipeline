#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "{} is outside the census run scope: the 48 continental states plus the District of Columbia \
     (ADR-009)",
    .0.code()
)]
pub struct OutsideCensusScope(pub super::table::UsJurisdiction);
