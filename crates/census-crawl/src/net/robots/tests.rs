#[cfg(test)]
mod tests_inner {
    use super::super::*;

    /// The rule Bound publishes: `Disallow: /*directory` has to refuse a directory path that does
    /// not begin with one, which a prefix matcher would walk straight through.
    #[test]
    fn a_mid_path_wildcard_disallow_refuses_the_match() {
        let rules = parse_robots("User-agent: *\nDisallow: /*directory\n");
        assert!(!rules.allows("/ia/schools/albia/directory/new"));
        assert!(!rules.allows("/directory"));
        assert!(rules.allows("/ia/schools/albia/roster"));
    }

    #[test]
    fn a_trailing_anchor_limits_a_rule_to_one_path() {
        let rules = parse_robots("User-agent: *\nDisallow: /staff$\n");
        assert!(!rules.allows("/staff"));
        assert!(rules.allows("/staff/dana-reid"));
    }

    #[test]
    fn a_wildcard_allow_outranks_an_equally_long_deny() {
        let rules = parse_robots("User-agent: *\nDisallow: /api/*\nAllow: /api/public/\n");
        assert!(!rules.allows("/api/private"));
        assert!(rules.allows("/api/public/teams"));
    }

    #[test]
    fn the_longest_pattern_decides() {
        let rules = parse_robots(
            "User-agent: *\nDisallow: /*calendar*\nAllow: /meets/calendar/2026\nDisallow: /meets/*\n",
        );
        // `/meets/calendar/2026` is matched by the 21-character allow and the 8-character deny.
        assert!(rules.allows("/meets/calendar/2026"));
        // A plain meet path is matched by the deny only.
        assert!(!rules.allows("/meets/regionals"));
    }

    #[test]
    fn a_host_that_refuses_its_robots_file_is_closed_to_us() {
        let rules = parse_robots(REFUSAL_RULES);
        assert!(
            rules.fetched,
            "a refusal is a fetched rule set, not an absent one"
        );
        assert!(!rules.allows("/"));
        assert!(!rules.allows("/ia/schools/adm/directory/new"));
    }

    #[test]
    fn every_pattern_bound_publishes_is_parsed() {
        let rules = parse_robots(
            "User-agent: *\nCrawl-Delay: 10\nDisallow: /api/\nDisallow: /*directory\n\
             Disallow: /profile/*\nDisallow: */athletes/*\nDisallow: /*/leaderlist*\n\
             Disallow: /*/calendar*\n",
        );
        for path in [
            "/api/teams",
            "/ia/schools/albia/directory/new",
            "/profile/123",
            "/ia/athletes/456",
            "/ia/leaderlist/100m",
            "/ia/meets/calendar",
        ] {
            assert!(!rules.allows(path), "{path} should be disallowed");
        }
        assert!(rules.allows("/ia/schools/albia"));
    }
}
