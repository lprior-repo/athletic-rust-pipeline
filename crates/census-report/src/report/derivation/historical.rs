use super::{is_core_evidenced, CanonicalMeet, CanonicalSchool, Derivation, Scope};

impl Derivation<'_> {
    pub(crate) fn historical_schools(&self) -> impl Iterator<Item = &CanonicalSchool> {
        self.dataset.schools.values()
    }

    pub(crate) fn historical_meets(&self) -> impl Iterator<Item = &CanonicalMeet> {
        self.dataset
            .meets
            .iter()
            .filter(|meet| self.scope != Scope::Core || is_core_evidenced(*meet))
    }
}
