use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use census_domain::model::{CanonicalAthlete, GradYear, ObservedGrade};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CohortEvidence<'a> {
    supported: BTreeSet<GradYear>,
    unsupported: Vec<&'a ObservedGrade>,
}

impl<'a> CohortEvidence<'a> {
    pub(crate) fn of(athlete: &'a CanonicalAthlete) -> Self {
        let mut evidence =
            athlete
                .observed_grades
                .iter()
                .fold(Self::default(), |mut evidence, observation| {
                    match observation.grad_year() {
                        Some(year) => {
                            evidence.supported.insert(year);
                        }
                        None => evidence.unsupported.push(observation),
                    }
                    evidence
                });
        evidence.supported.extend(
            athlete
                .published_graduations
                .iter()
                .map(|observation| observation.grad_year),
        );
        evidence
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.supported.is_empty() && self.unsupported.is_empty()
    }

    pub(crate) fn conflicts_with(&self, other: &Self) -> bool {
        !self.unsupported.is_empty()
            || !other.unsupported.is_empty()
            || (!self.supported.is_empty()
                && !other.supported.is_empty()
                && self.supported != other.supported)
    }
}

impl Display for CohortEvidence<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "supported [")?;
        for (index, year) in self.supported.iter().enumerate() {
            if index != 0 {
                write!(formatter, "/")?;
            }
            write!(formatter, "{year}")?;
        }
        write!(formatter, "]")?;
        for observation in &self.unsupported {
            write!(
                formatter,
                "; unsupported grade {} in school year {} from {} ({})",
                observation.grade,
                observation.school_year.get(),
                observation.source.id,
                observation
                    .source
                    .url
                    .as_deref()
                    .map_or("no source URL", |url| url),
            )?;
        }
        Ok(())
    }
}
