use super::{measured_pair, TestResult};
use crate::athlete_cluster_findings::Observed;
use crate::athlete_verdict::HardContradiction;
use census_domain::model::{AppliedIdentityKind, AthleteId, AthleteIdentityIndex, Gender};

#[test]
fn o8rm_review_gender_compatibility_matches_domain_for_every_order() -> TestResult {
    let genders = [Gender::Boys, Gender::Girls, Gender::Mixed, Gender::Unknown];
    for first in genders {
        for second in genders {
            for third in genders {
                let measured = measured_pair()?;
                let mut rows = measured.athletes;
                let mut additional = rows.first().ok_or("first captured athlete")?.clone();
                additional.id = AthleteId::mint("ath_subject", &["gender-parity-third"]);
                rows.push(additional);
                let mut index = AthleteIdentityIndex::default();
                for (row, gender) in rows.iter_mut().zip([first, second, third]) {
                    row.gender = gender;
                    index.observe(row)?;
                }
                let members = rows.iter().map(|row| row.id.cast()).collect::<Vec<_>>();
                let supports = index.supports_identity(AppliedIdentityKind::SamePerson, &members);
                let observed = Observed {
                    schools: measured
                        .schools
                        .into_iter()
                        .map(|(id, school)| (id, school.name))
                        .collect(),
                    rows,
                };
                let findings = observed.findings();
                check!(eq; findings.spans.len(), 1);
                let span = findings.spans.first().ok_or("shared-primary span")?;
                check!(eq; span.agrees(), supports, "gender order: {first:?}, {second:?}, {third:?}");
                check!(eq; span.hard_contradiction(), if supports { None } else { Some(HardContradiction::GenderDiffers) });
            }
        }
    }
    Ok(())
}
