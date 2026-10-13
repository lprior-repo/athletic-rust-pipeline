use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, CanonicalTeam, NaturalKey,
    RetainedConflict, SourceEntityKind, SourceIdentity, SourceObjectIdentity,
};

trait IdentityBearing {
    const KIND: SourceEntityKind;
    fn canonical_id(&self) -> &str;
    fn source_identities(&self) -> impl Iterator<Item = &SourceIdentity>;
}

macro_rules! identity_bearing {
    ($type:ty, $kind:expr) => {
        impl IdentityBearing for $type {
            const KIND: SourceEntityKind = $kind;

            fn canonical_id(&self) -> &str {
                self.id.as_str()
            }

            fn source_identities(&self) -> impl Iterator<Item = &SourceIdentity> {
                self.source_identities.iter()
            }
        }
    };
}

identity_bearing!(CanonicalSchool, SourceEntityKind::Schools);
identity_bearing!(CanonicalTeam, SourceEntityKind::Teams);
identity_bearing!(CanonicalCoach, SourceEntityKind::Coaches);
identity_bearing!(CanonicalMeet, SourceEntityKind::Meets);

impl IdentityBearing for CanonicalAthlete {
    const KIND: SourceEntityKind = SourceEntityKind::Athletes;

    fn canonical_id(&self) -> &str {
        self.id.as_str()
    }

    fn source_identities(&self) -> impl Iterator<Item = &SourceIdentity> {
        self.identities()
    }
}

#[derive(Default)]
pub(super) struct CanonicalPass {
    pub(super) identities: Vec<SourceObjectIdentity>,
    pub(super) collisions: Vec<RetainedConflict>,
}

pub(super) fn canonical_pass(dataset: &census_report::export::ExportDataset) -> CanonicalPass {
    let mut pass = CanonicalPass::default();
    absorb(&mut pass, dataset.schools.values());
    absorb(&mut pass, dataset.teams.values());
    absorb(&mut pass, &dataset.coaches);
    absorb(&mut pass, &dataset.athletes);
    absorb(&mut pass, &dataset.meets);
    take_collisions(&mut pass, &dataset.events);
    take_collisions(&mut pass, &dataset.performances);
    pass
}

fn absorb<'a, T: IdentityBearing + NaturalKey + 'a>(
    pass: &mut CanonicalPass,
    entities: impl IntoIterator<Item = &'a T>,
) {
    for entity in entities {
        for identity in entity.source_identities() {
            let mut row = SourceObjectIdentity::new(
                identity.namespace.clone(),
                T::KIND,
                identity.id.clone(),
                entity.canonical_id(),
            );
            if let Some(url) = identity.url.clone() {
                row = row.with_url(url);
            }
            pass.identities.push(row);
        }
        pass.collisions
            .extend(entity.retained_conflicts().iter().cloned());
    }
}

fn take_collisions<'a, T: NaturalKey + 'a>(
    pass: &mut CanonicalPass,
    entities: impl IntoIterator<Item = &'a T>,
) {
    for entity in entities {
        pass.collisions
            .extend(entity.retained_conflicts().iter().cloned());
    }
}
