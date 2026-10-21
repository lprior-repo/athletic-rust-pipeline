use super::objects::{digest, domain, invalid, object_reference, Objects};
use crate::participation_evidence::result_ownership::{
    CheckedHtmlResultDocumentV1, CheckedHtmlResultRowV1, CheckedRetainedResultDocumentV1,
    CheckedRetainedResultRowV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{AttachmentDigestV1, AttachmentReplayGrantV1};
use census_domain::model::{Gender, ProfileDate, SchoolYear, Sport};
use census_domain::result_calendar_correspondence::{
    ResultCalendarRowClaimV1, ResultCalendarSourceParserV1,
};
use census_store::{
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootV1, RetainedCaptureSlotV1,
};

pub(super) fn parser(
    document: &CheckedHtmlResultDocumentV1<'_>,
    row: &CheckedHtmlResultRowV1<'_>,
) -> CrawlResult<ResultCalendarSourceParserV1> {
    match (document, row) {
        (CheckedHtmlResultDocumentV1::MileSplit(_), CheckedHtmlResultRowV1::MileSplit(_)) => {
            Ok(ResultCalendarSourceParserV1::MileSplitDcc)
        }
        (CheckedHtmlResultDocumentV1::Tfrrs(_), CheckedHtmlResultRowV1::Tfrrs(_)) => {
            Ok(ResultCalendarSourceParserV1::TfrrsRoncalli)
        }
        _ => Err(invalid("calendar statement and original source differ")),
    }
}

pub(super) fn scope(
    document: &CheckedHtmlResultDocumentV1<'_>,
    row: &CheckedHtmlResultRowV1<'_>,
) -> CrawlResult<Scope> {
    match (document, row) {
        (
            CheckedHtmlResultDocumentV1::MileSplit(document),
            CheckedHtmlResultRowV1::MileSplit(row),
        ) => milesplit_scope(document, row),
        (CheckedHtmlResultDocumentV1::Tfrrs(document), CheckedHtmlResultRowV1::Tfrrs(row)) => {
            let year = SchoolYear::from_date(row.published_date())
                .ok_or_else(|| invalid("calendar TFRRS school year unsupported"))?;
            let day = ProfileDate::parse(row.published_date())
                .map_err(|_| invalid("calendar TFRRS exact day unsupported"))?;
            let parser = ResultCalendarSourceParserV1::TfrrsRoncalli;
            if document.year() != row.year()
                || !parser.supports_scope(document.sport(), row.gender())
            {
                return Err(invalid("calendar TFRRS source scope differs"));
            }
            Ok(Scope {
                day,
                year,
                sport: document.sport(),
                category: row.gender(),
            })
        }
        _ => Err(invalid("calendar statement and original source differ")),
    }
}

pub(super) fn claim(
    root: &RetainedCaptureRootV1,
    document: &CheckedHtmlResultDocumentV1<'_>,
    row: &CheckedHtmlResultRowV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultCalendarRowClaimV1> {
    let parser = parser(document, row)?;
    let original = match document {
        CheckedHtmlResultDocumentV1::MileSplit(document) => document.original_document(),
        CheckedHtmlResultDocumentV1::Tfrrs(document) => document.original_document(),
    };
    let body = object_reference(original.as_bytes(), grant)?;
    if body.digest.as_str() != document.capture().body_sha256() {
        return Err(invalid("calendar original result body differs"));
    }
    objects.retain(
        RetainedCaptureLeafV1 {
            version: 1,
            root: root.clone(),
            slot: RetainedCaptureSlotV1::SelfCapture,
            kind: RetainedCaptureLeafKindV1::OriginalBody,
            object: body.clone(),
        },
        grant,
    )?;
    let interpretation = objects.json(
        &(
            parser.row_interpretation(),
            document.capture().requested_url(),
            document.capture().body_sha256(),
            row.locator(),
        ),
        grant,
    )?;
    if interpretation.digest.as_str() != row.reference() {
        return Err(invalid("calendar result interpretation differs"));
    }
    let subject = row
        .subject()
        .map_err(|_| invalid("calendar result subject unavailable"))?;
    Ok(ResultCalendarRowClaimV1 {
        result_root: objects.json(root, grant)?,
        body,
        interpretation,
        row_locator: digest(subject.locator().as_bytes(), grant)?,
        row_digest: AttachmentDigestV1::parse(row.original_row_digest()).map_err(domain)?,
    })
}

pub(super) struct Scope {
    pub(super) day: ProfileDate,
    pub(super) sport: Sport,
    pub(super) category: Gender,
    pub(super) year: SchoolYear,
}

fn milesplit_scope(
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
) -> CrawlResult<Scope> {
    let start = crate::context::published_performance_date(document.published_start())
        .ok_or_else(|| invalid("calendar exact performance day unresolved"))?;
    if document
        .published_end()
        .map(crate::context::published_performance_date)
        .is_some_and(|end| end != Some(start))
    {
        return Err(invalid("calendar result spans multiple days"));
    }
    let day_text = start.format("%Y-%m-%d").to_string();
    let day = ProfileDate::parse(&day_text).map_err(|_| invalid("calendar result day invalid"))?;
    let sport = document
        .parsed_page()
        .sport
        .ok_or_else(|| invalid("calendar result sport absent"))?;
    let category = row.event().gender;
    if category == Gender::Unknown {
        return Err(invalid("calendar result category unknown"));
    }
    let year = document.parsed_page().school_year;
    if SchoolYear::from_date(day.as_str()) != Some(year) {
        return Err(invalid("calendar result day and year disagree"));
    }
    Ok(Scope {
        day,
        sport,
        category,
        year,
    })
}
