use super::*;
use crate::participation_evidence::provider_target::ProviderTargetProofV1;
use crate::participation_evidence::{markup, DocumentSubjectV1, IndependentParticipationProofV1};

pub(super) fn document_subject(subject: &DocumentSubjectV1) -> Value {
    json!({"namespace":subject.document_namespace(), "native_document_id":subject.native_document_id(),
        "url":subject.document_url(), "body_sha256":subject.body_sha256(), "locator":subject.locator(),
        "name":subject.name(), "school":subject.school(), "reference":subject.reference()})
}

pub(super) fn provider_proof(proof: &ProviderTargetProofV1) -> Value {
    json!({"capture":proof.capture(), "interpretation":proof.interpretation(),
        "locator":proof.locator(), "acquired_at":proof.acquired_at()})
}

fn independent_proof(proof: &IndependentParticipationProofV1) -> Value {
    json!({"capture":proof.capture(), "interpretation":proof.interpretation(),
        "locator":proof.locator(), "acquired_at":proof.acquired_at()})
}

pub(super) fn article(sources: &Sources) -> TestResult<Value> {
    let checked = sources.participation.checked();
    let capture = sources.get("article")?;
    let body = capture.capture.body()?;
    let url = url::Url::parse(capture.capture.requested_url())?;
    let selected = markup::status_article(&body, url.path())?;
    check!(eq; selected.span, 106790..125395);
    check!(eq; checked.subject().locator(), "html_bytes:113861:113927");
    check!(eq; checked.subject().native_document_id(), "28974");
    let headings = elements(selected.html, b"h4", selected.span.start)?;
    let paragraphs = elements(selected.html, b"p", selected.span.start)?;
    let ownership = match checked.ownership() {
        SubjectOwnershipV1::DocumentOnly(subject) => {
            json!({"document_only":document_subject(subject)})
        }
        SubjectOwnershipV1::ProviderLink(_) => {
            return Err("genuine article unexpectedly gained a provider link".into())
        }
    };
    Ok(
        json!({"schema":"genuine-attachment-article-context/v1", "authorship":"test-reviewed-reader-output",
        "packet_reference":checked.reference().as_str(), "store":checked.store_identity(),
        "snapshot_sequence":checked.snapshot_sequence(), "subject":document_subject(checked.subject()),
        "ownership":ownership, "publication_day":checked.day(), "school_year":checked.school_year(),
        "cohort":checked.cohort(), "competition":checked.competition(), "venue":checked.venue(),
        "season_scope":checked.season_scope(), "status_proof":independent_proof(checked.status_proof()),
        "season_proof":independent_proof(checked.season_proof()), "article_span":selected.span,
        "article_attributes":selected.attributes, "article_sha256":hash(selected.html.as_bytes()),
        "all_visible_h4":headings, "all_visible_p":paragraphs,
        "checked_reader":"status::read and schedule::read; exact canonical owner, active article, unique named heading and dated schedule",
        "outbound_citations":"retained in original paragraph; no outbound citation fetched or admitted"}),
    )
}

fn elements(body: &str, tag: &[u8], offset: usize) -> TestResult<Vec<Value>> {
    markup::elements(body, tag, None)?
        .into_iter()
        .map(|element| {
            let start = offset
                .checked_add(element.span.start)
                .ok_or("diagnostic offset overflow")?;
            let end = offset
                .checked_add(element.span.end)
                .ok_or("diagnostic offset overflow")?;
            Ok(
                json!({"start":start, "end":end, "sha256":hash(element.html.as_bytes()),
            "attributes":element.attributes}),
            )
        })
        .collect()
}

pub(super) fn provider(sources: &Sources) -> TestResult<Value> {
    let checked = sources.target.checked();
    let profile = checked.profile();
    let classification = match checked.categories() {
        CompetitionCategoryEvidenceV1::Agreed { category, rows } => {
            check!(eq; *category, Gender::Boys);
            check!(eq; rows.len(), 2);
            "agreed"
        }
        CompetitionCategoryEvidenceV1::Unavailable { .. } => {
            return Err("genuine category unavailable".into())
        }
        CompetitionCategoryEvidenceV1::Conflicted { .. } => {
            return Err("genuine category conflict".into())
        }
    };
    let rows = checked.categories().rows().iter().map(|row| json!({
        "raw_category":row.raw(), "category":row.category(), "scope":row.source_scope(),
        "meet_id":row.meet_id(), "result_document_id":row.result_document_id(),
        "result_document_url":row.result_document_url(), "event_code":row.event_code(), "mark":row.mark(),
        "meet_name":row.meet_name(), "meet_day":row.meet_day(), "context":provider_proof(row.context_proof()),
        "school_context":provider_proof(row.school_proof()), "proof":provider_proof(row.proof())
    })).collect::<Vec<_>>();
    let proofs = profile
        .proofs()
        .iter()
        .map(provider_proof)
        .collect::<Vec<_>>();
    let linked = sources.get("chsl")?.capture.body()?;
    let all_rows = elements(&linked, b"tr", 0)?;
    check!(eq; profile.person_gender().raw(), Some("Female"));
    check!(eq; profile.competition_category(), Gender::Unknown);
    Ok(
        json!({"schema":"genuine-attachment-provider-context/v1", "authorship":"test-reviewed-reader-output",
        "packet_reference":checked.reference().as_str(), "profile":{
            "person":profile.person(), "name":profile.name(), "school":profile.school(),
            "school_name":profile.school_name(), "cohort":profile.cohort(), "locality":profile.locality(),
            "person_gender":{"raw":profile.person_gender().raw(), "scope":profile.person_gender().source_scope(),
                "proof":provider_proof(profile.person_gender().proof()), "disposition":"ContextOnly"},
            "competition_category":profile.competition_category(), "proofs":proofs},
        "competition_classification":classification, "all_same_id_rows":rows, "all_visible_tr":all_rows,
        "checked_reader":"complete linked-row scan refuses target-looking changed/missing ownership and conflicting same-ID categories"}),
    )
}

pub(super) fn school(sources: &Sources) -> TestResult<Value> {
    use census_domain::model::Sport;
    let checked = sources.school.checked();
    let parsed = crate::milesplit::parse_team_index(&sources.get("directory")?.capture.body()?)?;
    let verdict = checked.roster_verdict();
    let roster = verdict
        .roster()
        .ok_or("genuine provider roster unavailable")?;
    let members = roster.athletes.iter().map(|located| {
        let row = located.athlete();
        let current = located.current_roster().map(|proof| json!({"owner":proof.owner(), "member":proof.member(),
            "row":proof.row(), "section":proof.section(), "container":proof.container(), "heading":proof.heading(),
            "heading_sha256":proof.heading_sha256(), "indoor":proof.supports(Sport::IndoorTrack),
            "outdoor":proof.supports(Sport::OutdoorTrack), "xc":proof.supports(Sport::CrossCountry)}));
        json!({"locator":located.locator(), "roster_name":row.roster_name, "name":row.name,
            "gender":row.gender, "grad_year":row.grad_year, "athlete_id":row.athlete_id,
            "profile_url":row.profile_url, "indoor":row.indoor, "outdoor":row.outdoor, "xc":row.xc,
            "current_roster":current, "temporal_unavailability":format!("{:?}", located.temporal_unavailability())})
    }).collect::<Vec<_>>();
    check!(roster
        .athletes
        .iter()
        .all(|row| row.athlete().athlete_id != "7223777"));
    check!(!sources
        .get("provider-roster")?
        .capture
        .body()?
        .contains("Brocks"));
    Ok(
        json!({"schema":"genuine-attachment-provider-school-context/v1", "authorship":"test-reviewed-reader-output",
        "packet_reference":checked.reference().as_str(), "school":checked.school(), "observation":checked.observation(),
        "directory_proof":checked.directory_proof(), "all_directory_teams":parsed.teams,
        "directory_disposition":parsed.disposition, "directory_errors":parsed.errors, "directory_unfinished":parsed.unfinished,
        "roster_team":roster.team, "all_roster_members":members, "roster_rejections":verdict.rejections(),
        "roster_unfinished":verdict.unfinished(), "current_roster_target_support":"Unavailable",
        "absent_provider_person":"7223777", "absence_meaning":"October roster has no subject support; no historical departure or competing-person inference",
        "contacts_disposition":"ContextOnly; current owner/contact details do not prove historical enrollment or campus"}),
    )
}

pub(super) fn unavailable_roster(sources: &Sources) -> TestResult<Value> {
    let capture = &sources.get("dcc-roster")?.capture;
    let body = capture.body()?;
    let refusal = markup::canonical(&body, capture.requested_url());
    let error = match refusal {
        Err(StoreError::Refused { detail })
            if detail == "participation declared canonical owner differs" =>
        {
            detail
        }
        other => {
            return Err(format!("expected genuine DCC canonical mismatch, got {other:?}").into())
        }
    };
    Ok(
        json!({"schema":"genuine-attachment-unavailable-dcc-owner/v1", "authorship":"test-reviewed-reader-output",
        "request":capture.requested_url(), "final_url":serde_json::to_value(capture)?.get("final_url"), "acquired_at":capture.acquired_at(),
        "body":sources.get("dcc-roster")?.body, "capture":sources.get("dcc-roster")?.json,
        "disposition":"Unavailable", "checked_owner_refusal":error,
        "meaning":"Original canonical/request mismatch preserved; no URL rewrite or accepted person-roster ownership"}),
    )
}

pub(super) fn raw_results(sources: &Sources) -> TestResult<Value> {
    let document = sources.result.read(&locators())?;
    let page = document.parsed_page();
    Ok(
        json!({"schema":"genuine-attachment-complete-raw-result-context/v1", "authorship":"test-reviewed-reader-output",
        "reader_reference":document.context_reference(), "meet_id":document.meet_id(),
        "result_document_id":document.result_document_id(), "published_start":document.published_start(),
        "published_end":document.published_end(), "all_events_and_rows":page.meet, "sport":page.sport,
        "region":page.region, "school_year":page.school_year, "all_skipped":page.skipped,
        "all_grade_issues":page.grade_issues, "requested":locators(), "selected_count":document.selected_count(),
        "owner":"document-row only; original raw document publishes no numeric provider athlete owner"}),
    )
}
