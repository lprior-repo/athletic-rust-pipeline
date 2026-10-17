use super::*;
use crate::milesplit::parse::markup::Markup;
use html5gum::Tokenizer;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn declaration_offsets_follow_original_bytes_after_utf8_entities_and_crlf() -> TestResult {
    let prefix = "é雪\r\n&amp;<!-- <link rel=canonical href=decoy> -->";
    let declaration = "<meta property='og&#58;url' content='https://wi.milesplit.com/teams/9001/roster'>";
    let html = format!("{prefix}{declaration}tail");
    let tokens = Tokenizer::new_with_emitter(html.as_str(), Markup::default())
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; tokens.len(), 1);
    let token = tokens.first().ok_or("located declaration")?;
    check!(matches!(token.kind, DeclarationKind::OpenGraph));
    let end = prefix.len().checked_add(declaration.len()).ok_or("offset")?;
    check!(eq; token.span.clone(), Some(prefix.len()..end));
    check!(eq; html.get(token.span.clone().ok_or("span")?), Some(declaration));
    check!(matches!(&token.declaration, Declaration::Url(bytes)
        if bytes == b"https://wi.milesplit.com/teams/9001/roster"));
    Ok(())
}

#[test]
fn raw_text_and_inert_decoys_do_not_produce_located_declarations() -> TestResult {
    let decoy = "<link rel='canonical' href='https://wi.milesplit.com/teams/9002/roster'>";
    for prefix in [
        format!("<!-- {decoy} -->"),
        format!("<div data-text=\"{decoy}\"></div>"),
        format!("<template>{decoy}</template>"),
        format!("<svg>{decoy}</svg>"),
        format!("<math>{decoy}</math>"),
        format!("<style>{decoy}</style>"),
        format!("<noscript>{decoy}</noscript>"),
        format!("<textarea>{decoy}</textarea>"),
        format!("<script>{decoy}</script>"),
    ] {
        let real = "<link rel='canonical' href='https://wi.milesplit.com/teams/9001/roster'>";
        let html = format!("{prefix}{real}");
        let tokens = Tokenizer::new_with_emitter(html.as_str(), Markup::default())
            .collect::<Result<Vec<_>, _>>()?;
        check!(eq; tokens.len(), 1);
        let token = tokens.first().ok_or("real owner")?;
        check!(eq; token.span.clone(), Some(prefix.len()..html.len()));
    }
    Ok(())
}

#[test]
fn json_ld_locations_cover_actual_element_or_exact_eof() -> TestResult {
    let prefix = "é\r\n<!-- prefix -->";
    let opening = "<script type='application/ld+json'>";
    let body = "{\r\n\"@type\":\"WebPage\",\"url\":\"/teams/9001/roster\"}";
    for (ending, termination) in [
        ("</script>", Termination::ScriptEnd),
        ("", Termination::ScriptEof),
    ] {
        let html = format!("{prefix}{opening}{body}{ending}");
        let tokens = Tokenizer::new_with_emitter(html.as_str(), Markup::default())
            .collect::<Result<Vec<_>, _>>()?;
        check!(eq; tokens.len(), 1);
        let token = tokens.first().ok_or("script")?;
        check!(eq; token.span.clone(), Some(prefix.len()..html.len()));
        check!(eq; token.termination, termination);
        check!(matches!(&token.declaration, Declaration::Script(bytes)
            if bytes == body.replace("\r\n", "\n").as_bytes()));
    }
    Ok(())
}
