use super::super::markup;
use super::{bound, invalid, MhsaaOriginalSpanV1, MHSAA_CLASSIFICATION_SOURCE_URL};
use census_store::StoreResult;
use html5gum::{DefaultEmitter, Token, Tokenizer};

const TITLE: &str = "MHSAA 2026-27 School Year Classifications Announced";
const NATIVE_PATH: &str = "\"currentPath\":\"node\\/28899\"";
const PARAGRAPH: &str = "<p class=\"MsoNormal\">Among spring sports teams currently competing, Pickford’s boys track &amp; field team won the last three Upper Peninsula Division 2 titles and will move to Division 3 for 2027 after competing in Division 2 again this spring, while Newberry – winners of the last two UP Division 3 titles – will move into Division 2 in 2027 after competing in Division 3 this season. The Pickford and Newberry girls teams, both coming off Finals championships last spring as well, will make the same switches for 2027.&nbsp;Powers North Central boys golf, which won the last two UP Division 3 titles, will compete in Division 3 again this spring but move to Division 2 next year.</p>";
const REVISION: &str = "<p class=\"MsoNormal\">For 2026-27,&nbsp;there are 755 tournament-qualified member schools. Schools recently were notified of their&nbsp;classification, and sport-by-sport divisions were posted to the MHSAA Website today (April 7).&nbsp;MHSAA Executive Director Mark Uyl said a school may not subsequently lower its enrollment figure.&nbsp;However, if a revised enrollment figure is higher and indicates that a school should be playing in a higher division, that school would be moved up.</p>";
const OPT_UP: &str = "<p class=\"MsoNormal\">Schools also may request to play in a higher classification or division in a sport for a minimum of two years. Requests to opt up in fall sports for 2026-27 must be submitted by May 1, winter sports by Aug. 15 and spring sports by Oct. 15</p>";

pub(super) struct Document<'a> {
    pub(super) article: MhsaaOriginalSpanV1<'a>,
    pub(super) heading: MhsaaOriginalSpanV1<'a>,
    pub(super) published_label: MhsaaOriginalSpanV1<'a>,
    pub(super) native_path: MhsaaOriginalSpanV1<'a>,
    pub(super) school_year_label: MhsaaOriginalSpanV1<'a>,
    pub(super) paragraph: MhsaaOriginalSpanV1<'a>,
    pub(super) revision_notice: MhsaaOriginalSpanV1<'a>,
    pub(super) opt_up_notice: MhsaaOriginalSpanV1<'a>,
}

pub(super) fn read(html: &str) -> StoreResult<Document<'_>> {
    markup::canonical(html, MHSAA_CLASSIFICATION_SOURCE_URL)?;
    let heading = markup::single(markup::elements(html, b"h1", None)?)?;
    if markup::text(heading.html)? != TITLE {
        return Err(invalid("MHSAA announcement heading differs"));
    }
    let article = markup::single(markup::elements(html, b"article", Some("node__news"))?)?;
    article_shape(article.html)?;
    let footer = markup::single(markup::elements(article.html, b"footer", None)?)?;
    let published = markup::single(markup::elements(
        footer.html,
        b"span",
        Some("text--uppercase"),
    )?)?;
    if markup::text(published.html)? != "April 7, 2026" {
        return Err(invalid("MHSAA announcement publication label differs"));
    }
    let article_span = MhsaaOriginalSpanV1::new(html, article.span)?;
    let heading = MhsaaOriginalSpanV1::new(html, heading.span)?;
    let published_label = article_span.one(published.html)?.one("April 7, 2026")?;
    let paragraph = selected_paragraph(&article_span, PARAGRAPH)?;
    let revision_notice = selected_paragraph(&article_span, REVISION)?;
    let opt_up_notice = selected_paragraph(&article_span, OPT_UP)?;
    let school_year_label = heading.one("2026-27")?;
    Ok(Document {
        article: article_span,
        heading,
        published_label,
        native_path: native_path(html)?,
        school_year_label,
        paragraph,
        revision_notice,
        opt_up_notice,
    })
}

fn selected_paragraph<'a>(
    article: &MhsaaOriginalSpanV1<'a>,
    expected: &str,
) -> StoreResult<MhsaaOriginalSpanV1<'a>> {
    let mut selected = None;
    for paragraph in markup::elements(article.raw(), b"p", Some("MsoNormal"))? {
        if paragraph.html == expected && selected.replace(paragraph.html).is_some() {
            return Err(invalid("MHSAA announcement paragraph is ambiguous"));
        }
    }
    article.one(selected.ok_or_else(|| invalid("MHSAA announcement paragraph is absent"))?)
}

fn article_shape(html: &str) -> StoreResult<()> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut active = markup::Active::default();
    let mut count = 0usize;
    for (index, token) in Tokenizer::new_with_emitter(html, emitter)
        .take(100_001)
        .enumerate()
    {
        bound(index, 99_999, "MHSAA article tokens")?;
        let token = token.map_err(|_| invalid("MHSAA article token failure"))?;
        if active.accept(&token)?
            && matches!(&token, Token::StartTag(tag) if tag.name.as_slice() == b"article")
        {
            count = count
                .checked_add(1)
                .ok_or(census_store::StoreError::CounterOverflow)?;
        }
    }
    active.finish()?;
    if count != 1 {
        return Err(invalid("MHSAA announcement nested article differs"));
    }
    Ok(())
}

fn native_path(html: &str) -> StoreResult<MhsaaOriginalSpanV1<'_>> {
    let mut selected = None;
    for script in markup::elements(html, b"script", None)? {
        if script.attribute("data-drupal-selector") == Some("drupal-settings-json") {
            if selected.is_some() || script.attribute("type") != Some("application/json") {
                return Err(invalid("MHSAA native article identity is ambiguous"));
            }
            bound(script.html.len(), 16 * 1024, "MHSAA settings bytes")?;
            validate_path(script.html)?;
            selected = Some(MhsaaOriginalSpanV1::new(html, script.span)?.one(NATIVE_PATH)?);
        }
    }
    selected.ok_or_else(|| invalid("MHSAA native article identity is absent"))
}

fn validate_path(script: &str) -> StoreResult<()> {
    #[derive(serde::Deserialize)]
    struct Settings {
        path: Path,
    }
    #[derive(serde::Deserialize)]
    struct Path {
        #[serde(rename = "currentPath")]
        current: String,
    }
    let (_, content) = script
        .split_once('>')
        .ok_or_else(|| invalid("MHSAA settings open tag differs"))?;
    let json = content
        .strip_suffix("</script>")
        .ok_or_else(|| invalid("MHSAA settings close tag differs"))?;
    let settings: Settings =
        serde_json::from_str(json).map_err(|_| invalid("MHSAA native article settings invalid"))?;
    if settings.path.current != "node/28899" {
        return Err(invalid("MHSAA native article path differs"));
    }
    Ok(())
}
