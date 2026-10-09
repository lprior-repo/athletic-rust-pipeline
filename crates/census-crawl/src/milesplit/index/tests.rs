use super::{validate_surface, CrawlError};

const URL: &str = "https://tx.milesplit.com/results/2026/outdoor?page=1";

#[test]
fn accepts_decoded_authoritative_provider_declaration() -> Result<(), Box<dyn std::error::Error>> {
    validate_surface("<html><head><META NAME='application-name' content='Mile&#83;plit'></head><body>Results</body></html>", URL)?;
    Ok(())
}

#[test]
fn rejects_provider_claims_inside_non_authoritative_markup() {
    let marker = "<meta name='application-name' content='MileSplit'>";
    for (open, close) in [
        ("<!--", "-->"),
        ("<script>", "</script>"),
        ("<style>", "</style>"),
        ("<template>", "</template>"),
        ("<svg>", "</svg>"),
        ("<math>", "</math>"),
        ("<textarea>", "</textarea>"),
        ("<noscript>", "</noscript>"),
    ] {
        let body =
            format!("<html><head>{open}{marker}{close}</head><body>Other provider</body></html>");
        assert!(matches!(
            validate_surface(&body, URL),
            Err(CrawlError::Schema { .. })
        ));
    }
}

#[test]
fn rejects_conflicting_provider_declarations_and_foreign_authorities() {
    let body = "<meta name='application-name' content='MileSplit'><meta name='application-name' content='Other'>";
    assert!(matches!(
        validate_surface(body, URL),
        Err(CrawlError::Schema { .. })
    ));
    let owned = "<meta name='application-name' content='MileSplit'>";
    for url in [
        "https://milesplit.com.evil.example/results",
        "https://evil.milesplit.com/results",
        "https://tx.milesplit.com:9000/results",
        "https://operator@tx.milesplit.com/results",
    ] {
        assert!(matches!(
            validate_surface(owned, url),
            Err(CrawlError::Schema { .. })
        ));
    }
}

#[test]
fn rejects_a_valid_marker_hidden_beyond_the_representation_node_budget() {
    let body = format!(
        "{}<meta name='application-name' content='MileSplit'>",
        "<div>".repeat(65_537)
    );
    assert!(matches!(
        validate_surface(&body, URL),
        Err(CrawlError::Resource { .. })
    ));
}

#[test]
fn accepts_a_rebranded_state_index_that_loads_network_assets(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = "<html><head><meta name='application-name' content='AlabamaRunners'><link rel='icon' href='https://assets.sp.milesplit.com/images/milesplit-favicon.png'><script src='https://js.sp.milesplit.com/drivefaze/api.js'></script></head><body>Teams</body></html>";
    validate_surface(body, "https://al.milesplit.com/teams")?;
    Ok(())
}

#[test]
fn rejects_a_rebranded_index_without_network_assets() {
    let body = "<html><head><meta name='application-name' content='AlabamaRunners'><link rel='icon' href='https://assets.sp.example.com/images/favicon.png'></head><body>Teams</body></html>";
    assert!(matches!(
        validate_surface(body, "https://al.milesplit.com/teams"),
        Err(CrawlError::Schema { .. })
    ));
}

#[test]
fn rejects_network_asset_claims_on_foreign_or_relative_targets() {
    for href in [
        "https://milesplit.com.evil.example/favicon.png",
        "https://assets.milesplit.com.evil.example/favicon.png",
        "https://operator@assets.sp.milesplit.com/favicon.png",
        "https://assets.sp.milesplit.com:9000/favicon.png",
        "/images/milesplit-favicon.png",
    ] {
        let body = format!("<html><head><link href='{href}'></head><body>Teams</body></html>");
        assert!(matches!(
            validate_surface(&body, "https://al.milesplit.com/teams"),
            Err(CrawlError::Schema { .. })
        ));
    }
}

#[test]
fn rejects_network_assets_inside_non_authoritative_markup() {
    let marker = "<link href='https://assets.sp.milesplit.com/favicon.png'>";
    for (open, close) in [
        ("<!--", "-->"),
        ("<script>", "</script>"),
        ("<style>", "</style>"),
        ("<template>", "</template>"),
        ("<svg>", "</svg>"),
        ("<math>", "</math>"),
        ("<textarea>", "</textarea>"),
        ("<noscript>", "</noscript>"),
    ] {
        let body = format!("<html><head>{open}{marker}{close}</head><body>Teams</body></html>");
        assert!(matches!(
            validate_surface(&body, "https://al.milesplit.com/teams"),
            Err(CrawlError::Schema { .. })
        ));
    }
}
