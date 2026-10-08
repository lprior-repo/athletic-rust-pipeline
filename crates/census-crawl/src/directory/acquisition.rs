use crate::net::FetchOutcome;
use crate::{AdapterReport, CollectionDisposition, CrawlError, CrawlResult};
use std::fmt::Write;

pub(crate) const MAX_INPUTS: usize = 65_536;
pub(crate) const MAX_BODY: usize = 8 * 1024 * 1024;

pub(crate) fn bounded<T>(values: &mut Vec<T>, value: T) -> CrawlResult<()> {
    if values.len() >= MAX_INPUTS {
        return Err(resource(
            "source frontier",
            values.len().saturating_add(1),
            MAX_INPUTS,
        ));
    }
    values
        .try_reserve(1)
        .map_err(|_| resource("source allocation", 1, MAX_INPUTS))?;
    values.push(value);
    Ok(())
}

pub(crate) fn owe(report: &mut AdapterReport, locator: impl Into<String>) -> CrawlResult<()> {
    let locator = locator.into();
    if !report.unfinished.contains(&locator) {
        bounded(&mut report.unfinished, locator)?;
    }
    report.disposition = CollectionDisposition::Partial;
    Ok(())
}

pub(crate) fn fail(
    report: &mut AdapterReport,
    locator: &str,
    error: impl std::fmt::Display,
) -> CrawlResult<()> {
    report.errors = report.errors.saturating_add(1);
    owe(report, locator)?;
    if report.errors <= 5 {
        report
            .notes
            .try_reserve(1)
            .map_err(|_| resource("source error detail", 1, 5))?;
        report.note(detail(locator, error)?);
    }
    Ok(())
}

pub(crate) fn detail(locator: &str, error: impl std::fmt::Display) -> CrawlResult<String> {
    let mut text = Detail {
        value: String::new(),
        chars: 0,
    };
    text.value
        .try_reserve(16_384)
        .map_err(|_| resource("source error detail", 16_384, 16_384))?;
    write!(&mut text, "{locator}: {error}").map_err(|_| CrawlError::Invariant {
        detail: "bounded error rendering".into(),
    })?;
    Ok(text.value)
}

struct Detail {
    value: String,
    chars: usize,
}

impl std::fmt::Write for Detail {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        value
            .chars()
            .take(
                4096usize
                    .checked_sub(self.chars)
                    .map_or(0, |remaining| remaining),
            )
            .for_each(|character| {
                self.value.push(character);
                self.chars = self.chars.saturating_add(1);
            });
        Ok(())
    }
}

pub(crate) fn text(capture: &FetchOutcome) -> CrawlResult<&str> {
    if capture.status != 200 {
        return Err(crate::net::FetchError::Http {
            url: capture.url.clone(),
            status: capture.status,
        }
        .into());
    }
    if capture.body.len() > MAX_BODY {
        return Err(resource("source body", capture.body.len(), MAX_BODY));
    }
    let body = std::str::from_utf8(&capture.body).map_err(|error| CrawlError::Schema {
        url: capture.url.clone(),
        detail: error.to_string(),
    })?;
    let nodes = body
        .bytes()
        .filter(|byte| matches!(byte, b'<' | b'{' | b'['))
        .take(MAX_INPUTS.saturating_add(1))
        .count();
    if nodes > MAX_INPUTS {
        return Err(resource("source nodes", nodes, MAX_INPUTS));
    }
    Ok(body)
}

pub(crate) fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}

pub(crate) fn persist<T: serde::Serialize>(
    ctx: &crate::AdapterContext<'_>,
    origin: (&str, &str),
    table: census_store::Table,
    rows: &[T],
) -> CrawlResult<usize> {
    let locator = census_domain::model::serialized_digest(
        &origin.1.split_once('#').map_or(origin.1, |(base, _)| base),
    )
    .map_err(|source| CrawlError::Canonical {
        table: table.file().into(),
        source,
    })?;
    let phase = format!("northern_projection_v3:{}:{locator}", origin.0);
    rows.iter().try_fold(0usize, |written, row| {
        let admitted = ctx.append_row_once(&phase, table, row)?;
        written
            .checked_add(usize::from(admitted))
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "source effect count".into(),
            })
    })
}

pub(crate) fn publish<T: serde::Serialize>(
    ctx: &crate::AdapterContext<'_>,
    origin: (&str, &str),
    table: census_store::Table,
    rows: &[T],
    report: &mut AdapterReport,
) -> CrawlResult<usize> {
    rows.iter().try_fold(0usize, |written, row| {
        match persist(ctx, origin, table, std::slice::from_ref(row)) {
            Ok(admitted) => written
                .checked_add(admitted)
                .ok_or_else(|| resource("source count", written, usize::MAX)),
            Err(CrawlError::Store(error)) => Err(CrawlError::Store(error)),
            Err(error) => {
                fail(report, origin.1, error)?;
                Ok(written)
            }
        }
    })
}

pub(crate) fn publish_school(
    ctx: &crate::AdapterContext<'_>,
    origin: (&str, &str),
    subject: (
        &census_domain::model::SourceNamespace,
        &census_domain::model::CanonicalSchool,
        &str,
    ),
    report: &mut AdapterReport,
) -> CrawlResult<usize> {
    let written = publish(
        ctx,
        origin,
        census_store::Table::Schools,
        std::slice::from_ref(subject.1),
        report,
    )?;
    let observation =
        census_domain::model::SourceSchoolObservation::of_school(subject.0, subject.1, subject.2)
            .map(census_domain::model::SourceObservation::School);
    publish(
        ctx,
        origin,
        census_store::Table::SourceObservations,
        observation.as_slice(),
        report,
    )?;
    Ok(written)
}
