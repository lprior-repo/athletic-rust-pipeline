use super::*;

#[test]
fn selected_owner_paths_require_exact_uuid_label_pid_and_owned_log() -> Result<()> {
    let generation = "12345678-abcd-1234-abcd-123456789abc";
    let mut owner = json!({"endpoint":{"label":"serve","pid":3,"generation":generation,"log":format!("{GUEST}/serve-{generation}.log")},
        "node":{"label":"restate","pid":4,"generation":generation,"log":format!("{GUEST}/restate-{generation}.log")}});
    assert_eq!(selected_specs(&owner)?.len(), 4);
    *owner
        .get_mut("endpoint")
        .and_then(|value| value.get_mut("log"))
        .context("endpoint log absent")? = json!("/root/.ssh/id_ed25519");
    assert!(selected_specs(&owner).is_err());
    assert!(!valid_generation("../../../../../../etc/shadow"));
    assert!(!valid_generation("12345678-abcd-1234-abcd-123456789ab/"));
    Ok(())
}

#[test]
fn raw_tails_preserve_bytes_and_refuse_over_cap_or_bad_framing() -> Result<()> {
    let root = tempfile::tempdir()?;
    let spec = Spec::file("http", format!("{GUEST}/http.jsonl"), 4);
    let retained = retain(
        root.path(),
        "before-drain",
        &spec,
        "http\tOK\t19\tAP8KDQ==\t0",
    )?;
    assert_eq!(
        std::fs::read(root.path().join("diagnostics-before-drain-http.bin"))?,
        [0, 255, 10, 13]
    );
    assert_eq!(retained.get("tail_may_be_partial"), Some(&json!(true)));
    assert!(retain(
        root.path(),
        "after-drain",
        &spec,
        "other\tOK\t4\tAP8KDQ==\t0"
    )
    .is_err());
    assert!(retain(
        root.path(),
        "after-drain",
        &spec,
        "http\tOK\t9\tMDEyMzQ1Njc4\t0"
    )
    .is_err());
    assert!(retain(root.path(), "after-drain", &spec, "http\tOK\t4\t***\t0").is_err());
    let missing = retain(root.path(), "after-drain", &spec, "http\tMISSING\t0\t\t1")?;
    assert!(!all_read(&json!([missing])));
    Ok(())
}
