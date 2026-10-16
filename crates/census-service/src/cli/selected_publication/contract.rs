use anyhow::{Context, Result};
use census_domain::model::performance_comparison::SurfaceClass;
use census_domain::model::{Gender, QueryProfileV1, Sport};
use census_domain::profile_execution::validate_selected_qualification_profile_v3;
use census_report::{export::ExportDataset, report::Scope, workbook};
use std::path::Path;

pub(super) fn options(profile: &QueryProfileV1, out: &Path) -> Result<workbook::Options> {
    let checked = validate_selected_qualification_profile_v3(profile)?;
    let input = checked.profile().input();
    let [cohort] = input.graduation_years.as_slice() else {
        anyhow::bail!("unsupported qualification profile: exactly one graduation year required");
    };
    Ok(workbook::Options {
        grad_year: Some(cohort.get()),
        out: Some(out.to_path_buf()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: input.jurisdictions.clone(),
        school_year: input.contact_school_year,
    })
}

pub(super) fn unsupported_input(dataset: &ExportDataset, profile: &QueryProfileV1) -> Result<()> {
    let input = profile.input();
    for athlete in &dataset.athletes {
        category(athlete.gender, &input.competition_categories)?;
        sports(&athlete.sports, &input.sports)?;
    }
    for team in dataset.teams.values() {
        category(team.gender, &input.competition_categories)?;
        sports(&[team.sport], &input.sports)?;
    }
    for meet in &dataset.meets {
        anyhow::ensure!(
            SurfaceClass::resolve(meet) != SurfaceClass::Unresolved,
            "unsupported qualification input: ambiguous retained meet sport"
        );
        sports(&meet.sports, &input.sports)?;
    }
    let meets: std::collections::BTreeMap<_, _> = dataset
        .meets
        .iter()
        .map(|meet| (meet.id.as_str(), meet))
        .collect();
    let events: std::collections::BTreeMap<_, _> = dataset
        .events
        .iter()
        .map(|event| (event.id.as_str(), event))
        .collect();
    for event in &dataset.events {
        category(event.gender, &input.competition_categories)?;
        let meet = meets
            .get(event.meet.as_str())
            .context("unsupported qualification input: event has no retained meet")?;
        sports(&meet.sports, &input.sports)?;
    }
    for performance in &dataset.performances {
        anyhow::ensure!(
            events
                .get(performance.event.as_str())
                .is_some_and(|event| event.meet == performance.meet),
            "unsupported qualification input: performance has no bound retained event"
        );
    }
    anyhow::ensure!(
        dataset.relays.is_empty(),
        "unsupported qualification input: relay profile selection is unavailable"
    );
    Ok(())
}

fn category(category: Gender, admitted: &[Gender]) -> Result<()> {
    anyhow::ensure!(
        category != Gender::Unknown && admitted.contains(&category),
        "unsupported qualification input: unknown or outside-profile competition category"
    );
    Ok(())
}

fn sports(sports: &[Sport], admitted: &[Sport]) -> Result<()> {
    anyhow::ensure!(
        !sports.is_empty()
            && sports
                .iter()
                .all(|sport| *sport != Sport::Unknown && admitted.contains(sport)),
        "unsupported qualification input: unknown or outside-profile sport"
    );
    Ok(())
}
