use super::{bare_runs_in, parse_attempts, violations, Grammars, RunGrammars, Site, Sites};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn sites(attribute: u64, builder: u64) -> Sites {
    Sites {
        attribute: vec![Site {
            at: "pkg:src/a.rs:1".to_string(),
            attempts: attribute,
        }],
        builder: vec![Site {
            at: "pkg:src/a.rs:2".to_string(),
            attempts: builder,
        }],
        unclaimed: Vec::new(),
        bare_runs: Vec::new(),
    }
}

fn masked_of(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

#[test]
fn integer_literals_normalise_across_radix_suffix_and_separators() {
    let cases = [
        ("3", Some(3)),
        ("3u32", Some(3)),
        ("0x3", Some(3)),
        ("0b11", Some(3)),
        ("0o3", Some(3)),
        ("1_000", Some(1000)),
        ("70", Some(70)),
        ("", None),
        ("_", None),
        ("n", None),
        ("1+3", None),
    ];
    for (token, expected) in cases {
        assert_eq!(parse_attempts(token), expected, "token {token:?}");
    }
}

#[test]
fn a_site_at_its_ceiling_holds_and_one_above_it_is_named() {
    assert!(violations(&sites(3, 1), 3, 1).is_empty());
    assert!(violations(&sites(2, 1), 3, 1).is_empty());
    let over = violations(&sites(4, 1), 3, 1);
    assert_eq!(over.len(), 1);
    assert!(
        over.first().is_some_and(
            |line| line.contains("pkg:src/a.rs:1") && line.contains("max_attempts = 4")
        ),
        "the failure names the site and its value: {over:?}"
    );
    let over = violations(&sites(3, 4), 3, 1);
    assert_eq!(over.len(), 1);
    assert!(
        over.first()
            .is_some_and(|line| line.contains("RunRetryPolicy")),
        "the inner policy is a class of its own: {over:?}"
    );
}

#[test]
fn the_ceiling_action_option_is_not_an_attempt_site() -> TestResult {
    let grammars = Grammars::compile()?;
    let action = "        on_max_attempts = \"pause\",";
    let mut declared = Sites::default();
    check!(eq; grammars.claim("pkg:src/a.rs:1", action, &mut declared), 0);
    check!(eq; grammars.mention.find_iter(action).count(), 0);
    let attempts = "        max_attempts = 3,";
    check!(eq; grammars.claim("pkg:src/a.rs:2", attempts, &mut declared), 1);
    check!(eq; declared.attribute.len(), 1);
    check!(
        declared.builder.is_empty() && declared.unclaimed.is_empty(),
        "the attempt line is claimed as an attribute site and nothing else: {declared:?}"
    );
    Ok(())
}

#[test]
fn an_unclaimed_mention_fails_closed() {
    let mut declared = sites(3, 1);
    declared.unclaimed.push("pkg:src/a.rs:9".to_string());
    let failures = violations(&declared, 3, 1);
    assert_eq!(failures.len(), 1);
    assert!(
        failures
            .first()
            .is_some_and(|line| line.contains("pkg:src/a.rs:9")),
        "{failures:?}"
    );
}

#[test]
fn a_bare_ctx_run_is_named_and_a_chained_policy_holds() -> TestResult {
    let grammars = RunGrammars::compile()?;
    let bare =
        masked_of("    ctx.run(move || jobs::flush_journal(store, entries))\n        .await?;\n");
    let found = bare_runs_in("pkg:src/a.rs", &bare, &grammars);
    check!(eq; found.len(), 1);
    check!(
        found.first().is_some_and(|site| site == "pkg:src/a.rs:1"),
        "the failure names the site: {found:?}"
    );
    let covered = masked_of(
        "        let Json(journaled) = ctx\n                .run(move || jobs::flush_journal(store, entries))\n                .retry_policy(RunRetryPolicy::new().max_attempts(1))\n                .await?;\n",
    );
    check!(
        bare_runs_in("pkg:src/a.rs", &covered, &grammars).is_empty(),
        "a chained policy covers the split-chain spelling the tree uses"
    );
    Ok(())
}

#[test]
fn the_run_scan_leaves_clients_alone_and_never_covers_across_sites() -> TestResult {
    let grammars = RunGrammars::compile()?;
    let client = masked_of(
        "                client\n                    .run(Json(request.for_jurisdiction(*jurisdiction)))\n                    .call(),\n",
    );
    check!(
        bare_runs_in("pkg:src/a.rs", &client, &grammars).is_empty(),
        "the object client's run is not a ctx.run effect"
    );
    let two = masked_of(
        "        let a = ctx\n            .run(move || step_a(store))\n            .await?;\n        let b = ctx\n            .run(move || step_b(store))\n            .retry_policy(RunRetryPolicy::new().max_attempts(1))\n            .await?;\n",
    );
    let found = bare_runs_in("pkg:src/a.rs", &two, &grammars);
    check!(eq; found.len(), 1);
    check!(
        found.first().is_some_and(|site| site == "pkg:src/a.rs:1"),
        "one effect's policy never covers another's absence: {found:?}"
    );
    Ok(())
}

#[test]
fn a_bare_run_fails_the_violations_alongside_ceilings() {
    let mut declared = sites(3, 1);
    declared.bare_runs.push("pkg:src/a.rs:7".to_string());
    let failures = violations(&declared, 3, 1);
    assert_eq!(failures.len(), 1);
    assert!(
        failures
            .first()
            .is_some_and(|line| line.contains("pkg:src/a.rs:7") && line.contains("ctx.run")),
        "the absence gate fails through the same violations all contract check 3 reads: \
         {failures:?}"
    );
}
