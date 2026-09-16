use crate::{config::RetrievalConfig, discovery::allowed_profile_url};
use anyhow::{anyhow, bail, Context, Result};
use spider::page::Page;
use spider::website::Website;
use std::{env, fs::File, io::Read, path::Path, time::Duration};

const MAX_PROFILE_BYTES: usize = 4 * 1024 * 1024;
const SPIDER_MAX_SIZE_BYTES: &str = "SPIDER_MAX_SIZE_BYTES";

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
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("opening manually saved profile {}", path.display()));
        }
    };
    let read_limit = u64::try_from(MAX_PROFILE_BYTES)?
        .checked_add(1)
        .context("saved profile size limit overflow")?;
    let mut html = String::new();
    file.by_ref()
        .take(read_limit)
        .read_to_string(&mut html)
        .with_context(|| format!("reading manually saved profile {}", path.display()))?;
    if html.len() > MAX_PROFILE_BYTES {
        bail!("saved profile exceeds 4 MiB evidence limit");
    }
    Ok(Some(html))
}

fn configured_profile_limit() -> Result<usize> {
    let value = env::var(SPIDER_MAX_SIZE_BYTES).with_context(|| {
        format!("{SPIDER_MAX_SIZE_BYTES} must be set to a value from 1 MiB through 4 MiB")
    })?;
    let limit = value
        .parse::<usize>()
        .with_context(|| format!("{SPIDER_MAX_SIZE_BYTES} must be an integer byte limit"))?;
    if !(1024 * 1024..=MAX_PROFILE_BYTES).contains(&limit) {
        bail!("{SPIDER_MAX_SIZE_BYTES} must be from 1 MiB through 4 MiB");
    }
    Ok(limit)
}

pub async fn fetch_exact_profile(url: &str, config: &RetrievalConfig) -> Result<String> {
    if !config.authorized_direct_fetch {
        bail!("direct retrieval is disabled");
    }
    let configured_limit = configured_profile_limit()?;
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

    let crawl = async {
        Box::pin(website.crawl()).await;
        website.unsubscribe();
    };
    let (_, received) = tokio::time::timeout(
        Duration::from_secs(30),
        Box::pin(async { tokio::join!(crawl, receiver.recv()) }),
    )
    .await
    .context("profile retrieval exceeded 30 seconds")?;
    let page = received.context("Spider returned no profile page")?;
    if page.content_truncated {
        bail!("profile response was truncated before reaching the evidence limit");
    }
    if !page.status_code.is_success() {
        bail!("profile request returned HTTP {}", page.status_code);
    }
    let actual =
        allowed_profile_url(page.get_url()).context("profile redirected outside allow-list")?;
    if actual != normalized {
        bail!("profile redirected to a different athlete URL");
    }
    let bytes = page.get_html_bytes_u8();
    if bytes.len() > configured_limit {
        bail!("profile exceeds configured Spider evidence limit");
    }
    if bytes.len() > MAX_PROFILE_BYTES {
        bail!("profile exceeds 4 MiB evidence limit");
    }
    let html = String::from_utf8(bytes.to_vec()).context("profile is not UTF-8")?;
    let lowered = html.to_ascii_lowercase();
    if lowered.contains("cf-chl-") || lowered.contains("verify you are human") {
        bail!("profile returned an access challenge");
    }
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_saved_profile_is_not_an_input_error() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let result = load_saved_profile(
            "https://www.athletic.net/athlete/123/track-and-field",
            directory.path(),
        )?;
        assert_eq!(result, None);
        Ok(())
    }

    #[test]
    fn oversized_saved_profile_is_rejected_before_returning_html() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("123.html");
        std::fs::write(&path, vec![b'x'; MAX_PROFILE_BYTES + 1])?;
        let error = match load_saved_profile(
            "https://www.athletic.net/athlete/123/track-and-field",
            directory.path(),
        ) {
            Ok(_) => anyhow::bail!("oversized saved profile unexpectedly loaded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("exceeds 4 MiB"));
        Ok(())
    }
}
