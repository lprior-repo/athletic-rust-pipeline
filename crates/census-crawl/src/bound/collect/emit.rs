use super::super::map::{map_role, school_entities, ProfileFacts, SchoolExtract};
use super::super::parse::{parse, CoachRow};
use super::super::{school_namespace, HOST};
use super::{bound_fetch_options_permissive, index::slugify};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{Gender, SchoolYear, Sport};
use census_domain::UsJurisdiction;
use census_store::Table;

const SEASON: &str = "2026-27";

pub(in crate::bound) struct SchoolJob<'a> {
    pub(super) name: &'a str,
    pub(super) slug: &'a str,
    pub(super) state_code: &'a str,
    pub(super) state: UsJurisdiction,
}

struct StaffPage {
    url: String,
    sha256: String,
    fetched_at: String,
    program: String,
    sport: Sport,
    gender: Gender,
    coaches: Vec<CoachRow>,
}

pub(in crate::bound) async fn process_school(
    ctx: &AdapterContext<'_>,
    job: &SchoolJob<'_>,
    association: &str,
    entries: &[(&str, Sport, Gender)],
) -> CrawlResult<Option<SchoolExtract>> {
    let pages = fetch_pages(ctx, job, association, entries).await;
    let key = format!("{}/{}", job.state_code, job.slug);
    merge_pages(job, &key, &pages, ctx.school_year)
}

async fn fetch_pages(
    ctx: &AdapterContext<'_>,
    job: &SchoolJob<'_>,
    association: &str,
    entries: &[(&str, Sport, Gender)],
) -> Vec<StaffPage> {
    let mut pages = Vec::new();

    for &(sport_slug, sport, gender) in entries {
        let url = format!(
            "{HOST}/{}/{association}/{sport_slug}/{SEASON}/{}/v/staff",
            job.state_code, job.slug
        );
        let fetch_opts = bound_fetch_options_permissive(ctx);

        let outcome = match ctx.fetcher.get(&url, &fetch_opts).await {
            Ok(o) => o,
            Err(_) => continue,
        };

        if outcome.status != 200 {
            continue;
        }

        let html = String::from_utf8_lossy(&outcome.body);
        let parsed = parse(&html);

        if !parsed.page.title_school.is_empty()
            && !title_matches_slug(job.slug, &parsed.page.title_school)
        {
            continue;
        }

        pages.push(StaffPage {
            url: outcome.url,
            sha256: outcome.content_digest,
            fetched_at: outcome.fetched_at,
            program: format!("{} {SEASON}", parsed.page.title_sport),
            sport,
            gender,
            coaches: parsed.coaches,
        });
    }

    pages
}

fn merge_pages(
    job: &SchoolJob<'_>,
    key: &str,
    pages: &[StaffPage],
    school_year: SchoolYear,
) -> CrawlResult<Option<SchoolExtract>> {
    let mut coaches = Vec::new();
    let mut school_extract: Option<SchoolExtract> = None;

    for page in pages {
        let facts = ProfileFacts {
            name: job.name,
            key,
            url: &page.url,
            observed_on: &page.fetched_at,
            state: job.state,
        };
        let mut extract = school_entities(&facts, &page.coaches, page.sport, page.gender);
        for (coach, row) in extract.coaches.iter_mut().zip(
            page.coaches
                .iter()
                .filter(|row| map_role(&row.role).is_some()),
        ) {
            crate::coach_directories::record_appointment(
                coach,
                Some(crate::coach_directories::PublishedListing {
                    role: &row.role,
                    program: &page.program,
                    source: super::super::SOURCE_ID,
                    school_year,
                }),
                crate::coach_directories::StaffCapture {
                    url: &page.url,
                    observed_on: &page.fetched_at,
                    sha256: &page.sha256,
                },
            )?;
        }
        coaches.append(&mut extract.coaches);
        if school_extract.is_none() {
            school_extract = Some(extract);
        }
    }

    Ok(school_extract.map(|extract| SchoolExtract { coaches, ..extract }))
}

pub(in crate::bound) fn title_matches_slug(slug: &str, title_school: &str) -> bool {
    let expected = slugify(slug);
    let title = slugify(title_school);

    if expected.is_empty() || title.is_empty() {
        return false;
    }

    title.starts_with(&expected)
}

pub(in crate::bound) fn emit_school(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    coach_tally: &mut usize,
) -> CrawlResult<()> {
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    batch.append_many(
        Table::SourceObservations,
        std::slice::from_ref(
            &ctx.school_observation(&school_namespace(), &extract.school)
                .ok_or(CrawlError::Invariant {
                    detail: "school observation failed".to_string(),
                })?,
        ),
    )?;

    for coach in &extract.coaches {
        *coach_tally = coach_tally.saturating_add(1);
        batch.append_many(Table::Coaches, std::slice::from_ref(coach))?;
    }

    batch.commit()?;
    Ok(())
}
