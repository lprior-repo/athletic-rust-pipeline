use super::*;

#[test]
fn current_public_source_paths_cannot_reintroduce_runtime_robots() -> TestResult {
    let sources = [
        include_str!("../../mod.rs"),
        include_str!("../../execute.rs"),
        include_str!("../../execute/fetch.rs"),
        include_str!("../../execute/attempt.rs"),
        include_str!("../../execute/dispatch.rs"),
        include_str!("../../public_http.rs"),
        include_str!("../../acquisition/transport.rs"),
        include_str!("../../acquisition/public_resources.rs"),
        include_str!("../../acquisition/public_resources/replay.rs"),
        include_str!("../../acquisition/public_resources/roster.rs"),
        include_str!("../../../school_sites/run.rs"),
        include_str!("../../../coach_directories/generic.rs"),
    ];
    for source in sources {
        for forbidden in [
            "mod robots;",
            "robots::",
            "get_robots_checked",
            "raise_robots_delay",
            "texting_robots",
            "\"/robots.txt\"",
        ] {
            check!(!source.contains(forbidden));
        }
    }
    check!(!include_str!("../../../../Cargo.toml").contains("texting_robots"));
    Ok(())
}
