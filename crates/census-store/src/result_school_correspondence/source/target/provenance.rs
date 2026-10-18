use super::*;
use census_domain::model::{Evidence, EvidenceMethod};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap};

type Fields<'a> = BTreeMap<&'a str, &'a RawValue>;

pub(super) fn require(
    school: &CanonicalSchool,
    root: &crate::RetainedCaptureRootV1,
    provider: &crate::evidence_subject_attachment::external::CheckedSchool<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if root.schema != crate::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1 {
        return Err(Error::SourceMismatch("provider packet root identity").into());
    }
    let mut seen = (false, false);
    school.evidence.iter().try_for_each(|evidence| {
        grant.charge_steps(1).map_err(grant_error)?;
        let Some(fields) = relevant(evidence, root, grant)? else {
            return Ok(());
        };
        match text(&fields, "role")?.as_ref() {
            "directory_name_city" => {
                require_capture(evidence, &fields, &provider.directory, grant)?;
                seen.0 = true;
            }
            "provider_roster_owner" => {
                require_capture(evidence, &fields, &provider.roster, grant)?;
                seen.1 = true;
            }
            _ => return Err(Error::SourceMismatch("provider evidence role").into()),
        }
        Ok::<_, StoreError>(())
    })?;
    if seen != (true, true) {
        return Err(Error::SourceMismatch("provider packet evidence absent").into());
    }
    Ok(())
}

fn relevant<'a>(
    evidence: &'a Evidence,
    root: &crate::RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<Fields<'a>>> {
    let Some(note) = evidence.note.as_deref() else {
        return Ok(None);
    };
    if note.len() > MAX_RESULT_SCHOOL_RECORD_BYTES {
        return Err(Error::BudgetExceeded.into());
    }
    if !note.trim_start().starts_with('{') {
        return Ok(None);
    }
    grant.charge_bytes(note.len()).map_err(grant_error)?;
    let fields: Fields<'_> = match serde_json::from_str(note) {
        Ok(fields) => fields,
        Err(_) if note.contains(root.schema.phase()) || note.contains(&root.key) => {
            return Err(Error::Corrupt("school provider provenance JSON").into())
        }
        Err(_) => return Ok(None),
    };
    let phase = optional_text(&fields, "phase");
    let packet = optional_text(&fields, "packet");
    let phase_matches = matches!(&phase, Ok(Some(value)) if value.as_ref() == root.schema.phase());
    let packet_matches = matches!(&packet, Ok(Some(value)) if value.as_ref() == root.key);
    if !phase_matches && !packet_matches {
        return Ok(None);
    }
    if phase?.as_deref() != Some(root.schema.phase()) {
        return Err(Error::SourceMismatch("provider evidence phase").into());
    }
    let packet = packet?.ok_or(Error::Corrupt("provider evidence packet absent"))?;
    if packet.as_ref() != root.key {
        return Ok(None);
    }
    Ok(Some(fields))
}

fn require_capture(
    evidence: &Evidence,
    provenance: &Fields<'_>,
    capture: &crate::evidence_subject_attachment::external::CheckedCapture<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let claimed = text(provenance, "capture_sha256")?;
    let url = capture.requested_url(grant)?;
    if evidence.method != EvidenceMethod::Parsed
        || claimed != capture.body_sha256
        || evidence.source.url.as_deref() != Some(url.as_ref())
    {
        return Err(Error::SourceMismatch("provider evidence capture binding").into());
    }
    Ok(())
}

fn text<'a>(fields: &Fields<'a>, field: &str) -> StoreResult<Cow<'a, str>> {
    optional_text(fields, field)?
        .ok_or_else(|| Error::Corrupt("provider provenance field absent").into())
}

fn optional_text<'a>(fields: &Fields<'a>, field: &str) -> StoreResult<Option<Cow<'a, str>>> {
    fields.get(field).map(|value| raw_text(value)).transpose()
}

fn raw_text(raw: &RawValue) -> StoreResult<Cow<'_, str>> {
    let inner = raw
        .get()
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .ok_or(Error::Corrupt("provider provenance field type"))?;
    if inner.contains('\\') {
        serde_json::from_str(raw.get())
            .map(Cow::Owned)
            .map_err(|_| Error::Corrupt("provider provenance string").into())
    } else {
        Ok(Cow::Borrowed(inner))
    }
}
