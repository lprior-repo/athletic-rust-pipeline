#[cfg(test)]
mod tests_inner {
    use super::super::parse_robots;
    use std::time::Duration;

    #[test]
    fn a_star_group_crawl_delay_becomes_pacing() {
        let policy = parse_robots("User-agent: *\nCrawl-delay: 2\n");
        assert_eq!(policy.crawl_delay, Some(Duration::from_secs(2)));
    }

    #[test]
    fn a_named_agent_group_does_not_pace_us() {
        let policy = parse_robots("User-agent: GPTBot\nCrawl-delay: 30\n");
        assert_eq!(policy.crawl_delay, None);
        let with_star = parse_robots(
            "User-agent: GPTBot\nCrawl-delay: 30\n\nUser-agent: *\nCrawl-delay: 2\n",
        );
        assert_eq!(with_star.crawl_delay, Some(Duration::from_secs(2)));
    }

    #[test]
    fn disallow_lines_publish_no_refusal_and_no_pacing_of_their_own() {
        let bare = parse_robots("User-agent: *\nDisallow: /\n");
        assert_eq!(bare.crawl_delay, None);
        let mixed =
            parse_robots("User-agent: *\nDisallow: /api/\nDisallow: /*directory\nCrawl-delay: 3\n");
        assert_eq!(mixed.crawl_delay, Some(Duration::from_secs(3)));
    }

    #[test]
    fn rules_outside_a_group_publish_nothing() {
        let policy = parse_robots("Disallow: /x\nCrawl-delay: 5\n");
        assert_eq!(policy.crawl_delay, None);
    }

    #[test]
    fn a_hostile_crawl_delay_is_refused_or_clamped() {
        for hostile in ["inf", "-inf", "nan", "1e30", "1e300", "-5"] {
            let body = format!("User-agent: *\nCrawl-delay: {hostile}\n");
            let policy = parse_robots(&body);
            assert!(
                policy
                    .crawl_delay
                    .is_none_or(|delay| delay <= Duration::from_secs(3600)),
                "{hostile} must be refused or clamped, got {:?}",
                policy.crawl_delay
            );
        }
        let clamped = parse_robots("User-agent: *\nCrawl-delay: 1e9\n");
        assert_eq!(clamped.crawl_delay, Some(Duration::from_secs(3600)));
    }

    #[test]
    fn comments_and_unknown_fields_publish_only_the_delay() {
        let policy = parse_robots(
            "# rules\nUser-agent: *\nSitemap: https://x/sitemap.xml\nCrawl-delay: 4 # seconds\n",
        );
        assert_eq!(policy.crawl_delay, Some(Duration::from_secs(4)));
        let comment_only = parse_robots("# just a comment\n  \n");
        assert_eq!(comment_only.crawl_delay, None);
    }
}
