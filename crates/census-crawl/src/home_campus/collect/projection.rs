use crate::directory::acquisition::{fail, publish, publish_school};
use crate::home_campus::parse::{
    decode_coach, decode_faculty, parse_sport_and_gender, SchoolProfile,
};
use crate::home_campus::{school_entities, ProfileFacts, Section, SOURCE_ID};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::SourceNamespace;
use census_store::Table;
use serde_json::Value;

struct Projection<'a> {
    ctx: &'a AdapterContext<'a>,
    section: &'a Section,
    source: (&'a str, &'a str),
    profile: &'a SchoolProfile,
    report: &'a mut AdapterReport,
}

pub(super) fn project(
    ctx: &AdapterContext<'_>,
    section: &Section,
    source: (&str, &str),
    parsed: (&SchoolProfile, &Value),
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let mut projection = Projection {
        ctx,
        section,
        source,
        profile: parsed.0,
        report,
    };
    let extract = school_entities(&projection.facts(), &[], &[]);
    let written = publish_school(
        ctx,
        (SOURCE_ID, source.0),
        (
            &SourceNamespace::association_school(section.association),
            &extract.school,
            source.1,
        ),
        projection.report,
    )?;
    projection.report.rows =
        projection
            .report
            .rows
            .saturating_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
                detail: "school count".into(),
            })?);
    projection.coaches(parsed.1)?;
    projection.faculties(parsed.1)
}

impl Projection<'_> {
    fn facts(&self) -> ProfileFacts<'_> {
        ProfileFacts {
            state: self.section.state,
            association: self.section.association,
            name: &self.profile.name,
            city: &self.profile.city,
            address: &self.profile.address,
            zip: &self.profile.zip,
            league: &self.profile.league,
            phone: &self.profile.phone,
            url: self.source.0,
            observed_on: self.source.1,
        }
    }

    fn coaches(&mut self, parsed: &Value) -> CrawlResult<()> {
        let Some(rows) = parsed.get("coaches").and_then(Value::as_array) else {
            return fail(
                self.report,
                &format!("{}#coaches", self.source.0),
                "missing coach array",
            );
        };
        rows.iter().enumerate().try_for_each(|(ordinal, value)| {
            let locator = format!("{}#coaches={ordinal}", self.source.0);
            let row = match decode_coach(value.clone()) {
                Ok(row) => row,
                Err(error) => return fail(self.report, &locator, error),
            };
            if row.sport.trim().is_empty()
                || (parse_sport_and_gender(&row.sport).is_some()
                    && (row.name.trim().is_empty() || row.role.trim().is_empty()))
            {
                return fail(self.report, &locator, "missing sport appointment fields");
            }
            let extract = school_entities(&self.facts(), std::slice::from_ref(&row), &[]);
            self.report.with_email = self.report.with_email.saturating_add(
                u64::try_from(
                    extract
                        .coaches
                        .iter()
                        .filter(|coach| coach.has_published_email())
                        .count(),
                )
                .map_or(u64::MAX, |value| value),
            );
            publish(
                self.ctx,
                (SOURCE_ID, &locator),
                Table::Coaches,
                &extract.coaches,
                self.report,
            )
            .map(|_| ())
        })
    }

    fn faculties(&mut self, parsed: &Value) -> CrawlResult<()> {
        let Some(rows) = parsed.get("athleticFaculties").and_then(Value::as_array) else {
            return fail(
                self.report,
                &format!("{}#athleticFaculties", self.source.0),
                "missing faculty array",
            );
        };
        rows.iter().enumerate().try_for_each(|(ordinal, value)| {
            let locator = format!("{}#athleticFaculties={ordinal}", self.source.0);
            let row = match decode_faculty(value.clone()) {
                Ok(row) if !row.name.trim().is_empty() && !row.role.trim().is_empty() => row,
                Ok(_) => return fail(self.report, &locator, "missing faculty appointment fields"),
                Err(error) => return fail(self.report, &locator, error),
            };
            let extract = school_entities(&self.facts(), &[], std::slice::from_ref(&row));
            self.report.with_email = self.report.with_email.saturating_add(
                u64::try_from(
                    extract
                        .coaches
                        .iter()
                        .filter(|coach| coach.has_published_email())
                        .count(),
                )
                .map_or(u64::MAX, |value| value),
            );
            publish(
                self.ctx,
                (SOURCE_ID, &locator),
                Table::Coaches,
                &extract.coaches,
                self.report,
            )
            .map(|_| ())
        })
    }
}
