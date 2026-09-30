use census_domain::is_core_source;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, Evidence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    AllSources,
    Core,
}

impl Scope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Scope::AllSources => "all_sources",
            Scope::Core => "core",
        }
    }

    pub(super) const fn file_suffix(self) -> &'static str {
        match self {
            Scope::AllSources => "",
            Scope::Core => "-core",
        }
    }

    pub fn primary_evidence<T: CoreScoped>(self, row: &T) -> Option<&Evidence> {
        match self {
            Self::AllSources => row.evidence().first(),
            Self::Core => row
                .evidence()
                .iter()
                .find(|evidence| is_core_source(&evidence.source.id)),
        }
    }
}

pub trait CoreScoped {
    fn evidence(&self) -> &[Evidence];

    fn evidence_mut(&mut self) -> &mut Vec<Evidence>;

    fn drop_non_core_observations(&mut self) {}

    fn drop_non_core_identities(&mut self) {}
}

pub fn is_core_evidenced<T: CoreScoped>(row: &T) -> bool {
    row.evidence()
        .iter()
        .any(|evidence| is_core_source(&evidence.source.id))
}

impl CoreScoped for CanonicalAthlete {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }

    fn drop_non_core_observations(&mut self) {
        self.observed_grades
            .retain(|observation| is_core_source(&observation.source.id));
    }

    fn drop_non_core_identities(&mut self) {
        self.source_links
            .retain(|identity| identity.namespace.is_core());
    }
}

impl CoreScoped for CanonicalMeet {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }

    fn drop_non_core_identities(&mut self) {
        self.source_identities
            .retain(|identity| identity.namespace.is_core());
    }
}

impl CoreScoped for CanonicalEvent {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }
}

impl CoreScoped for CanonicalPerformance {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }
}

pub fn retain_core<T: CoreScoped>(rows: &mut Vec<T>) -> usize {
    let before = rows.len();
    for row in rows.iter_mut() {
        retain_core_row(row);
    }
    rows.retain(|row| !row.evidence().is_empty());
    before.saturating_sub(rows.len())
}

pub fn retain_core_row<T: CoreScoped>(row: &mut T) -> bool {
    row.evidence_mut()
        .retain(|evidence| is_core_source(&evidence.source.id));
    row.drop_non_core_observations();
    row.drop_non_core_identities();
    !row.evidence().is_empty()
}
