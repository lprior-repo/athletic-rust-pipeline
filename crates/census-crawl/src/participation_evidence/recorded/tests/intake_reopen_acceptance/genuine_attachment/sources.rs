use super::*;
use serde_json::value::RawValue;
use std::collections::BTreeMap;

const DCC_ROSTER: &[u8] = include_bytes!("fixtures/dcc-roster.html");
const DCC_HEADERS: &[u8] = include_bytes!("fixtures/dcc-roster.headers");
const DCC_URL: &str =
    "https://dccshamrocks.net/sports/track-and-field/roster/zacchaeus-brocks/6591";

pub(super) struct Capture {
    pub(super) key: &'static str,
    pub(super) capture: RetainedHtmlCaptureV1,
    pub(super) json: AttachmentObjectReferenceV1,
    pub(super) body: AttachmentObjectReferenceV1,
    pub(super) original_json: Vec<u8>,
    pub(super) original_body: Vec<u8>,
}

pub(super) struct Sources {
    pub(super) objects: Objects,
    pub(super) captures: Vec<Capture>,
    pub(super) roots: Vec<RetainedCaptureRootV1>,
    pub(super) school: RecordedProviderSchoolV1,
    pub(super) target: RecordedProviderTargetV1,
    pub(super) participation: RecordedIndependentParticipationV1,
    pub(super) result: RecordedHtmlResultCaptureV1,
    pub(super) source_read_usage: AttachmentReplayUsageV1,
}

impl Sources {
    pub(super) fn install_and_read(store: &Store) -> TestResult<Self> {
        let packet = participation_packet()?;
        let capture = html_capture()?;
        let document = read_retained_html_result_rows(&capture, &locators())?;
        let recording = Recording::new();
        record_independent_participation_packet(store, &recording, &packet)?;
        record_retained_html_result_capture(store, &recording, &document)?;
        let recorded = recording.drain();
        let prepared =
            crate::recording::prepare_recorded(recorded, "genuine-attachment-original-inputs")?;
        let applied = prepared.apply(store)?;
        check!(eq; applied.appended(), 0);
        let snapshot = store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let school_packet = school_fixtures::packet(school_fixtures::DIRECTORY)?;
        let school =
            read_recorded_provider_school(&snapshot, &school_packet.reference()?, &mut grant)?
                .ok_or("genuine provider school retained root")?;
        let target = read_recorded_provider_target(
            &snapshot,
            &provider_packet(RESULTS)?.reference()?,
            &mut grant,
        )?
        .ok_or("genuine provider target retained root")?;
        let participation =
            read_recorded_independent_participation(&snapshot, &packet.reference()?, &mut grant)?
                .ok_or("genuine participation retained root")?;
        let result = read_recorded_html_result_capture(
            &snapshot,
            document.context_reference(),
            &locators(),
            &mut grant,
        )?
        .ok_or("genuine result retained root")?;
        school_fixtures::verify_checked_school(school.checked())?;
        let roots = vec![
            school.root().clone(),
            target.root().clone(),
            participation.root().clone(),
            result.root().clone(),
        ];
        let mut sources = Self {
            objects: Objects::default(),
            captures: Vec::new(),
            roots,
            school,
            target,
            participation,
            result,
            source_read_usage: grant.usage(),
        };
        sources.capture(
            &snapshot,
            0,
            "directory",
            "directory",
            RetainedCaptureSlotV1::ProviderSchoolDirectory,
            school_fixtures::DIRECTORY,
        )?;
        sources.capture(
            &snapshot,
            0,
            "roster",
            "provider-roster",
            RetainedCaptureSlotV1::ProviderSchoolRoster,
            school_fixtures::ROSTER,
        )?;
        sources.capture(
            &snapshot,
            1,
            "profile",
            "profile",
            RetainedCaptureSlotV1::ProviderProfile,
            PROFILE,
        )?;
        sources.capture(
            &snapshot,
            1,
            "linked_results",
            "chsl",
            RetainedCaptureSlotV1::ProviderLinkedResults,
            RESULTS,
        )?;
        sources.capture(
            &snapshot,
            2,
            "status",
            "article",
            RetainedCaptureSlotV1::ParticipationStatus,
            STATUS,
        )?;
        sources.capture(
            &snapshot,
            2,
            "schedule",
            "schedule",
            RetainedCaptureSlotV1::ParticipationSchedule,
            SCHEDULE,
        )?;
        sources.capture(
            &snapshot,
            3,
            "",
            "may15",
            RetainedCaptureSlotV1::SelfCapture,
            HTML,
        )?;
        sources.unavailable_roster()?;
        check!(eq; sources.captures.len(), 8);
        check!(eq; sources.objects.0.len(), 16);
        Ok(sources)
    }

    fn capture(
        &mut self,
        snapshot: &StoreSnapshot<'_>,
        index: usize,
        field: &str,
        key: &'static str,
        slot: RetainedCaptureSlotV1,
        original: &[u8],
    ) -> TestResult {
        let root = self.roots.get(index).ok_or("root slot")?;
        let payload = snapshot
            .journal_payload_raw_bounded(
                root.schema.phase(),
                &root.key,
                JournalReadBudget::new(2 * 1024 * 1024)?,
            )?
            .ok_or("installed raw payload")?;
        check!(eq; reference(payload.get().as_bytes())?, root.payload);
        let raw = if field.is_empty() {
            payload.get().to_owned()
        } else {
            let fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(payload.get())?;
            fields
                .get(field)
                .ok_or("exact installed capture slot")?
                .get()
                .to_owned()
        };
        let capture: RetainedHtmlCaptureV1 = serde_json::from_str(&raw)?;
        check!(eq; capture.body()?.as_bytes(), original);
        let json = reference(raw.as_bytes())?;
        let body = reference(original)?;
        for (kind, object) in [
            (RetainedCaptureLeafKindV1::CaptureJson, json.clone()),
            (RetainedCaptureLeafKindV1::OriginalBody, body.clone()),
        ] {
            self.objects
                .insert(Material::Retained(RetainedCaptureLeafV1 {
                    version: 1,
                    root: root.clone(),
                    slot,
                    kind,
                    object,
                }))?;
        }
        self.captures.push(Capture {
            key,
            capture,
            json,
            body,
            original_json: raw.into_bytes(),
            original_body: original.to_vec(),
        });
        Ok(())
    }

    fn unavailable_roster(&mut self) -> TestResult {
        check!(eq; hash(DCC_ROSTER), "f09ce2ccb8e9e789c4f7e782f286d95d5387aefd28996327a40a0aa82054a324");
        check!(eq; hash(DCC_HEADERS), "98c18e5df4725d2825db7da293ac97f298e44e465054dbd360ddfd0867451b90");
        let capture = RetainedHtmlCaptureV1::new(
            DCC_URL,
            DCC_URL,
            200,
            instant("2026-10-16T00:08:22Z")?,
            DCC_ROSTER,
            RetainedHttpProvenanceV1 {
                request_method: "GET".into(),
                request_headers: vec![
                    ("User-Agent".into(), "OpenAI-research/0.1".into()),
                    ("Accept".into(), "*/*".into()),
                ],
                response_header_bytes: DCC_HEADERS.to_vec(),
                content_type: "text/html; charset=utf-8".into(),
                body_encoding: RetainedBodyEncodingV1::Identity,
                started_at: instant("2026-10-16T00:08:18Z")?,
            },
        )?;
        let bytes = serialized_bytes(&capture)?;
        let json = reference(&bytes)?;
        let body = reference(DCC_ROSTER)?;
        self.objects.bytes(bytes.clone())?;
        self.objects.bytes(DCC_ROSTER.to_vec())?;
        self.captures.push(Capture {
            key: "dcc-roster",
            capture,
            json,
            body,
            original_json: bytes,
            original_body: DCC_ROSTER.to_vec(),
        });
        Ok(())
    }

    pub(super) fn get(&self, key: &str) -> TestResult<&Capture> {
        Ok(self
            .captures
            .iter()
            .find(|capture| capture.key == key)
            .ok_or("source capture key")?)
    }
}

pub(super) fn instant(value: &str) -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
