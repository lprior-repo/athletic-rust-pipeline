use super::corpus::{Event, EVENTS};
use anyhow::{ensure, Context, Result};
use census_domain::model::{CanonicalAthlete, CanonicalPerformance, Mark, SourceNamespace};
use census_report::export::ExportDataset;
use census_report::report::{Derivation, Scope};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
pub struct Expected(
    pub String,
    pub String,
    pub u64,
    pub String,
    pub Option<i64>,
    pub Option<u16>,
);

#[derive(Deserialize)]
pub struct Oracle {
    xc: Vec<Expected>,
    jump: Vec<Expected>,
    track: Vec<Expected>,
}

impl Oracle {
    pub fn load() -> Result<Self> {
        let oracle: Self = serde_json::from_str(include_str!("oracle.json"))?;
        ensure!(
            oracle.xc.len() == 45 && oracle.jump.len() == 8 && oracle.track.len() == 7,
            "frozen oracle census changed"
        );
        let names: BTreeSet<_> = oracle.all().map(|row| row.0.as_str()).collect();
        ensure!(names.len() == 60, "frozen oracle has duplicate identities");
        Ok(oracle)
    }

    pub fn all(&self) -> impl Iterator<Item = &Expected> {
        self.xc.iter().chain(&self.jump).chain(&self.track)
    }

    pub fn event(&self, expected: &Expected) -> Result<&Event> {
        let id = if self.xc.iter().any(|row| row.0 == expected.0) {
            2_150_205
        } else if self.jump.iter().any(|row| row.0 == expected.0) {
            2_254_280
        } else {
            2_254_285
        };
        EVENTS
            .iter()
            .find(|event| event.event == id)
            .context("missing frozen event context")
    }

    pub fn verify(&self, dataset: &ExportDataset) -> Result<()> {
        ensure!(
            dataset.athletes.len() == 185 && dataset.performances.len() == 185,
            "canonical row loss or duplication"
        );
        let derived = Derivation::of(dataset, Scope::AllSources, Some(2027));
        ensure!(
            derived.athletes().len() == 60 && derived.performances().len() == 60,
            "cohort census mismatch"
        );
        self.all().try_for_each(|expected| {
            verify_athlete(dataset, &derived, expected, self.event(expected)?)
        })?;
        ensure!(
            dataset
                .coaches
                .iter()
                .any(|coach| coach.name == "Alex Larson"
                    && coach.professional_email.as_deref() == Some("alarson@abbotsford.k12.wi.us")),
            "source-published WIAA athletic director observation lost"
        );
        Ok(())
    }
}

fn verify_athlete(
    dataset: &ExportDataset,
    derived: &Derivation<'_>,
    expected: &Expected,
    event: &Event,
) -> Result<()> {
    let athlete = derived
        .athletes()
        .iter()
        .find(|athlete| athlete.canonical_name == expected.0)
        .context("frozen cohort member missing")?;
    let school = dataset
        .schools
        .get(&athlete.school)
        .context("canonical school join missing")?;
    ensure!(
        school.name == expected.1 && school.state == Some(event.state),
        "structural team/state mismatch for {}",
        expected.0
    );
    ensure!(
        athlete
            .identity_in(&SourceNamespace::athletic_net("athlete"))
            .is_some_and(|source| source.id == expected.2.to_string()),
        "provider identity lost for {}",
        expected.0
    );
    let mut matches = derived
        .performances()
        .iter()
        .copied()
        .filter(|row| row.athlete == athlete.id);
    let performance = matches.next().context("canonical performance absent")?;
    ensure!(
        matches.next().is_none(),
        "duplicate performance for {}",
        expected.0
    );
    ensure!(
        performance.source_athlete.as_ref()
            == athlete.identity_in(&SourceNamespace::athletic_net("athlete")),
        "performance provider owner differs from athlete"
    );
    verify_mark(performance, expected)?;
    verify_provenance(athlete, performance, event)
}

fn verify_mark(performance: &CanonicalPerformance, expected: &Expected) -> Result<()> {
    let value = match &performance.mark {
        Mark::TimeSeconds(time) => Some(time.value()),
        Mark::DistanceMetres(metres) | Mark::FieldImperial { metres, .. } => {
            Some(i64::from(metres.value()))
        }
        Mark::Raw(_) => None,
        Mark::Points(_) => anyhow::bail!("unexpected points mark"),
    };
    ensure!(
        value == expected.4 && performance.place == expected.5,
        "exact value or place mismatch for {}",
        expected.0
    );
    ensure!(
        census_report::bests::mark_text(&performance.mark) == expected.3,
        "published mark precision changed for {}",
        expected.0
    );
    if let Mark::TimeSeconds(time) = &performance.mark {
        let precision = if expected.4.is_some_and(|value| value > 1_000_000_000_000) {
            1
        } else {
            2
        };
        ensure!(
            time.precision() == precision,
            "published precision mismatch for {}",
            expected.0
        );
    }
    Ok(())
}

fn verify_provenance(
    athlete: &CanonicalAthlete,
    performance: &CanonicalPerformance,
    event: &Event,
) -> Result<()> {
    let url = census_crawl::athleticlive::event_doc_url(event.event);
    ensure!(performance.date == event.date, "performance date mismatch");
    ensure!(
        performance
            .evidence
            .iter()
            .any(|evidence| evidence.source.id == "athleticlive_results"
                && evidence.source.url.as_deref() == Some(url.as_str())
                && evidence.observed_on == event.acquired_at),
        "performance route/capture provenance lost"
    );
    ensure!(
        athlete
            .observed_grades
            .iter()
            .any(|grade| grade.grade.get() == 11 && grade.school_year.get() == 2025),
        "published junior grade provenance lost"
    );
    Ok(())
}
