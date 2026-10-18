use super::{invalid, markup, DocumentSubjectV1, RetainedHtmlCaptureV1, SubjectOwnershipV1};
use census_domain::model::{Grade, ProfileDate, SchoolYear, SourceIdentity, SourceNamespace};
use census_store::StoreResult;
use chrono::NaiveDate;

pub(super) struct Status {
    pub(super) subject: DocumentSubjectV1,
    pub(super) ownership: SubjectOwnershipV1,
    pub(super) day: ProfileDate,
    pub(super) grade: Grade,
}

pub(super) fn read(capture: &RetainedHtmlCaptureV1) -> StoreResult<Status> {
    let body = capture.body()?;
    read_context(capture, &body).map(|(status, _)| status)
}

pub(super) fn read_context<'a>(
    capture: &RetainedHtmlCaptureV1,
    body: &'a str,
) -> StoreResult<(Status, markup::Element<'a>)> {
    let url =
        url::Url::parse(capture.requested_url()).map_err(|_| invalid("status URL invalid"))?;
    if url.host_str() != Some("www.mhsaa.com")
        || url.query().is_some()
        || !url.path().starts_with("/sports/boys-track-field/stories/")
    {
        return Err(invalid("unsupported student-status issuer or sport"));
    }
    markup::canonical(body, capture.requested_url())?;
    let article = markup::status_article(body, url.path())?;
    if article.attribute("data-nid").is_none_or(|id| !numeric(id)) {
        return Err(invalid(
            "status article has no exact numeric document identity",
        ));
    }
    let date = markup::single(markup::elements(article.html, b"p", Some("publish-date"))?)?;
    let day = NaiveDate::parse_from_str(&markup::text(date.html)?, "%B %e, %Y")
        .map_err(|_| invalid("student-status article publication day invalid"))?;
    let day = ProfileDate::parse(&day.format("%Y-%m-%d").to_string())
        .map_err(|_| invalid("student-status day unsupported"))?;
    let heading = subject_heading(article.html, &day)?;
    let (name, school, grade) = statement(&markup::text(heading.html)?)?;
    let start = article
        .span
        .start
        .checked_add(heading.span.start)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    let end = article
        .span
        .start
        .checked_add(heading.span.end)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    let subject = DocumentSubjectV1 {
        document_namespace: "mhsaa_article_v1".into(),
        native_document_id: article
            .attribute("data-nid")
            .ok_or_else(|| invalid("article identity absent"))?
            .into(),
        document_url: capture.requested_url().into(),
        body_sha256: capture.body_sha256().into(),
        locator: format!("html_bytes:{start}:{end}"),
        name,
        school,
    };
    let ownership = ownership(heading.html, &subject)?;
    Ok((
        Status {
            subject,
            ownership,
            day,
            grade,
        },
        article,
    ))
}

fn subject_heading<'a>(html: &'a str, day: &ProfileDate) -> StoreResult<markup::Element<'a>> {
    let headings = markup::elements(html, b"h4", None)?;
    let mut selected = None;
    let mut seen_index = false;
    for heading in headings {
        if !heading.html.contains("text--secondary") {
            continue;
        }
        let text = markup::text(heading.html)?;
        if text.starts_with("Previous ") && text.ends_with(" honorees") {
            index_heading(&heading, &text, day)?;
            if selected.is_none() || seen_index {
                return Err(invalid(
                    "status index heading is repeated or precedes subject",
                ));
            }
            seen_index = true;
            continue;
        }
        statement(&text)?;
        if selected.is_some() || seen_index {
            return Err(invalid("status subject heading is ambiguous"));
        }
        selected = Some(heading);
    }
    selected.ok_or_else(|| invalid("status subject heading is missing"))
}

fn index_heading(heading: &markup::Element<'_>, text: &str, day: &ProfileDate) -> StoreResult<()> {
    let year = SchoolYear::from_date(day.as_str())
        .ok_or_else(|| invalid("status index day has no canonical school year"))?;
    let span = markup::single(markup::elements(
        heading.html,
        b"span",
        Some("text--secondary"),
    )?)?;
    if text != format!("Previous {} honorees", year.short()) || markup::text(span.html)? != text {
        return Err(invalid("unsupported status index heading"));
    }
    Ok(())
}

fn statement(value: &str) -> StoreResult<(String, String, Grade)> {
    let (subject, status) = value
        .split_once(" | ")
        .ok_or_else(|| invalid("student-status heading shape unsupported"))?;
    let (name, school) = subject
        .split_once('♦')
        .ok_or_else(|| invalid("student-status subject absent"))?;
    let (class, sport) = status
        .split_once('♦')
        .ok_or_else(|| invalid("student-status class absent"))?;
    if !matches!(class.trim(), "Freshman" | "Sophomore" | "Junior" | "Senior")
        || sport.trim() != "Track"
        || school.trim() != "Detroit Catholic Central"
        || name.trim().is_empty()
        || name.len() > 256
    {
        return Err(invalid("unsupported explicit high-school status statement"));
    }
    let grade = match class.trim() {
        "Freshman" => 9,
        "Sophomore" => 10,
        "Junior" => 11,
        "Senior" => 12,
        _ => return Err(invalid("unsupported student class")),
    };
    let grade = Grade::new(grade).ok_or_else(|| invalid("unsupported student grade"))?;
    Ok((name.trim().into(), school.trim().into(), grade))
}

fn ownership(html: &str, subject: &DocumentSubjectV1) -> StoreResult<SubjectOwnershipV1> {
    let links = markup::elements(html, b"a", None)?;
    if links.is_empty() {
        return Ok(SubjectOwnershipV1::DocumentOnly(subject.clone()));
    }
    let link = markup::single(links)?;
    if markup::text(link.html)? != subject.name {
        return Err(invalid(
            "subject link does not enclose the exact named person",
        ));
    }
    let raw = link
        .attribute("href")
        .ok_or_else(|| invalid("subject link target absent"))?;
    let url = url::Url::parse(raw).map_err(|_| invalid("subject link URL invalid"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("mi.milesplit.com")
        || url.query().is_some()
        || url.fragment().is_some()
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid("subject provider link is unsupported"));
    }
    let token = url
        .path()
        .strip_prefix("/athletes/")
        .ok_or_else(|| invalid("subject provider path unsupported"))?;
    let id = token
        .split('-')
        .next()
        .ok_or_else(|| invalid("subject provider identifier absent"))?;
    if !numeric(id) || token.contains('/') {
        return Err(invalid("subject provider identifier invalid"));
    }
    let mut identity = SourceIdentity::new(SourceNamespace::MilesplitAthlete, id);
    identity.url = Some(raw.into());
    Ok(SubjectOwnershipV1::ProviderLink(identity))
}

fn numeric(value: &str) -> bool {
    !value.starts_with('0')
        && !value.is_empty()
        && value.len() <= 20
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok()
}
