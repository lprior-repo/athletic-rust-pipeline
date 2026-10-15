use super::*;

#[tokio::test]
async fn manifest_mutation_cannot_reuse_a_valid_archive_key() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let original = fixture.payload(fixture.raw.manifest_key())?;
    for (field, value) in [
        ("bytes", json!(1)),
        ("fetched_at", json!("2020-01-01T00:00:00Z")),
        ("method", json!("POST")),
        ("status", json!(403)),
        ("content_digest", json!("0".repeat(64))),
    ] {
        let mut payload = original.clone();
        payload
            .get_mut("capture")
            .and_then(Value::as_object_mut)
            .ok_or("capture")?
            .insert(field.into(), value);
        fixture.replace(fixture.raw.manifest_key(), &payload)?;
        check!(fixture.read().is_err(), "{field}");
    }
    fixture.replace(fixture.raw.manifest_key(), &original)?;
    check!(fixture.read().is_ok());
    Ok(())
}

#[tokio::test]
async fn chunk_bytes_metadata_index_and_body_budget_are_verified() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let prefix = fixture
        .raw
        .manifest_key()
        .strip_suffix("/manifest")
        .ok_or("prefix")?;
    let key = format!("{prefix}/0");
    let original = fixture.payload(&key)?;
    for (field, value) in [
        ("capture", json!({})),
        ("chunk_index", json!(1)),
        ("source_instance", json!("milesplit_oh")),
        ("role", json!("owned_result")),
        (
            "raw_base64",
            json!("A".repeat(FEMALE_RAW.len().div_ceil(3) * 4)),
        ),
    ] {
        let mut payload = original.clone();
        payload
            .as_object_mut()
            .ok_or("chunk")?
            .insert(field.into(), value);
        fixture.replace(&key, &payload)?;
        check!(fixture.read().is_err(), "{field}");
    }
    fixture.replace(&key, &original)?;
    let view = fixture.store.snapshot();
    let owned = read_retained_owned_result(&view, &fixture.owned, "data[0]", budget()?)?;
    check!(read_retained_result_metadata(
        &view,
        &fixture.reference,
        &fixture.raw,
        &owned,
        CaptureReadBudget::new(1)?
    )
    .is_err());
    Ok(())
}

#[tokio::test]
async fn canonical_published_final_url_and_result_set_must_name_the_owned_source() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let foreign = "https://al.milesplit.com/meets/725218/results/1266815/raw";
    let redirected = fixture.archive(FEMALE_RAW, Some(foreign))?;
    check!(fixture.read_archive(&redirected).is_err());
    let changed = std::str::from_utf8(FEMALE_RAW)?.replace("1266814/raw", "1266815/raw");
    let archive = fixture.archive(changed.as_bytes(), None)?;
    check!(fixture.read_archive(&archive).is_err());
    let mut reference = fixture.reference.clone();
    reference.rsid = "1266815".into();
    reference.url = foreign.into();
    let view = fixture.store.snapshot();
    let owned = read_retained_owned_result(&view, &fixture.owned, "data[0]", budget()?)?;
    check!(matches!(
        read_retained_result_metadata(&view, &reference, &fixture.raw, &owned, budget()?),
        Err(RetainedMetadataError::ForeignResult)
    ));
    Ok(())
}

#[tokio::test]
async fn pinned_raw_and_owned_sources_ignore_later_corruption_without_reading_latest() -> TestResult
{
    let fixture = Fixture::prepare(TROY).await?;
    let pinned = fixture.store.snapshot();
    let owned = read_retained_owned_result(&pinned, &fixture.owned, "data[0]", budget()?)?;
    let prefix = fixture
        .raw
        .manifest_key()
        .strip_suffix("/manifest")
        .ok_or("prefix")?;
    fixture.replace(&format!("{prefix}/0"), &json!({"forged": true}))?;
    let source = read_retained_result_metadata(
        &pinned,
        &fixture.reference,
        &fixture.raw,
        &owned,
        budget()?,
    )?;
    check!(eq; source.snapshot_sequence(), pinned.sequence());
    check!(fixture.read().is_err());
    Ok(())
}
