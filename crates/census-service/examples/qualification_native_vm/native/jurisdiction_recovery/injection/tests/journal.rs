use super::super::journal::{self, Registration};
use super::{failure, fixture};
use anyhow::{Context, Result};
use serde_json::{json, Value};

fn entries() -> Result<Vec<Value>> {
    let (_, observation) = fixture::active()?;
    Ok(observation
        .children
        .first()
        .context("fixture child absent")?
        .journal
        .as_array()
        .context("fixture journal absent")?
        .clone())
}

#[test]
fn registration_uses_the_original_journal_completion_without_reconstruction() -> Result<()> {
    let entries = entries()?;
    let (identity, references) = journal::registration(&entries)?.context("registration absent")?;
    assert_eq!(
        identity,
        Registration {
            request_digest: "a".repeat(64),
            observed_on: "2026-10-02".to_owned()
        }
    );
    assert_eq!(
        references,
        json!({"command":{"invocation_id":"inv_child","index":1,"version":2},"completion":{"invocation_id":"inv_child","index":2,"version":2}})
    );
    Ok(())
}

#[test]
fn registration_waits_for_matching_completion_and_refuses_duplicate_authority() -> Result<()> {
    let mut entries = entries()?;
    let notification = entries.pop().context("fixture completion absent")?;
    assert!(journal::registration(&entries)?.is_none());
    let mut unrelated = notification.clone();
    unrelated["entry_json"]["Notification"]["Completion"]["Run"]["completion_id"] = json!(2);
    entries.push(unrelated);
    assert!(journal::registration(&entries)?.is_none());
    entries.push(notification.clone());
    assert_eq!(
        journal::registration(&entries)?
            .context("matching completion absent")?
            .0
            .observed_on,
        "2026-10-02"
    );
    entries.push(notification);
    assert!(failure(journal::registration(&entries))?
        .contains("duplicate journaled source registration completion"));
    Ok(())
}

#[test]
fn registration_refuses_nonregistration_first_effect_and_failed_result() -> Result<()> {
    let mut entries = entries()?;
    let command = entries.get_mut(1).context("fixture Run absent")?;
    command["entry_json"] = json!({"Command":{"Sleep":{"completion_id":1}}});
    assert!(failure(journal::registration(&entries))?
        .contains("first source effect is not registration Run"));
    let mut entries = self::entries()?;
    let completion = entries.get_mut(2).context("fixture completion absent")?;
    completion["entry_json"]["Notification"]["Completion"]["Run"]["result"] =
        json!({"Failure":{"code":500}});
    assert!(
        failure(journal::registration(&entries))?.contains("registration did not journal success")
    );
    Ok(())
}

#[test]
fn registration_refuses_malformed_digest_date_and_payload_bytes() -> Result<()> {
    [
        ("not-a-digest", "2026-10-02", "request digest malformed"),
        (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "2026-02-30",
            "observation date malformed",
        ),
    ]
    .into_iter()
    .try_for_each(|(digest, date, diagnostic)| -> Result<()> {
        let mut entries = entries()?;
        *entries.get_mut(2).context("fixture completion absent")? =
            fixture::registration_entry(2, date, digest)?;
        assert!(failure(journal::registration(&entries))?.contains(diagnostic));
        Ok(())
    })?;
    [json!([256]), json!([-1])]
        .into_iter()
        .try_for_each(|payload| -> Result<()> {
            let mut entries = entries()?;
            entries.get_mut(2).context("fixture completion absent")?["entry_json"]
                ["Notification"]["Completion"]["Run"]["result"] = json!({"Success":payload});
            let error = failure(journal::registration(&entries))?;
            assert!(error.contains("out of range") || error.contains("byte malformed"));
            Ok(())
        })
}

#[test]
fn registration_refuses_valid_json_over_four_kib_but_retains_boundary_identity() -> Result<()> {
    let identity = Registration {
        request_digest: "a".repeat(64),
        observed_on: "2026-10-02".to_owned(),
    };
    let mut payload = serde_json::to_vec(&identity)?;
    payload.resize(4096, b' ');
    let mut entries = entries()?;
    entries.get_mut(2).context("fixture completion absent")?["entry_json"]["Notification"]
        ["Completion"]["Run"]["result"] = json!({"Success":payload});
    assert_eq!(
        journal::registration(&entries)?
            .context("boundary registration refused")?
            .0,
        identity
    );
    payload.push(b' ');
    assert_eq!(payload.len(), 4097);
    assert_eq!(serde_json::from_slice::<Registration>(&payload)?, identity);
    entries.get_mut(2).context("fixture completion absent")?["entry_json"]["Notification"]
        ["Completion"]["Run"]["result"] = json!({"Success":payload});
    assert!(journal::registration(&entries).is_err());
    Ok(())
}

#[test]
fn reached_marker_alone_cannot_replace_unjournaled_registration() -> Result<()> {
    let (original, mut observation) = fixture::active()?;
    let directory = fixture::directory()?;
    let config = fixture::configured(directory.path(), &original)?;
    fixture::publish_marker(&config, &fixture::marker(&original)?)?;
    observation
        .children
        .first_mut()
        .context("fixture child absent")?
        .journal
        .as_array_mut()
        .context("fixture journal absent")?
        .truncate(2);
    [&mut observation.before, &mut observation.after]
        .into_iter()
        .try_for_each(|statuses| -> Result<()> {
            let child = statuses
                .as_array_mut()
                .context("fixture statuses absent")?
                .iter_mut()
                .find(|status| status.get("id") == Some(&json!("inv_child")))
                .context("fixture child status absent")?;
            child["journal_size"] = json!(2);
            Ok(())
        })?;
    assert!(fixture::runtime(&config, &original, &observation)?.is_none());
    Ok(())
}
