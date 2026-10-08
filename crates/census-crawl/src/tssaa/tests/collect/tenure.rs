use super::{content_digest, page_options, FixtureRun, CAPTURED, RETAINED};
use census_domain::model::{assess_coach_tenure, CanonicalCoach, CoachTenure, SchoolYear};

const STATEMENT: &str = "Staff information for the 2026-2027 school year is displayed as it is entered/verified by Page High School administration.";

fn year(start: i16) -> anyhow::Result<SchoolYear> {
    SchoolYear::new(start).ok_or_else(|| anyhow::anyhow!("invalid test academic year"))
}

fn assessment(coach: &CanonicalCoach, start: i16) -> anyhow::Result<CoachTenure> {
    Ok(assess_coach_tenure(&coach.tenure_evidence, year(start)?)?)
}

#[test]
fn published_year_qualifies_ad_and_retired_educator_coaching_without_becoming_timeless(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(RETAINED, Some(RETAINED))?;
            let report = run.run(&page_options(), None).await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            let coaches = run.coaches()?;
            for name in ["Benji Gray", "Ron Brock"] {
                let coach = coaches
                    .iter()
                    .find(|coach| coach.name == name)
                    .ok_or_else(|| anyhow::anyhow!("published appointment {name}"))?;
                check!(eq;
                    assessment(coach, 2026)?,
                    CoachTenure::Current {
                        school_year: year(2026)?
                    }
                );
                for outside_year in [2025, 2027] {
                    check!(eq; assessment(coach, outside_year)?, CoachTenure::Unknown);
                }
                let evidence = coach
                    .tenure_evidence
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("published tenure"))?;
                check!(eq; evidence.statement, STATEMENT);
                check!(eq;
                    evidence.source.url.as_deref(),
                    Some("https://portal.tssaa.org/common/directory/?id=157")
                );
                check!(eq; evidence.source_sha256, content_digest(RETAINED));
                check!(eq; evidence.retrieved_at, CAPTURED);
            }
            Ok(())
        })
}

#[test]
fn absent_statement_preserves_unknown_appointments_despite_dated_admin_header(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let text = std::str::from_utf8(RETAINED)?.replacen(STATEMENT, "", 1);
            let run = FixtureRun::new(RETAINED, Some(text.as_bytes()))?;
            let report = run.run(&page_options(), None).await?;
            check!(eq; report.with_email, 11);
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(
                report
                    .unfinished
                    .contains(&super::super::super::DIRECTORY_URL.to_string())
                    || report.unfinished.iter().any(|url| url.contains("?id=157"))
            );
            check!(eq; run.coaches()?.len(), 11);
            for coach in run.coaches()? {
                check!(eq; coach.tenure_evidence, vec![]);
                check!(eq; assessment(&coach, 2026)?, CoachTenure::Unknown);
            }
            Ok(())
        })
}

#[test]
fn malformed_foreign_and_conflicting_context_retain_contacts_without_completion(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let text = std::str::from_utf8(RETAINED)?;
            let variants = [
                (
                    STATEMENT.to_string(),
                    STATEMENT.replace("2026-2027", "2026-2028"),
                ),
                (
                    STATEMENT.to_string(),
                    STATEMENT.replace("2026-2027", "2026-27"),
                ),
                (
                    STATEMENT.to_string(),
                    STATEMENT.replace("2026-2027", "20x6-2027"),
                ),
                (
                    STATEMENT.to_string(),
                    STATEMENT.replace("2026-2027", "1899-1900"),
                ),
                (
                    STATEMENT.to_string(),
                    STATEMENT.replace("2026-2027", "2101-2102"),
                ),
                (
                    STATEMENT.to_string(),
                    STATEMENT.replace("Page High School", "Alcoa High School"),
                ),
                (
                    "2026-2027 School Administration".to_string(),
                    "2025-2026 School Administration".to_string(),
                ),
            ];
            for (original, replacement) in variants {
                let mutated = text.replacen(&original, &replacement, 1);
                let run = FixtureRun::new(RETAINED, Some(mutated.as_bytes()))?;
                let report = run.run(&page_options(), None).await?;
                check!(eq; report.with_email, 11, "{replacement}");
                check!(eq; report.disposition, crate::CollectionDisposition::Partial, "{replacement}");
                check!(report.unfinished.iter().any(|url| url.contains("?id=157")));
                check!(eq; run.coaches()?.len(), 11);
                for coach in run.coaches()? {
                    check!(eq;
                        assessment(&coach, 2026)?,
                        CoachTenure::Unknown,
                        "{replacement}"
                    );
                }
                check!(run.store.journal_payloads(super::super::super::collect::JOURNAL)?
                    .iter().all(|payload| payload["complete"] != true));
            }
            Ok(())
        })
}

#[test]
fn older_published_year_is_retained_without_becoming_current_or_former_in_run_year(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let text = std::str::from_utf8(RETAINED)?.replace("2026-2027", "2025-2026");
            let run = FixtureRun::new(RETAINED, Some(text.as_bytes()))?;
            let report = run.run(&page_options(), None).await?;
            check!(eq; report.with_email, 11);
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(report.unfinished.iter().any(|url| url.contains("?id=157")));
            check!(eq; run.coaches()?.len(), 11);
            for coach in run.coaches()? {
                check!(eq;
                    assessment(&coach, 2025)?,
                    CoachTenure::Current {
                        school_year: year(2025)?
                    }
                );
                check!(eq; assessment(&coach, 2026)?, CoachTenure::Unknown);
            }
            Ok(())
        })
}
