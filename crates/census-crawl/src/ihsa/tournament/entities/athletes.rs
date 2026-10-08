use super::super::attestation::CaptureLineage;
use super::super::map::{AthleteRow, Mapper, ASSOCIATION};
use super::push_once;
use census_domain::model::{
    AthleteId, CanonicalAthlete, Evidence, GradYear, ObservedGrade, SourceAthleteObservation,
    SourceIdentity, SourceNamespace,
};

type SourceIdentities = std::iter::Flatten<std::array::IntoIter<Option<SourceIdentity>, 3>>;

struct Admitted<'a> {
    name: &'a str,
    grad_year: GradYear,
    observation: ObservedGrade,
    source: SourceIdentity,
    links: SourceIdentities,
}

impl Mapper<'_> {
    pub(in crate::ihsa::tournament) fn athlete(
        &mut self,
        row: AthleteRow<'_>,
        evidence: Evidence,
    ) -> Option<(AthleteId, SourceIdentity)> {
        let admitted = self.admit_row(&row, &evidence)?;
        let id = CanonicalAthlete::mint(
            row.school,
            admitted.name,
            admitted.grad_year,
            row.gender,
            &admitted.source,
        );
        let athlete = self
            .accumulated
            .athletes
            .entry(id.as_str().to_owned())
            .or_insert_with(|| {
                CanonicalAthlete::new(
                    row.school,
                    admitted.name,
                    admitted.grad_year,
                    row.gender,
                    admitted.source.clone(),
                )
            });
        append_claim(
            athlete,
            &admitted.source,
            self.capture.as_ref(),
            &row.source_key,
        );
        push_once(&mut athlete.observed_grades, admitted.observation);
        push_once(&mut athlete.sports, row.sport);
        admitted.links.for_each(|identity| {
            append_claim(athlete, &identity, self.capture.as_ref(), &row.source_key);
            athlete.add_identity(identity);
        });
        push_once(&mut athlete.evidence, evidence);
        Some((id, admitted.source))
    }

    fn admit_row<'a>(&mut self, row: &AthleteRow<'a>, evidence: &Evidence) -> Option<Admitted<'a>> {
        let name = row.name.map(str::trim).filter(|value| !value.is_empty())?;
        let observation = ObservedGrade {
            grade: row.grade?,
            school_year: row.school_year,
            source: evidence.source.clone(),
        };
        let mut identities = identities(row.net_id, row.live_id, row.entry.as_deref())
            .into_iter()
            .flatten();
        let source = match identities.next() {
            Some(identity) => identity,
            None => SourceIdentity::new(
                SourceNamespace::Other("ihsa_result_row".into()),
                &row.source_key,
            ),
        };
        let (grad_year, observation, source) =
            self.accumulated
                .unsupported
                .admit(observation, source, |source| {
                    SourceAthleteObservation::new(
                        source.namespace,
                        source.id,
                        &row.source_key,
                        name,
                        &self.origin.observed_on,
                    )
                    .with_gender(row.gender)
                })?;
        Some(Admitted {
            name,
            grad_year,
            observation,
            source,
            links: identities,
        })
    }
}

fn append_claim(
    athlete: &mut CanonicalAthlete,
    subject: &SourceIdentity,
    capture: Option<&CaptureLineage>,
    locator: &str,
) {
    if let Some(capture) = capture {
        let exists = athlete.identity_attestations.iter().any(|claim| {
            claim.subject.namespace == subject.namespace
                && claim.subject.id == subject.id
                && claim.subject_locator == locator
                && capture.matches(claim)
        });
        if !exists {
            athlete
                .identity_attestations
                .push(capture.claim(subject, locator));
        }
    }
}

fn identities(
    net_id: Option<u64>,
    live_id: Option<u64>,
    entry: Option<&str>,
) -> [Option<SourceIdentity>; 3] {
    let association = entry.map(|id| {
        SourceIdentity::new(
            SourceNamespace::AssociationAthlete {
                association: ASSOCIATION.into(),
            },
            id,
        )
    });
    let net = entry.is_none().then_some(net_id).flatten().map(|id| {
        SourceIdentity::new(
            SourceNamespace::AthleticNet {
                kind: "athlete".into(),
            },
            id.to_string(),
        )
    });
    let live = entry.is_none().then_some(live_id).flatten().map(|id| {
        SourceIdentity::new(
            SourceNamespace::AthleticNet {
                kind: "live".into(),
            },
            id.to_string(),
        )
    });
    [association, net, live]
}
