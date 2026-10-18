use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::Fetcher;
use crate::school_sites::{crawl_site, QueueRow, Rules};
use std::collections::HashMap;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn cache(fetcher: &Fetcher, requested: &str, actual: &str, body: &[u8], status: u16) -> TestResult {
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", requested, ""));
    let meta = CacheMeta {
        url: requested.to_owned(),
        response_url: Some(actual.to_owned()),
        method: "GET".to_owned(),
        status,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: crate::net::now_iso8601(),
        ..CacheMeta::default()
    };
    write_cache(&body_path, &meta_path, body, &meta)?;
    Ok(())
}

#[tokio::test]
async fn redirected_hits_bind_exact_captured_url_without_cross_capture_aliasing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["school.example".to_owned()],
    )?
    .with_offline(true);
    cache(
        &fetcher,
        "https://school.example/robots.txt",
        "https://school.example/robots.txt",
        b"",
        200,
    )?;
    let first = b"<table><tr><th>Sport</th><th>Coach</th><th>E-mail</th></tr><tr><td>Cross Country - Girls</td><td>Mary Ann Lee</td><td>mlee@school.example</td></tr></table><a href=\"/coaches\">Coaches</a>";
    let second = b"<table><tr><th>Sport</th><th>Coach</th><th>E-mail</th></tr><tr><td>Track - Boys</td><td>John Smith</td><td>jsmith@school.example</td></tr></table>";
    cache(
        &fetcher,
        "https://school.example/start",
        "https://school.example/staff",
        first,
        200,
    )?;
    cache(
        &fetcher,
        "https://school.example/coaches",
        "https://school.example/start",
        second,
        200,
    )?;
    let row = QueueRow {
        state: "MI".to_owned(),
        name: "School".to_owned(),
        website: "https://school.example/start".to_owned(),
    };
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    check!(outcome
        .signals
        .coach_hits
        .iter()
        .any(|hit| hit.name == "Mary Ann Lee" && hit.url == "https://school.example/staff"));
    check!(outcome
        .signals
        .coach_hits
        .iter()
        .any(|hit| hit.name == "John Smith" && hit.url == "https://school.example/start"));
    for hit in &outcome.signals.coach_hits {
        check!(outcome.pages.iter().any(|page| page.url == hit.url));
    }
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn lakeview_shaped_navigation_excludes_unrelated_sports_with_spare_budget() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["school.example".to_owned()],
    )?
    .with_offline(true);
    let home = br#"<nav>
        <a href="/activities/athletics/cheerleading">Cheerleading</a>
        <a href="/activities/athletics/football">Football</a>
        <a href="/activities/athletics/dance">Dance</a>
        <a href="/activities/athletics/basketball">Basketball</a>
        <a href="/activities/athletics/baseball">Baseball</a>
        <a href="/activities/athletics/volleyball">Volleyball</a>
        <a href="/cross-country">Cross Country</a>
        <a href="/activities/athletics/track">Track</a>
        <a href="/staff-directory">Staff Directory</a>
        <a href="/activities">Activities</a>
    </nav>"#;
    cache(
        &fetcher,
        "https://school.example/",
        "https://school.example/",
        home,
        200,
    )?;
    for path in [
        "/robots.txt",
        "/cross-country",
        "/activities/athletics/track",
        "/staff-directory",
        "/activities",
        "/activities/athletics/cheerleading",
        "/activities/athletics/football",
        "/activities/athletics/dance",
        "/activities/athletics/basketball",
        "/activities/athletics/baseball",
        "/activities/athletics/volleyball",
    ] {
        let url = format!("https://school.example{path}");
        cache(&fetcher, &url, &url, b"", 200)?;
    }
    let row = QueueRow {
        state: "MN".to_owned(),
        name: "School".to_owned(),
        website: "https://school.example/".to_owned(),
    };
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    let paths: Vec<_> = outcome.pages.iter().map(|page| page.url.as_str()).collect();
    check!(eq; paths, vec![
        "https://school.example/",
        "https://school.example/cross-country",
        "https://school.example/activities/athletics/track",
        "https://school.example/staff-directory",
        "https://school.example/activities",
    ]);
    check!(eq; (outcome.requests, outcome.errors), (5, 0));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn lotus_root_reaches_athletics_before_generic_staff_exhausts_budget() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["lotusschool.org".to_owned()],
    )?
    .with_offline(true);
    let root = "https://lotusschool.org/";
    let director = "https://lotusschool.org/athletics-dpt/";
    let home = br#"<nav>
        <a href="/staff-resources/">Staff Resources</a>
        <a href="/#">Staff</a>
        <a href="/contact/">Contact</a>
        <a href="/staff/mehmet-nalcaci/">Mehmet Nalcaci</a>
        <a href="/staff/john-kara/">John Kara</a>
        <a href="/staff/ermek-bakyt/">Ermek Bakyt</a>
        <a href="/athletics-dpt/">Athletics</a>
    </nav>"#;
    let body = br#"<ul class='profile-list-box'><li>Deon Jackson</li><li>Athletic Director</li><li><a href='mailto:djackson@lotusschool.org'>djackson@lotusschool.org</a></li></ul>"#;
    cache(&fetcher, root, root, home, 200)?;
    cache(&fetcher, director, director, body, 200)?;
    for path in [
        "robots.txt",
        "staff-resources/",
        "contact/",
        "staff/mehmet-nalcaci/",
        "staff/john-kara/",
        "staff/ermek-bakyt/",
    ] {
        let url = format!("{root}{path}");
        cache(&fetcher, &url, &url, b"", 200)?;
    }
    let row = QueueRow {
        state: "CO".to_owned(),
        name: "Lotus School for Excellence".to_owned(),
        website: root.to_owned(),
    };
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    let paths: Vec<_> = outcome.pages.iter().map(|page| page.url.as_str()).collect();
    check!(eq; paths, vec![root, "https://lotusschool.org/contact/", director,
        "https://lotusschool.org/staff-resources/",
        "https://lotusschool.org/staff/mehmet-nalcaci/",
        "https://lotusschool.org/staff/john-kara/",
        "https://lotusschool.org/staff/ermek-bakyt/"]);
    let hit = outcome
        .signals
        .staff_contacts
        .first()
        .ok_or("director candidate")?;
    check!(eq; (hit.name.as_str(), hit.email.as_str()), ("Deon Jackson", "djackson@lotusschool.org"));
    check!(eq; hit.url.as_str(), director);
    check!(outcome
        .pages
        .iter()
        .any(|page| page.url == hit.url && page.digest == content_digest(body)));
    check!(eq; (outcome.requests, outcome.errors), (7, 0));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn fragment_aliases_share_budget_without_rewriting_evidence_or_queries() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["school.example".to_owned()],
    )?
    .with_offline(true);
    let root = "https://school.example/";
    let contact = "https://school.example/contact#faculty";
    let query = "https://school.example/contact?view=all#faculty";
    let home = br#"<nav>
        <a href="/#staff">Staff</a>
        <a href="/contact#faculty">Contact</a>
        <a href="/contact#office">Contact</a>
        <a href="/contact">Contact</a>
        <a href="/contact?view=all#faculty">Contact</a>
        <a href="/contact?view=all#office">Contact</a>
    </nav>"#;
    cache(&fetcher, root, root, home, 200)?;
    for url in ["https://school.example/robots.txt", contact, query] {
        cache(&fetcher, url, url, b"", 200)?;
    }
    let row = QueueRow {
        state: "CO".to_owned(),
        name: "School".to_owned(),
        website: root.to_owned(),
    };
    let outcome = crawl_site(&Rules::new()?, &fetcher, &row, false).await?;
    let urls: Vec<_> = outcome.pages.iter().map(|page| page.url.as_str()).collect();
    check!(eq; urls, vec![root, contact, query]);
    check!(eq; (outcome.requests, outcome.errors), (3, 0));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}
