use crate::{config::RetrievalConfig, discovery::allowed_profile_url};
use anyhow::{anyhow, bail, Context, Result};
use spider::page::Page;
use spider::website::Website;
use std::{fs, path::Path, time::Duration};

fn stop_after_seed_page(_: &Page) -> bool {
    false
}

pub fn load_saved_profile(url: &str, directory: &Path) -> Result<Option<String>> {
    let normalized = allowed_profile_url(url).context("candidate URL is outside the allow-list")?;
    let parsed = url::Url::parse(&normalized)?;
    let identifier = parsed.path_segments().and_then(|segments| {
        let mut after_athlete = segments.skip_while(|segment| *segment != "athlete");
        after_athlete.next()?;
        after_athlete.next()
    });
    let Some(identifier) = identifier else {
        return Ok(None);
    };
    if identifier.is_empty()
        || !identifier
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Ok(None);
    }
    let path = directory.join(format!("{identifier}.html"));
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(fs::read_to_string(&path).with_context(|| {
        format!("reading manually saved profile {}", path.display())
    })?))
}

pub async fn fetch_exact_profile(url: &str, config: &RetrievalConfig) -> Result<String> {
    if !config.authorized_direct_fetch {
        bail!("direct retrieval is disabled");
    }
    let normalized = allowed_profile_url(url).context("candidate URL is outside the allow-list")?;
    let mut website = Website::new(&normalized)
        .with_limit(1)
        .with_delay(config.delay_ms)
        .with_respect_robots_txt(config.respect_robots_txt)
        .with_user_agent(Some(config.user_agent.as_str()))
        .with_on_should_crawl_callback(Some(stop_after_seed_page))
        .build()
        .map_err(|_| anyhow!("failed to build Spider website configuration"))?;
    let mut receiver = website.subscribe(4);

    let mut collector = tokio::spawn(async move {
        let mut body = None;
        while let Ok(page) = receiver.recv().await {
            if allowed_profile_url(page.get_url()).is_none() {
                continue;
            }
            body = Some(page.get_html_bytes_u8().to_vec());
        }
        body
    });

    website.crawl().await;
    website.unsubscribe();
    let bytes = match tokio::time::timeout(Duration::from_secs(30), &mut collector).await {
        Ok(result) => result
            .context("Spider collector task failed")?
            .context("Spider returned no allowed profile page")?,
        Err(_) => {
            collector.abort();
            let _join_result = collector.await;
            bail!("timed out waiting for Spider page stream");
        }
    };
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
