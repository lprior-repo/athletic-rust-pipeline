use super::*;

const DATE: &str = "2026-10-02";
const BOOT: &str = "same-guest-boot";
const MACHINE: &str = "same-guest-machine";

fn before(boot: &str, machine: &str) -> Value {
    json!({"boot_id": boot, "machine_id": machine})
}

fn after(date: &str, realtime: &str, boot: &str, machine: &str) -> Value {
    json!({"date": date, "realtime": realtime, "boot_id": boot, "machine_id": machine})
}

#[test]
fn injection_accepts_only_the_half_open_ten_second_target_window() {
    let cases = [
        ("2026-10-02T23:55:00Z", Ok(())),
        ("2026-10-02T23:54:59.999999999Z", Err(())),
        ("2026-10-02T23:55:09.999999999Z", Ok(())),
        ("2026-10-02T23:55:10Z", Err(())),
    ];
    cases.into_iter().for_each(|(realtime, expected)| {
        let before = before(BOOT, MACHINE);
        let after = after(DATE, realtime, BOOT, MACHINE);
        let result = validate_set_clock(&before, &after, DATE);
        assert_eq!(result.map_err(|_| ()), expected, "realtime {realtime}");
    });
}

#[test]
fn injection_uses_the_utc_instant_instead_of_the_local_date_and_time() {
    let cases = [
        ("2026-10-03T01:55:05+02:00", Ok(())),
        ("2026-10-02T18:55:05-05:00", Ok(())),
        ("2026-10-02T23:55:05+02:00", Err(())),
        ("2026-10-03T01:55:10+02:00", Err(())),
    ];
    cases.into_iter().for_each(|(realtime, expected)| {
        let before = before(BOOT, MACHINE);
        let after = after(DATE, realtime, BOOT, MACHINE);
        let result = validate_set_clock(&before, &after, DATE);
        assert_eq!(result.map_err(|_| ()), expected, "realtime {realtime}");
    });
}

#[test]
fn injection_rejects_disagreement_between_reported_date_and_utc_realtime() {
    let cases = [
        ("2026-10-03", "2026-10-02T23:55:05Z"),
        (DATE, "2026-10-03T23:55:05Z"),
    ];
    cases.into_iter().for_each(|(date, realtime)| {
        let before = before(BOOT, MACHINE);
        let after = after(date, realtime, BOOT, MACHINE);
        let result = validate_set_clock(&before, &after, DATE);
        assert_eq!(
            result.map_err(|_| ()),
            Err(()),
            "reported date {date}, realtime {realtime}"
        );
    });
}

#[test]
fn injection_rejects_a_different_machine_even_when_the_boot_is_unchanged() {
    let before = before(BOOT, MACHINE);
    let after = after(DATE, "2026-10-02T23:55:05Z", BOOT, "other-guest-machine");
    let result = validate_set_clock(&before, &after, DATE);
    assert_eq!(result.map_err(|_| ()), Err(()));
}

#[test]
fn injection_requires_nonempty_boot_and_machine_identifiers_on_both_sides() {
    let cases = [
        ("before boot empty", "", MACHINE, BOOT, MACHINE),
        ("after boot empty", BOOT, MACHINE, "", MACHINE),
        ("both boots empty", "", MACHINE, "", MACHINE),
        ("before machine empty", BOOT, "", BOOT, MACHINE),
        ("after machine empty", BOOT, MACHINE, BOOT, ""),
        ("both machines empty", BOOT, "", BOOT, ""),
    ];
    cases.into_iter().for_each(
        |(label, before_boot, before_machine, after_boot, after_machine)| {
            let before = before(before_boot, before_machine);
            let after = after(DATE, "2026-10-02T23:55:05Z", after_boot, after_machine);
            let result = validate_set_clock(&before, &after, DATE);
            assert_eq!(result.map_err(|_| ()), Err(()), "{label}");
        },
    );
}

#[test]
fn masked_synchronization_is_rejected_while_running_or_transitioning() {
    for active in ["active", "activating", "deactivating", "reloading"] {
        let state = format!("LoadState=masked\nActiveState={active}\n");
        assert!(validate_synchronization_state(&state, "systemd-timesyncd.service").is_err());
    }
    for state in [
        "LoadState=masked\nActiveState=inactive\n",
        "ActiveState=failed\nLoadState=masked\n",
    ] {
        assert!(validate_synchronization_state(state, "systemd-timesyncd.service").is_ok());
    }
}

#[test]
fn synchronization_requires_complete_unambiguous_masked_unit_state() {
    for state in [
        "LoadState=loaded\nActiveState=inactive\n",
        "LoadState=not-found\nActiveState=inactive\n",
        "LoadState=masked\n",
        "ActiveState=inactive\n",
        "LoadState=masked\nLoadState=masked\n",
        "LoadState=masked\nActiveState=inactive\nActiveState=active\n",
    ] {
        assert!(validate_synchronization_state(state, "systemd-timesyncd.service").is_err());
    }
}
