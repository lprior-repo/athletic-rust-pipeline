use anyhow::{anyhow, Context, Result};
use census_crawl::geocode::{
    GeocodeOutcome, GeocodeQuery, GoogleGeocoder, HttpTransport, SecretKey, UspsValidator,
    ValidationOutcome, ValidationQuery,
};
use census_domain::school_directory::{SchoolDirectoryEntry, SourceLabel};

use super::pipeline::Corpus;
use super::report::PhaseReport;
use super::SchoolAddressArgs;

const GOOGLE_CREDENTIALS: [&str; 2] = ["GOOGLE_MAPS_API_KEY", "GOOGLE_API_KEY"];
const USPS_CREDENTIALS: [&str; 1] = ["USPS_API_TOKEN"];

pub(super) fn apply(args: &SchoolAddressArgs, corpus: &mut Corpus) -> Result<Option<PhaseReport>> {
    if !args.geocode && !args.validate_postal {
        return Ok(None);
    }
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("building the geocode phase runtime")?;
            runtime.block_on(apply_async(args, corpus))
        });
        worker
            .join()
            .map_err(|_| anyhow!("the geocode phase thread panicked"))?
            .map(Some)
    })
}

async fn apply_async(args: &SchoolAddressArgs, corpus: &mut Corpus) -> Result<PhaseReport> {
    let mut report = PhaseReport {
        geocode_requested: args.geocode,
        validation_requested: args.validate_postal,
        ..PhaseReport::default()
    };
    if args.geocode {
        geocode_phase(&mut report, corpus).await?;
    }
    if args.validate_postal {
        validate_phase(&mut report, corpus).await?;
    }
    Ok(report)
}

async fn geocode_phase(report: &mut PhaseReport, corpus: &mut Corpus) -> Result<()> {
    let transport = HttpTransport::new()
        .map_err(|failure| anyhow!("building the HTTP transport failed: {failure}"))?;
    let geocoder = GoogleGeocoder::new(credential(&GOOGLE_CREDENTIALS, "the geocoder")?, transport);
    for entry in corpus.entries.iter_mut() {
        let Some(query) = geocode_query(entry) else {
            report.geocode_skipped = report.geocode_skipped.saturating_add(1);
            continue;
        };
        match geocoder.geocode(&query).await {
            GeocodeOutcome::Found { coordinates, .. } => {
                entry.set_coordinates_from(coordinates, SourceLabel::Geocoder);
                report.geocoded = report.geocoded.saturating_add(1);
            }
            GeocodeOutcome::ZeroResults => {
                report.geocode_empty = report.geocode_empty.saturating_add(1);
            }
            GeocodeOutcome::Unusable { .. } => {
                report.geocode_unusable = report.geocode_unusable.saturating_add(1);
            }
            GeocodeOutcome::Transport { .. } => {
                report.geocode_transport = report.geocode_transport.saturating_add(1);
            }
            GeocodeOutcome::Denied
            | GeocodeOutcome::InvalidRequest
            | GeocodeOutcome::OverQueryLimit
            | GeocodeOutcome::OverDailyLimit
            | GeocodeOutcome::UnknownStatus { .. } => {
                report.geocode_refused = report.geocode_refused.saturating_add(1);
            }
        }
    }
    Ok(())
}

async fn validate_phase(report: &mut PhaseReport, corpus: &Corpus) -> Result<()> {
    let transport = HttpTransport::new()
        .map_err(|failure| anyhow!("building the HTTP transport failed: {failure}"))?;
    let validator = UspsValidator::new(
        credential(&USPS_CREDENTIALS, "the postal validator")?,
        transport,
    );
    for entry in corpus.entries.iter() {
        let Some(query) = validation_query(entry) else {
            report.validation_skipped = report.validation_skipped.saturating_add(1);
            continue;
        };
        match validator.validate(&query).await {
            ValidationOutcome::Validated { .. } => {
                report.validated = report.validated.saturating_add(1);
            }
            ValidationOutcome::Rejected { .. } => {
                report.validation_rejected = report.validation_rejected.saturating_add(1);
            }
            ValidationOutcome::Unparsed { .. } => {
                report.validation_unparsed = report.validation_unparsed.saturating_add(1);
            }
            ValidationOutcome::Transport { .. } => {
                report.validation_transport = report.validation_transport.saturating_add(1);
            }
        }
    }
    Ok(())
}

fn credential(names: &[&'static str], phase: &str) -> Result<SecretKey> {
    SecretKey::from_env_first(names).map_err(|missing| {
        anyhow!(
            "{phase} needs `{}` in the environment: export the credential or drop the flag, \
             because a run that reported geocoded or validated addresses without calling the \
             vendor would be inventing evidence",
            missing.name
        )
    })
}

fn geocode_query(entry: &SchoolDirectoryEntry) -> Option<GeocodeQuery> {
    if entry.coordinates().is_some() {
        return None;
    }
    let address = entry.address()?;
    Some(GeocodeQuery {
        street: address.line1()?.as_str().to_string(),
        city: address.city()?.as_str().to_string(),
        state: address.state()?.code().to_string(),
        postal_code: address.zip().map(|zip| zip.to_string()),
    })
}

fn validation_query(entry: &SchoolDirectoryEntry) -> Option<ValidationQuery> {
    let address = entry.address()?;
    Some(ValidationQuery {
        street: address.line1()?.as_str().to_string(),
        city: address.city()?.as_str().to_string(),
        state: address.state()?.code().to_string(),
        postal_code: address.zip()?.code().to_string(),
    })
}
