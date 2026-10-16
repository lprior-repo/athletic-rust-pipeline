use super::*;
use census_domain::model::AppliedAthleteIdentity;

pub(super) fn prior_objects(history: &History) -> Vec<AttachmentObjectReferenceV1> {
    let mut objects = history
        .records
        .iter()
        .flat_map(|record| {
            record
                .command
                .reviewed_context
                .objects
                .as_slice()
                .iter()
                .cloned()
        })
        .collect::<Vec<_>>();
    objects.sort();
    objects.dedup();
    objects
}

pub(super) fn identity_scope(
    store: &Store,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
) -> TestResult<Value> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted target ledger")?;
    Ok(
        json!({"schema":"genuine-attachment-accepted-target-scope/v1", "authorship":"test-reviewed-store-output",
        "all_athletes":snapshot.scan::<CanonicalAthlete>(Table::Athletes)?,
        "all_schools":snapshot.scan::<CanonicalSchool>(Table::Schools)?,
        "all_source_observations":snapshot.scan::<SourceObservation>(Table::SourceObservations)?,
        "all_identity_decisions":snapshot.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?,
        "complete_accepted_history":accepted.history_records(), "accepted_head":accepted.head(),
        "target":inspection.target(), "witness":inspection.accepted_target_witness(),
        "evidence_generation":inspection.evidence_generation(), "derived_generation":inspection.derived_generation(),
        "public_input_revisions":input_revisions(&snapshot)?,
        "actual_issued_input_and_target_objects":inspection.issued_objects().collect::<Vec<_>>(),
        "scope":"All installed source/candidate/decision rows are retained; competing applicable subjects are not removed."}),
    )
}

fn input_revisions(snapshot: &StoreSnapshot<'_>) -> TestResult<Value> {
    let census_store::PublicIdentityInputRevisionsV2::Enabled(token) =
        snapshot.public_identity_input_revisions_v2()?
    else {
        return Err("actual accepted-target input revision capability is not enabled".into());
    };
    let revisions = [
        Table::Athletes,
        Table::SourceObservations,
        Table::ReviewCases,
        Table::IdentityVerdicts,
        Table::Performances,
        Table::RelayResults,
    ]
    .into_iter()
    .map(|table| Ok(json!({"table":table.file(), "revision":token.table_revision(table)?})))
    .collect::<TestResult<Vec<_>>>()?;
    Ok(
        json!({"schema":"genuine-attachment-public-input-revision-observation/v1", "enabled":true,
        "all_public_table_revisions":revisions,
        "opaque_baseline":"Retained and verified in actual Inspection-issued input objects; not reconstructed by this DTO"}),
    )
}

pub(super) fn lineage(
    objects: &mut Objects,
    sources: &Sources,
    publisher: &str,
    keys: &[&str],
) -> TestResult<AttachmentDigestV1> {
    let documents = keys.iter().map(|key| {
        let capture = sources.get(key)?;
        Ok(json!({"key":key, "request":capture.capture.requested_url(),
            "capture":capture.json, "body":capture.body, "acquired_at":capture.capture.acquired_at()}))
    }).collect::<TestResult<Vec<_>>>()?;
    objects.json(&json!({"schema":"genuine-attachment-reviewed-publisher-lineage/v1",
        "authorship":"test-reviewed original publication context; no new independent-publisher qualification",
        "publisher_family":publisher, "original_documents":documents,
        "upstream_producer_qualification":"Not independently issued by this fixture; retained source context is available for explicit review",
        "receipt_is_publisher":false}))
}

pub(super) fn scoped_facts(
    objects: &mut Objects,
    sources: &Sources,
    subjects: &subjects::Subjects,
    accepted_witness: &AttachmentDigestV1,
    mhsaa: &AttachmentDigestV1,
    milesplit: &AttachmentDigestV1,
    dcc: &AttachmentDigestV1,
    contexts: [&AttachmentDigestV1; 6],
) -> TestResult<Vec<ReviewedAttachmentFactV1>> {
    use AttachmentFactDispositionV1::{ContextOnly, Unavailable};
    let [article, provider, school, unavailable, raw, identities] = contexts;
    let profile_subject = sources.get("profile")?.json.digest.clone();
    let school_subject = sources.get("provider-roster")?.json.digest.clone();
    let unavailable_subject = sources.get("dcc-roster")?.json.digest.clone();
    let raw_subject = sources.get("may15")?.json.digest.clone();
    let schedule_subject = sources.get("schedule")?.json.digest.clone();
    let schedule = objects.json(&json!({"schema":"genuine-attachment-schedule-context/v1",
        "capture":sources.get("schedule")?.json, "body":sources.get("schedule")?.body,
        "season_scope":sources.participation.checked().season_scope(), "day":sources.participation.checked().day(),
        "competition":sources.participation.checked().competition(), "venue":sources.participation.checked().venue()}))?;
    let person = sources.target.checked().profile().person_gender();
    let person_fact = objects.json(&json!({"schema":"genuine-attachment-raw-person-gender/v1",
        "raw":person.raw(), "scope":person.source_scope(), "proof":diagnostics::provider_proof(person.proof())}))?;
    Ok(vec![
        fact(article.clone(), subjects.article.subject.clone(), mhsaa, ContextOnly, "Complete original article, named heading, publication and visibility inventory")?,
        fact(provider.clone(), profile_subject.clone(), milesplit, ContextOnly, "Profile document context with both linked CHSL rows; all visible row diagnostics retained")?,
        fact(person_fact, profile_subject.clone(), milesplit, ContextOnly, "Profile document raw Person.gender Female remains unmapped; no competition-category inference")?,
        fact(school.clone(), school_subject.clone(), milesplit, Unavailable, "Provider roster document lacks 7223777/Brocks; no historical departure or enrollment inference")?,
        fact(unavailable.clone(), unavailable_subject, dcc, Unavailable, "DCC roster document canonical/request mismatch withholds owner support")?,
        fact(raw.clone(), raw_subject, milesplit, ContextOnly, "May15 result document context includes all events, all rows, skipped and grade issues")?,
        fact(identities.clone(), accepted_witness.clone(), milesplit, ContextOnly, "Actual accepted-target witness and complete source/candidate inventory dependencies")?,
        fact(schedule, schedule_subject, dcc, ContextOnly, "Dated schedule document remains retained context; not a second identity publisher")?,
    ])
}

pub(super) fn paired_facts(
    stage: Stage,
    subjects: &subjects::Subjects,
    mhsaa: &AttachmentDigestV1,
    milesplit: &AttachmentDigestV1,
    facts: &mut Vec<ReviewedAttachmentFactV1>,
) -> TestResult<(
    Vec<RetainedDocumentSubjectReferenceV1>,
    Vec<AttachmentDigestV1>,
    Vec<AttachmentCorroborationPairV1>,
)> {
    use AttachmentFactDispositionV1::SupportsAttachment;
    let mut pairs = Vec::new();
    if matches!(stage, Stage::Article) {
        facts.push(fact(subjects.article_fact.clone(), subjects.article.subject.clone(), mhsaa,
            SupportsAttachment, "Original person-specific paragraph states both CHSL hurdle marks; no direct numeric-provider cross-reference")?);
        for (subject, row) in subjects.target_subjects.iter().zip(&subjects.target_facts) {
            facts.push(fact(row.clone(), subject.clone(), milesplit, SupportsAttachment,
                "Exact original same-ID provider-linked CHSL row, retained together with the other hurdle row")?);
            pairs.push(pair(&subjects.article_fact, row,
                "Explicit test review compares original named paragraph with both provider-linked CHSL rows as one exact dual-hurdles pattern")?);
        }
    }
    match stage {
        Stage::Article => Ok((
            vec![subjects.article.clone()],
            subjects.target_subjects.clone(),
            pairs,
        )),
        Stage::Results => {
            let senior = subjects
                .article_senior_hurdles
                .as_ref()
                .ok_or("B original Senior/hurdles fact")?;
            facts.push(fact(senior.clone(), subjects.article.subject.clone(), mhsaa,
                SupportsAttachment, "Original Senior heading and same-season dual-hurdles paragraph; target join requires actual active A")?);
            for (subject, row) in subjects.results.iter().zip(&subjects.result_facts) {
                facts.push(fact(row.clone(), subject.subject.clone(), milesplit, SupportsAttachment,
                    "Exact May15 row keeps name, grade12, school, Boys event/round and mark; different marks are not an exact result cross-reference")?);
                pairs.push(pair(row, senior,
                    "Reviewable person-specific context with original MHSAA Senior/dual-hurdles span through active A; A is not a publisher")?);
            }
            Ok((
                subjects.results.clone(),
                vec![subjects.article.subject.clone()],
                pairs,
            ))
        }
    }
}

fn pair(
    subject: &AttachmentDigestV1,
    target: &AttachmentDigestV1,
    explanation: &str,
) -> TestResult<AttachmentCorroborationPairV1> {
    Ok(AttachmentCorroborationPairV1 {
        subject_fact: subject.clone(),
        accepted_target_fact: target.clone(),
        basis: AttachmentCorroborationBasisV1::IndependentPersonSpecificContext,
        explanation: text(explanation)?,
    })
}
