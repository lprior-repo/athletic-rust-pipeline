use super::super::{population::Population, TestResult};
use census_domain::model::*;
use census_domain::school_directory::{
    AddressKind, CityName, PostalAddress, SourceLabel, StreetLine, ZipCode,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) const DAY: &str = "2026-09-01";
pub(super) const STAFF_URL: &str = "https://staff.example.test/track";
pub(super) const ADDRESS_URL: &str = "https://mhsaa.test/readback-high";
pub(super) const RETRIEVED: &str = "2026-09-01T12:00:00Z";

pub(super) struct Positive {
    pub population: Population,
    pub members: [CanonicalAthlete; 2],
    pub decision: String,
    pub address: SchoolPostalAddress,
}

impl Positive {
    pub fn new() -> TestResult<Self> {
        let mut population = Population::new()?;
        let store = Store::open(population.directory.path().join("store"))?;
        let address = postal(&mut population.school)?;
        store.append(Table::Schools, &population.school)?;
        for coach in &mut population.coaches {
            admit(coach, population.options.school_year);
            store.append(Table::Coaches, coach)?;
        }
        let members = super::identity::members(&population.school.id)?;
        store.append_many(Table::Athletes, &members)?;
        let decision = super::identity::accept(&store, &members)?;
        let dataset = crate::export::ExportDataset::load(&store)?;
        drop(store);
        let archive = population.directory.path().join("positive-frozen.json");
        dataset.save_frozen(&archive)?;
        population.dataset = crate::export::ExportDataset::reopen_frozen(&archive)?;
        check!(eq; population.dataset.lineage.input_generation, dataset.lineage.input_generation);
        Ok(Self {
            population,
            members,
            decision,
            address,
        })
    }

    pub fn canonical(&self) -> &AthleteId {
        std::cmp::min(&self.members[0].id, &self.members[1].id)
    }

    pub fn alias(&self) -> &AthleteId {
        std::cmp::max(&self.members[0].id, &self.members[1].id)
    }

    pub fn write(&self, directory: &std::path::Path) -> TestResult {
        let population = &self.population;
        let bests = crate::bests::build_from_dataset(
            &population.dataset,
            &crate::bests::Options {
                scope: population.options.scope,
                grad_year: population.options.grad_year,
                limit: population.options.limit,
                jurisdictions: population.options.jurisdictions.clone(),
            },
        )?;
        crate::workbook::publication::normalized::write(
            directory,
            &population.dataset,
            &population.options,
            &bests,
            &[],
        )?;
        Ok(())
    }
}

fn admit(coach: &mut CanonicalCoach, year: SchoolYear) {
    let source = SourceRef::new("fixture_staff", Some(STAFF_URL.into()));
    coach.evidence.push(Evidence::parsed(source.clone(), DAY));
    coach.tenure_evidence.push(CoachTenureEvidence {
        tenure: CoachTenure::Current { school_year: year },
        source,
        source_sha256: "c".repeat(64),
        retrieved_at: RETRIEVED.into(),
        statement: "Published girls outdoor track appointment for the selected school year".into(),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: coach.school.clone(),
            role: coach.role,
            program: CoachContactProgram::Team {
                sport: Sport::OutdoorTrack,
                gender: Gender::Girls,
            },
            mailbox: coach.professional_email.clone(),
        }),
    });
}

fn postal(school: &mut CanonicalSchool) -> TestResult<SchoolPostalAddress> {
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("mhsaa"),
        "readback-high",
    );
    let address = PostalAddress::of(
        Some(StreetLine::parse("123 Runner Way")?),
        Some(StreetLine::parse("Athletics Office")?),
        Some(CityName::parse("Lansing")?),
        Some(UsJurisdiction::Michigan),
        Some(ZipCode::parse("48901")?),
    )
    .ok_or("missing fixture postal address")?
    .with_kind(AddressKind::Physical);
    let claim = SchoolPostalAddress::new(
        address,
        owner.clone(),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Michigan,
        },
        Evidence::parsed(SourceRef::new("mhsaa", Some(ADDRESS_URL.into())), DAY),
        "b".repeat(64),
    )?;
    school.source_identities.push(owner);
    school.add_postal_address(claim.clone())?;
    Ok(claim)
}
