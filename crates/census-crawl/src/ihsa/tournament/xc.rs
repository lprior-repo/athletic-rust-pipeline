use super::journal::Journal;
use super::map::{joined_name, school_year_of_term, AthleteRow, Mapper, XcList};
use super::parse::parse_grade;
use super::report::gender_word;
use super::requests::{self, qualifiers_url};
use super::wire::{QualifierAthlete, QualifiersEnvelope};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{Gender, SchoolId, SchoolYear, Sport};

const TOURNAMENTS: [(u32, &str, Gender); 6] = [
    (688, "1A", Gender::Boys),
    (689, "2A", Gender::Boys),
    (690, "3A", Gender::Boys),
    (691, "1A", Gender::Girls),
    (692, "2A", Gender::Girls),
    (693, "3A", Gender::Girls),
];

struct Tournament {
    id: u32,
    class: &'static str,
    gender: Gender,
}

pub(super) struct Route<'a, 'b> {
    pub(super) ctx: &'a AdapterContext<'a>,
    pub(super) report: &'b mut AdapterReport,
    pub(super) mapper: &'b mut Mapper<'a>,
    pub(super) journal: &'b mut Journal,
    pub(super) resumed: &'b mut usize,
}

impl Route<'_, '_> {
    pub(super) async fn walk(&mut self) -> CrawlResult<()> {
        let Some(term) = requests::newest_term(self.ctx, self.report).await else {
            return Ok(());
        };
        let Some(school_year) = school_year_of_term(&term) else {
            self.report.errors = self.report.errors.saturating_add(1);
            self.report
                .unfinished
                .push(format!("{}/v1/terms", crate::ihsa::IHSA_API));
            self.report.note(format!(
                "terms: {term:?} is not a school-year label; the qualifier lists were not read"
            ));
            return Ok(());
        };
        for (id, class, gender) in TOURNAMENTS {
            self.list(&term, school_year, Tournament { id, class, gender })
                .await?;
        }
        Ok(())
    }

    async fn list(
        &mut self,
        term: &str,
        school_year: SchoolYear,
        tournament: Tournament,
    ) -> CrawlResult<()> {
        let key = format!("{term}:{}", tournament.id);
        if self.journal.qualifiers.contains(&key) {
            *self.resumed = self.resumed.saturating_add(1);
            return Ok(());
        }
        let url = qualifiers_url(term, tournament.id);
        let Some(capture) = requests::qualifiers(self.ctx, self.report, &url).await else {
            return Ok(());
        };
        let Some(envelope) = self.decode_qualifiers(&capture, &tournament) else {
            return Ok(());
        };
        let bound = self.mapper.bind_capture(capture);
        if requests::decoded(self.report, &url, "qualifier capture lineage", bound).is_none() {
            return Ok(());
        }
        let rows = XcList {
            tournament_id: envelope.tournament_id.as_str(),
            gender: tournament.gender,
            school_year,
        };
        self.mapper.absorb_qualifiers(&envelope, &rows, &url);
        self.journal.list(&key, &url, &envelope);
        Ok(())
    }

    fn decode_qualifiers(
        &mut self,
        capture: &crate::net::FetchOutcome,
        tournament: &Tournament,
    ) -> Option<QualifiersEnvelope> {
        if let Ok(error) = capture.json::<super::wire::ErrorEnvelope>() {
            self.report.note(format!(
                "tournamentId {} ({}, {}): {}",
                tournament.id,
                tournament.class,
                gender_word(tournament.gender),
                error.error,
            ));
            return None;
        }
        let parsed = capture.json().map_err(crate::CrawlError::from);
        requests::decoded(
            self.report,
            &capture.url,
            "the cross-country qualifiers",
            parsed,
        )
    }
}

impl<'a> Mapper<'a> {
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
            for (row_index, athlete) in qualifier.athletes.iter().enumerate() {
                self.qualifier(&school, athlete, list, url, row_index);
            }
        }
    }

    fn qualifier(
        &mut self,
        school: &SchoolId,
        row: &QualifierAthlete,
        list: &XcList<'_>,
        url: &str,
        row_index: usize,
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
                source_key: format!("{url}:school:{school}:row:{row_index}"),
            },
            evidence,
        );
        if stored.is_some() {
            self.stats.qualifier_graded = self.stats.qualifier_graded.saturating_add(1);
        }
    }
}
