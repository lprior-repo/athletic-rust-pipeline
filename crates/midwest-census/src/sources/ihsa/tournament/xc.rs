//! Cross-country: the archive's state-finalist lists, and the rows they mint.
//!
//! The lists are entry lists, not results: they publish no mark, no per-athlete place and no date, so
//! a run mints the athletes and the teams that qualified as teams, and stores no performance. The one
//! field the list leaves implicit is gender — the tournament id it answers for is a single class's
//! boys or girls list — so that gender is recorded as a derived observation naming the id.
//!
//! The route reads `/v1/terms` once and then the six tournament ids of the newest completed term. A
//! term the archive does not hold answers with `{"error": "Archive not available for term ..."}`;
//! that body is a note, never an error, because the request did what it was asked.

use super::journal::Journal;
use super::map::{joined_name, school_year_of_term, AthleteRow, Mapper, XcList};
use super::parse::{parse_error, parse_grade, parse_qualifiers};
use super::report::gender_word;
use super::requests::{self, qualifiers_url};
use super::wire::{QualifierAthlete, QualifiersEnvelope};
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{Gender, SchoolId, SchoolYear, Sport};

/// The cross-country state-final tournaments the archive answers for: three classes, two genders.
///
/// Measured against `2025-26`: ids 688-690 are the boys 1A/2A/3A lists and 691-693 the girls, and the
/// id is also the only place the list's gender is stated.
const TOURNAMENTS: [(u32, &str, Gender); 6] = [
    (688, "1A", Gender::Boys),
    (689, "2A", Gender::Boys),
    (690, "3A", Gender::Boys),
    (691, "1A", Gender::Girls),
    (692, "2A", Gender::Girls),
    (693, "3A", Gender::Girls),
];

/// The handles this route shares with the walk it is a part of.
pub(super) struct Route<'a, 'b> {
    pub(super) ctx: &'a AdapterContext<'a>,
    pub(super) report: &'b mut AdapterReport,
    pub(super) mapper: &'b mut Mapper<'a>,
    pub(super) journal: &'b mut Journal,
    /// Bumped for every list the journal already held, so a resumed run says so.
    pub(super) resumed: &'b mut usize,
}

impl Route<'_, '_> {
    /// Walk the archive's cross-country state-finalist lists for the newest term it holds.
    pub(super) async fn walk(&mut self) -> CrawlResult<()> {
        let Some(term) = requests::newest_term(self.ctx, self.report).await else {
            return Ok(());
        };
        let Some(school_year) = school_year_of_term(&term) else {
            self.report.note(format!(
                "terms: {term:?} is not a school-year label; the qualifier lists were not read"
            ));
            return Ok(());
        };
        for (id, class, gender) in TOURNAMENTS {
            self.list(&term, school_year, id, class, gender).await?;
        }
        Ok(())
    }

    /// Read one tournament's entry list.
    async fn list(
        &mut self,
        term: &str,
        school_year: SchoolYear,
        id: u32,
        class: &str,
        gender: Gender,
    ) -> CrawlResult<()> {
        let key = format!("{term}:{id}");
        if self.journal.qualifiers.contains(&key) {
            *self.resumed = self.resumed.saturating_add(1);
            return Ok(());
        }
        let url = qualifiers_url(term, id);
        let Some(body) = requests::qualifiers(self.ctx, self.report, &url).await else {
            return Ok(());
        };
        if let Some(message) = parse_error(&body) {
            self.report.note(format!(
                "tournamentId {id} ({class}, {}): {message}",
                gender_word(gender)
            ));
            return Ok(());
        }
        let parsed = parse_qualifiers(&body);
        let Some(envelope) =
            requests::decoded(self.report, &url, "the cross-country qualifiers", parsed)
        else {
            return Ok(());
        };
        let rows = XcList {
            tournament_id: envelope.tournament_id.as_str(),
            gender,
            school_year,
        };
        self.mapper.absorb_qualifiers(&envelope, &rows, &url);
        self.journal.list(self.ctx, &key, &url, &envelope)
    }
}

impl<'a> Mapper<'a> {
    /// Mint the athletes one cross-country state-finalist list publishes.
    ///
    /// Team qualifiers come first, then the individuals who qualified without their team: both carry
    /// the school's own id, so both resolve onto the same canonical school.
    pub(super) fn absorb_qualifiers(
        &mut self,
        envelope: &QualifiersEnvelope,
        list: &XcList<'_>,
        url: &str,
    ) {
        let qualifiers = envelope
            .team_qualifiers
            .iter()
            .chain(envelope.individual_qualifiers.iter());
        for qualifier in qualifiers {
            let school = self.school(
                qualifier.ihsa_school_id.as_deref(),
                qualifier.school_name.as_deref(),
                url,
            );
            let Some(school) = school else {
                continue;
            };
            if qualifier.team_place.is_some() {
                self.team(
                    &school,
                    Sport::CrossCountry,
                    list.gender,
                    list.school_year,
                    None,
                    url,
                );
            }
            for athlete in &qualifier.athletes {
                self.qualifier(&school, athlete, list, url);
            }
        }
    }

    /// Mint one qualifying athlete of a cross-country list.
    ///
    /// The entry number is unique within the tournament that publishes it, so it is filed under the
    /// association's own namespace with the tournament id in front of it.
    fn qualifier(
        &mut self,
        school: &SchoolId,
        row: &QualifierAthlete,
        list: &XcList<'_>,
        url: &str,
    ) {
        self.stats.qualifier_rows = self.stats.qualifier_rows.saturating_add(1);
        let name = joined_name(row.first_name.as_deref(), row.last_name.as_deref());
        let grade = parse_grade(row.year_in_school.as_deref());
        if grade.is_none() {
            self.stats.qualifier_no_grade = self.stats.qualifier_no_grade.saturating_add(1);
        }
        let evidence = self.origin.derived(
            url,
            format!(
                "gender from the tournament id {}: the entry list itself publishes none",
                list.tournament_id
            ),
        );
        let stored = self.athlete(
            AthleteRow {
                name: name.as_deref(),
                grade,
                gender: list.gender,
                school,
                sport: Sport::CrossCountry,
                school_year: list.school_year,
                net_id: None,
                live_id: None,
                entry: row
                    .number
                    .map(|number| format!("{}:{number}", list.tournament_id)),
            },
            evidence,
        );
        if stored.is_some() {
            self.stats.qualifier_graded = self.stats.qualifier_graded.saturating_add(1);
        }
    }
}
