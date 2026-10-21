#[path = "qualification_genuine_workbook/attachment.rs"]
mod attachment;
#[path = "qualification_genuine_workbook/calendar.rs"]
mod calendar;
#[path = "qualification_genuine_workbook/event_context.rs"]
mod event_context;
#[path = "qualification_genuine_workbook/point.rs"]
mod point;
#[path = "qualification_genuine_workbook/school.rs"]
mod school;

use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketRefV1;
use census_crawl::participation_evidence::provider_target::SourceTargetPacketV1;
use census_crawl::participation_evidence::recorded::record_independent_participation_packet;
use census_crawl::participation_evidence::recorded::record_provider_target_intake;
use census_crawl::participation_evidence::recorded::record_retained_html_result_capture;
use census_crawl::participation_evidence::recorded::ProviderTargetIntakeDispositionV1;
use census_crawl::participation_evidence::result_ownership::read_retained_html_result_rows;
use census_crawl::participation_evidence::result_ownership::school_correspondence;
use census_crawl::participation_evidence::IndependentParticipationPacketV1;
use census_crawl::participation_evidence::RetainedHtmlCaptureV1;
use census_crawl::recording::RowSink;
use census_crawl::Recording;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::Gender;
use census_domain::model::ProfileDate;
use census_domain::model::ProfileSelectionV1;
use census_domain::model::SchoolYear;
use census_domain::model::Sport;
use census_domain::result_calendar_correspondence::ResultCalendarReviewActorV1;
use census_domain::result_calendar_correspondence::ResultCalendarTextV1;
use census_domain::result_school_correspondence::ResultSchoolReviewActorV1;
use census_domain::result_school_correspondence::ResultSchoolTextV1;
use census_store::Application;
use census_store::ReadBudget;
use census_store::Table;
use std::path::Path;
use std::path::PathBuf;

pub(crate) type Error = Box<dyn std::error::Error>;

struct Args {
    root: PathBuf,
    selection: PathBuf,
    captures: PathBuf,
    locator: SourceRowLocator,
    out: PathBuf,
    objects: PathBuf,
}

fn main() -> Result<(), Error> {
    run()
}

fn run() -> Result<(), Error> {
    let args = parse_args()?;
    let selection: ProfileSelectionV1 = serde_json::from_slice(&std::fs::read(&args.selection)?)?;
    let (store, _writer) =
        census_store::Store::open_current_qualification_v3(&args.root)?.into_parts();
    let profile = store.snapshot().selected_profile(&selection)?;
    let profile_capture = load_capture(&args.captures.join("owen-profile.capture.json"))?;
    let linked_capture = load_capture(&args.captures.join("chsl-linked.capture.json"))?;
    let result_capture = load_capture(&args.captures.join("chsl-760382-1300204.capture.json"))?;
    let schedule_capture = load_capture(&args.captures.join("dcc-schedule.capture.json"))?;
    let (candidate, target_ref) = intake_target(&store, profile_capture, linked_capture)
        .map_err(|error| format!("genuine target intake: {error}"))?;
    let (context_ref, published_day, source_name, source_key) =
        verify_and_record_result(&store, &result_capture, args.locator, &candidate)
            .map_err(|error| format!("genuine original result intake: {error}"))?;
    let participation_ref = record_participation(
        &store,
        result_capture.clone(),
        schedule_capture.clone(),
        args.locator,
    )
    .map_err(|error| format!("genuine participation intake: {error}"))?;
    let (attachment_digest, attachment_public) = attachment::ensure_person(
        &store,
        &candidate,
        &context_ref,
        &target_ref,
        args.locator,
        &args.objects,
    )
    .map_err(|error| format!("genuine person review: {error}"))?;
    let event_context_digest = attachment::ensure_event_context(
        &store,
        &candidate,
        &context_ref,
        &target_ref,
        args.locator,
        &args.objects,
    )
    .map_err(|error| format!("genuine event-context review: {error}"))?;
    eprintln!("genuine event-context review: {event_context_digest}");
    let point_digest = point::ensure_point(
        &store,
        &candidate,
        &participation_ref,
        &target_ref,
        &args.objects,
    )
    .map_err(|error| format!("genuine point review: {error}"))?;
    ensure_school_enrollment(&store)?;
    ensure_calendar_enrollment(&store)?;
    let (school_id, school_binding) = school::ensure(
        &store,
        &args.root,
        &args.objects,
        &context_ref,
        args.locator,
    )?;
    let calendar_digest = calendar::ensure(
        &store,
        &args.objects,
        &context_ref,
        args.locator,
        &participation_ref,
    )?;
    let performance_id = record_performances(
        &store,
        &result_capture,
        args.locator,
        &candidate,
        &school_id,
        &attachment_digest,
        &school_binding,
    )?;
    publish_genuine(
        &store,
        &selection,
        &profile,
        &args,
        &target_ref,
        &context_ref,
        &participation_ref,
        &candidate,
        &attachment_digest,
        &attachment_public,
        &point_digest,
        &school_binding,
        &calendar_digest,
        &performance_id,
        &published_day,
        &source_name,
        &source_key,
    )
}

fn parse_args() -> Result<Args, Error> {
    let raw: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if raw.len() < 6 {
        return Err("usage: qualification_genuine_workbook <root> <selection> <captures> <ordinal:offset:length> <out> [objects]".into());
    }
    let root = PathBuf::from(&raw[1]);
    let selection = PathBuf::from(&raw[2]);
    let captures = PathBuf::from(&raw[3]);
    let locator = parse_locator(raw[4].to_str().ok_or("locator is not unicode")?)?;
    let out = PathBuf::from(&raw[5]);
    let objects = if raw.len() > 6 {
        PathBuf::from(&raw[6])
    } else {
        match root.parent() {
            Some(parent) => parent.join("genuine-workbook-objects"),
            None => return Err("qualification root has no parent for objects".into()),
        }
    };
    std::fs::create_dir_all(&objects)?;
    Ok(Args {
        root,
        selection,
        captures,
        locator,
        out,
        objects,
    })
}

fn parse_locator(raw: &str) -> Result<SourceRowLocator, Error> {
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() != 3 {
        return Err("locator must be ordinal:byte_offset:byte_length".into());
    }
    let ordinal: u32 = parts[0].parse().map_err(|_| "locator ordinal invalid")?;
    let byte_offset: usize = parts[1]
        .parse()
        .map_err(|_| "locator byte offset invalid")?;
    let byte_length: usize = parts[2]
        .parse()
        .map_err(|_| "locator byte length invalid")?;
    if byte_length == 0 {
        return Err("locator byte length is empty".into());
    }
    byte_offset
        .checked_add(byte_length)
        .ok_or("locator span overflows")?;
    Ok(SourceRowLocator {
        ordinal,
        byte_offset,
        byte_length,
    })
}

pub(crate) fn load_capture(path: &Path) -> Result<RetainedHtmlCaptureV1, Error> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("reading capture {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| format!("decoding capture {}: {e}", path.display()).into())
}

pub(crate) fn school_budget() -> ReadBudget {
    ReadBudget {
        rows: 65_536,
        bytes: 32 * 1024 * 1024,
        record_bytes: 2 * 1024 * 1024,
    }
}

pub(crate) fn apply_recording(
    store: &census_store::Store,
    recording: Recording,
    prefix: &str,
) -> Result<(), Error> {
    let recorded = recording.drain();
    let prepared = census_crawl::recording::prepare_recorded(recorded, prefix)?;
    let _ = prepared.apply(store)?;
    Ok(())
}
fn provider_school_reference(root: &Path) -> Result<ProviderSchoolPacketRefV1, Error> {
    const PINNED: &str = "4859e01fafbd9315a346da3bfd77955c9badfa60bbdfbca3900385f9491ad50d";
    let path = root
        .parent()
        .ok_or("qualification root has no parent for school packet")?
        .join("original-provider-school-packet.json");
    if std::fs::metadata(&path).is_ok() {
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("reading school packet {}: {e}", path.display()))?;
        let packet: census_crawl::participation_evidence::provider_school::ProviderSchoolPacketV1 =
            serde_json::from_slice(&bytes)
                .map_err(|e| format!("decoding school packet {}: {e}", path.display()))?;
        let derived = packet
            .reference()
            .map_err(|e| format!("genuine school packet reference refused: {e}"))?;
        if derived.as_str() != PINNED {
            return Err(
                "genuine school packet file differs from pinned qualification input".into(),
            );
        }
        return Ok(derived);
    }
    ProviderSchoolPacketRefV1::parse(PINNED)
        .map_err(|e| format!("genuine school pin refused: {e}").into())
}

fn intake_target(
    store: &census_store::Store,
    profile: RetainedHtmlCaptureV1,
    linked: RetainedHtmlCaptureV1,
) -> Result<(census_domain::model::AthleteId, String), Error> {
    let packet = SourceTargetPacketV1::new(profile, linked)?;
    let checked = packet.read()?;
    let reference = checked.reference().as_str().to_owned();
    let recording = Recording::new();
    let disposition = record_provider_target_intake(store, &recording, &packet, school_budget())?;
    match disposition {
        ProviderTargetIntakeDispositionV1::ProjectedUnaccepted { candidate } => {
            apply_recording(store, recording, "genuine-target-intake")?;
            Ok((candidate, reference))
        }
        ProviderTargetIntakeDispositionV1::RetainedOnly(reason) => {
            Err(format!("genuine target intake withheld without athlete: {reason:?}").into())
        }
    }
}

fn verify_and_record_result(
    store: &census_store::Store,
    capture: &RetainedHtmlCaptureV1,
    locator: SourceRowLocator,
    candidate: &census_domain::model::AthleteId,
) -> Result<(String, String, String, String), Error> {
    let document = read_retained_html_result_rows(capture, &[locator])
        .map_err(|e| format!("genuine result row refused: {e}"))?;
    if document.selected_count() != 1 {
        return Err("genuine result document must select exactly one row".into());
    }
    let row = document
        .row(0)
        .ok_or("genuine result document has no selected row")?;
    if row.locator() != locator {
        return Err("genuine result row locator differs after check".into());
    }
    let parsed = row.parsed_row();
    if !parsed.legs.is_empty() {
        return Err("genuine relay legs cannot establish individual status".into());
    }
    let _statement = row
        .school_statement()
        .map_err(|e| format!("genuine school statement refused: {e}"))?;
    let subject = row
        .subject()
        .map_err(|e| format!("genuine row subject refused: {e}"))?;
    let published_start = document.published_start().to_owned();
    if let Some(end) = document.published_end() {
        if end != published_start {
            return Err("genuine multiday result cannot establish a single day".into());
        }
    }
    let identity =
        census_crawl::participation_evidence::physical_result::project_retained_html_result_identity(
            &document, 0, candidate,
        )
        .map_err(|e| format!("genuine result identity refused: {e}"))?;
    if identity.date() != published_start {
        return Err("genuine identity date differs from original document day".into());
    }
    if identity.athlete() != candidate {
        return Err("genuine identity athlete differs from target candidate".into());
    }
    let recording = Recording::new();
    record_retained_html_result_capture(store, &recording, &document)
        .map_err(|e| format!("genuine result capture refused: {e}"))?;
    apply_recording(store, recording, "genuine-result-intake")?;
    let _ = subject.reference();
    Ok((
        document.context_reference().to_owned(),
        published_start,
        parsed.name.clone(),
        identity.source_key().to_owned(),
    ))
}

fn record_participation(
    store: &census_store::Store,
    status: RetainedHtmlCaptureV1,
    schedule: RetainedHtmlCaptureV1,
    locator: SourceRowLocator,
) -> Result<String, Error> {
    let packet = IndependentParticipationPacketV1::from_result_row(status, schedule, locator)
        .map_err(|e| format!("genuine participation packet refused: {e}"))?;
    let reference = packet
        .reference()
        .map_err(|e| format!("genuine participation reference refused: {e}"))?;
    let recording = Recording::new();
    record_independent_participation_packet(store, &recording, &packet)
        .map_err(|e| format!("genuine participation intake refused: {e}"))?;
    apply_recording(store, recording, "genuine-participation-intake")?;
    Ok(reference.as_str().to_owned())
}

fn ensure_school_enrollment(store: &census_store::Store) -> Result<(), Error> {
    if !matches!(store.store_format()?.lifecycle_revision, Some(7 | 8)) {
        return Ok(());
    }
    let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_enrollment_v1(
        inputs,
        census_store::ResultSchoolEnrollmentReviewV1 {
            operation: ResultSchoolTextV1::parse("genuine-school-enrollment")?,
            actor: ResultSchoolReviewActorV1::LocalOperator,
            reviewed_at: chrono::Utc::now(),
            rationale: ResultSchoolTextV1::parse(
                "Genuine capability enrollment for original school-cell correspondence review",
            )?,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_school_enrollment_v1(prepared, &approved, &digest)? {
        Application::Written(_) => Ok(()),
        Application::Repeated(_) => Ok(()),
    }
}

fn ensure_calendar_enrollment(store: &census_store::Store) -> Result<(), Error> {
    if !matches!(store.store_format()?.lifecycle_revision, Some(9)) {
        return Ok(());
    }
    let inputs = store.inspect_result_calendar_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_calendar_enrollment_v1(
        inputs,
        census_store::ResultCalendarEnrollmentReviewV1 {
            operation: ResultCalendarTextV1::parse("genuine-calendar-enrollment")?,
            actor: ResultCalendarReviewActorV1::LocalOperator,
            reviewed_at: chrono::Utc::now(),
            rationale: ResultCalendarTextV1::parse(
                "Genuine capability enrollment for original calendar-day correspondence review",
            )?,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_calendar_enrollment_v1(prepared, &approved, &digest)? {
        Application::Written(_) => Ok(()),
        Application::Repeated(_) => Ok(()),
    }
}

fn school_scope(
    document: &census_crawl::participation_evidence::result_ownership::CheckedHtmlResultDocumentV1<
        '_,
    >,
    row: &census_crawl::participation_evidence::result_ownership::CheckedHtmlResultRowV1<'_>,
) -> Result<school_correspondence::ResultSchoolScopeV1, census_crawl::CrawlError> {
    let (Some(document), Some(row)) = (document.as_milesplit(), row.as_milesplit()) else {
        return Err(census_store::StoreError::Refused {
            detail: "genuine school parser contract does not support this source".into(),
        }
        .into());
    };
    let day = ProfileDate::parse(document.published_start()).map_err(|_| {
        census_store::StoreError::Refused {
            detail: "genuine performance day unsupported".into(),
        }
    })?;
    if document
        .published_end()
        .is_some_and(|end| end != document.published_start())
    {
        return Err(census_store::StoreError::Refused {
            detail: "genuine published end differs".into(),
        }
        .into());
    }
    let sport: Sport = document
        .parsed_page()
        .sport
        .ok_or(census_store::StoreError::Refused {
            detail: "genuine result sport absent".into(),
        })?;
    let category: Gender = row.event().gender;
    if category == Gender::Unknown {
        return Err(census_store::StoreError::Refused {
            detail: "genuine result category unknown".into(),
        }
        .into());
    }
    let year: SchoolYear =
        SchoolYear::from_date(day.as_str()).ok_or(census_store::StoreError::Refused {
            detail: "genuine school year unresolved".into(),
        })?;
    if year != document.parsed_page().school_year {
        return Err(census_store::StoreError::Refused {
            detail: "genuine school year differs".into(),
        }
        .into());
    }
    Ok(school_correspondence::ResultSchoolScopeV1 {
        day,
        sport,
        category,
        year,
    })
}

fn record_performances(
    store: &census_store::Store,
    capture: &RetainedHtmlCaptureV1,
    locator: SourceRowLocator,
    candidate: &census_domain::model::AthleteId,
    school: &census_domain::model::SchoolId,
    attachment: &str,
    school_binding: &str,
) -> Result<String, Error> {
    let document = read_retained_html_result_rows(capture, &[locator])
        .map_err(|e| format!("genuine projection document refused: {e}"))?;
    let row = document.row(0).ok_or("genuine projection row absent")?;
    let subject_reference = row
        .subject()
        .map_err(|e| format!("genuine projection subject refused: {e}"))?
        .reference();
    let target =
        census_crawl::participation_evidence::physical_result::ProposedHtmlResultTargetV1 {
            locator,
            subject_reference: subject_reference.as_str(),
            proposed_athlete: candidate,
            proposed_school: school,
            proposed_attachment_reference: attachment,
            proposed_school_binding_reference: school_binding,
        };
    let projection =
        census_crawl::participation_evidence::physical_result::project_retained_html_results(
            &document,
            &[target],
        )
        .map_err(|e| format!("genuine physical projection refused: {e}"))?;
    if projection.rows().len() != 1 {
        return Err("genuine physical projection must yield exactly one row".into());
    }
    let (meet, rows) = projection.into_proposed_entities();
    let mut iter = rows.into_iter();
    let row = iter.next().ok_or("genuine projected row absent")?;
    if iter.next().is_some() {
        return Err("genuine physical projection yielded extra rows".into());
    }
    let (event, team, performance) = row.into_proposed_entities();
    let performance_id = performance.id.as_str().to_owned();
    let recording = Recording::new();
    let sink = RowSink::Record {
        store,
        recording: &recording,
    };
    let mut batch = sink.write_batch();
    batch.append_many(Table::Meets, &[meet])?;
    batch.append_many(Table::Events, &[event])?;
    batch.append_many(Table::Teams, &[team])?;
    batch.append_many(Table::Performances, &[performance])?;
    batch.commit()?;
    apply_recording(store, recording, "genuine-physical-performances")?;
    Ok(performance_id)
}

fn publish_genuine(
    store: &census_store::Store,
    selection: &ProfileSelectionV1,
    profile: &census_domain::model::QueryProfileV1,
    args: &Args,
    target_ref: &str,
    context_ref: &str,
    participation_ref: &str,
    candidate: &census_domain::model::AthleteId,
    attachment: &str,
    public: &str,
    point: &str,
    school_binding: &str,
    calendar: &str,
    performance: &str,
    published_day: &str,
    source_name: &str,
    source_key: &str,
) -> Result<(), Error> {
    let snapshot = store.snapshot();
    let selected_objects = tempfile::tempdir_in(&args.objects)?;
    let dataset = census_report::export::ExportDataset::from_selected_snapshot(
        store,
        &snapshot,
        selection,
        selected_objects.path(),
    )?;
    let [cohort] = profile.input().graduation_years.as_slice() else {
        return Err("genuine workbook blocked: profile must select exactly one cohort".into());
    };
    let outcome = census_report::bests::build_with_exclusions(
        &dataset,
        &census_report::bests::Options {
            scope: census_report::report::Scope::AllSources,
            grad_year: Some(cohort.get()),
            limit: None,
            jurisdictions: profile.input().jurisdictions.clone(),
        },
    )?;
    if outcome.selections.is_empty() {
        eprintln!(
            "genuine checked event context: {}",
            serde_json::to_string_pretty(&event_context::inspect(store)?)?
        );
        let derivation = census_report::report::Derivation::of(
            &dataset,
            census_report::report::Scope::AllSources,
            Some(cohort.get()),
            &profile.input().jurisdictions,
        )?;
        eprintln!(
            "genuine selector inputs: {}",
            serde_json::to_string_pretty(&serde_json::json!({
                "candidate": candidate,
                "derived_athletes": derivation.athletes(),
                "derived_performances": derivation.performances(),
                "affiliation_exclusions": format!("{:?}", derivation.affiliation_exclusions()),
                "event_admission": census_report::report::EventAdmission::of(&dataset)?.audit(),
                "candidate_row": dataset.athletes.iter().find(|row| &row.id == candidate),
                "performances": dataset.performances,
                "events": dataset.events,
                "meets": dataset.meets,
                "teams": dataset.teams,
                "schools": dataset.schools,
            }))?
        );
        return Err(format!(
            "genuine workbook blocked: no eligible selected best marks; raw athletes={}, performances={}; participation={:?}; combined={:?}",
            dataset.athletes.len(),
            dataset.performances.len(),
            outcome.participation_exclusions,
            outcome.combined_exclusions,
        ).into());
    }
    let mut best = None;
    for selection_row in &outcome.selections {
        if selection_row.source.source_key == source_key {
            best = Some(selection_row);
            break;
        }
    }
    let best =
        best.ok_or("genuine workbook blocked: original May9 row is not the eligible best")?;
    if best.meet.date != published_day {
        return Err(
            "genuine workbook blocked: eligible best date differs from original day".into(),
        );
    }
    if best.source.performance_id.as_str() != performance {
        return Err(
            "genuine workbook blocked: eligible best is not the recorded physical performance"
                .into(),
        );
    }
    if best.athlete_id() != candidate {
        return Err(
            "genuine workbook blocked: eligible best athlete differs from candidate".into(),
        );
    }
    let options =
        census_report::workbook::options_for_selected_dataset(&dataset, Some(args.out.clone()))?;
    let out_base = store.out_dir();
    let censuses =
        census_report::workbook::Censuses::of(&dataset, &out_base, &options.jurisdictions)?;
    std::fs::create_dir(&args.out)?;
    let path = census_report::workbook::build_from(&dataset, store, &options, &censuses)?;
    let verified = census_report::workbook::publication::verify_published(&path)?;
    if verified.workbook.mapped_athletes == 0 || verified.workbook.rows == 0 {
        return Err("genuine workbook blocked: verified workbook maps no athletes or rows".into());
    }
    println!(
        "{}",
        serde_json::json!({
            "store": selection.store_identity(),
            "profile": selection.profile_id(),
            "target_packet": target_ref,
            "result_context": context_ref,
            "participation": participation_ref,
            "candidate": candidate.as_str(),
            "public": public,
            "attachment": attachment,
            "point": point,
            "school_binding": school_binding,
            "calendar": calendar,
            "performance": performance,
            "original_day": published_day,
            "original_athlete": source_name,
            "raw_athletes": dataset.athletes.len(),
            "raw_performances": dataset.performances.len(),
            "selected_best_marks": outcome.selections.len(),
            "participation_exclusions": outcome.participation_exclusions.len(),
            "combined_exclusions": outcome.combined_exclusions.len(),
            "owen_best": {
                "name": best.athlete.name,
                "date": best.meet.date,
                "mark": best.mark_text(),
                "event": best.event_key(),
                "source_key": best.source.source_key,
                "result_url": best.source.result_url,
            },
            "workbook": path.display().to_string(),
            "generation": verified.generation_digest,
        })
    );
    Ok(())
}
