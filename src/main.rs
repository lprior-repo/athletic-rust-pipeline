#![forbid(unsafe_code)]

mod cli;

use anyhow::{Context, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let telemetry = init_tracing()?;
    let outcome = cli::run().await;
    telemetry.shutdown();
    outcome
}

/// Install the process-wide tracing stack and return the handle for its optional telemetry.
///
/// The default stack is one registry, the `RUST_LOG`-driven [`EnvFilter`], and the stderr `fmt`
/// layer: the same filter parsing, the same writer and the same single-subscriber `try_init` error
/// as before, now composed as layers so an optional sink can be added without replacing the
/// subscriber. Nothing is sent anywhere but stderr unless one of the features below is enabled.
///
/// Optional layers (both off in every default build; the feature comments in `Cargo.toml` are the
/// other half of this documentation):
///
/// * `tokio-console` layers `console_subscriber::spawn()` under the same filter and fmt layer.
///   Tokio emits its task spans only under `cfg(all(tokio_unstable, feature = "tracing"))` (its
///   `cfg_trace!` macro) and `console_subscriber::spawn()` panics in a build without the flag, so
///   the layer is installed only under `RUSTFLAGS="--cfg tokio_unstable"`; without it this feature
///   compiles, starts no console server, and notes the skip at `debug`.
/// * `otlp` adds an OpenTelemetry tracing layer that exports the same spans over OTLP. Endpoint,
///   protocol and headers come from the standard `OTEL_EXPORTER_OTLP_*` environment variables, and
///   spans are batched, so [`Telemetry::shutdown`] flushes them before `main` returns. Runs that
///   leave through `clap` (`--help`, `--version`, a usage error) end the process inside
///   `Cli::parse()`, before `main` can flush, so those runs export nothing.
fn init_tracing() -> Result<Telemetry> {
    let subscriber = tracing_subscriber::registry()
        .with(env_filter()?)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr));
    let subscriber = with_console(subscriber)?;
    let (subscriber, telemetry) = with_otlp(subscriber)?;
    subscriber
        .try_init()
        .map_err(|error| anyhow::anyhow!("initializing tracing: {error}"))?;
    Ok(telemetry)
}

/// `RUST_LOG`, parsed exactly as before: a non-Unicode value is a typed error, an invalid filter is
/// rejected, and an absent value means `info`.
fn env_filter() -> Result<EnvFilter> {
    match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::try_new(value).context("invalid RUST_LOG filter"),
        Err(std::env::VarError::NotPresent) => Ok(EnvFilter::new("info")),
        Err(error @ std::env::VarError::NotUnicode(_)) => {
            Err(error).context("RUST_LOG is not Unicode")
        }
    }
}

/// The `tokio-console` layer, or the subscriber unchanged when the feature is off.
#[cfg(not(feature = "tokio-console"))]
fn with_console<S>(subscriber: S) -> Result<S>
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    Ok(subscriber)
}

/// The `tokio-console` layer: task, resource and async-op visibility in the same registry, so the
/// console observes exactly what the stderr layer prints.
#[cfg(all(feature = "tokio-console", tokio_unstable))]
fn with_console<S>(
    subscriber: S,
) -> Result<impl tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>>
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    Ok(subscriber.with(console_subscriber::spawn()))
}

/// The `tokio-console` feature in a build without `RUSTFLAGS="--cfg tokio_unstable"`: the layer is
/// skipped, because `console_subscriber::spawn()` panics in that build and tokio emits no task
/// spans for it to record anyway. The subscriber comes back unchanged and the skip is noted at
/// `debug`, so an operator who wonders why the console is empty has a line to find.
#[cfg(all(feature = "tokio-console", not(tokio_unstable)))]
fn with_console<S>(subscriber: S) -> Result<S>
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    tracing::debug!(
        "tokio-console feature is on but tokio is not built with `--cfg tokio_unstable`"
    );
    Ok(subscriber)
}

/// The OTLP layer and its provider, or the subscriber unchanged when the feature is off.
#[cfg(not(feature = "otlp"))]
fn with_otlp<S>(subscriber: S) -> Result<(S, Telemetry)>
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    Ok((subscriber, Telemetry::default()))
}

/// The OTLP layer, exporting the same spans the stderr layer prints.
#[cfg(feature = "otlp")]
fn with_otlp<S>(
    subscriber: S,
) -> Result<(
    impl tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
    Telemetry,
)>
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    use opentelemetry::trace::TracerProvider as _;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build()
        .context("building the OTLP span exporter")?;
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();
    let tracer = provider.tracer("athletic-rust-pipeline");
    let telemetry = Telemetry {
        provider: Some(provider),
    };
    Ok((
        subscriber.with(tracing_opentelemetry::layer().with_tracer(tracer)),
        telemetry,
    ))
}

/// The optional OTLP tracer provider, flushed and stopped after the command completes.
///
/// A batch exporter owns a background thread and a queue; a process that exits through `main` must
/// shut it down explicitly or the last spans never leave the queue.
#[derive(Default)]
struct Telemetry {
    #[cfg(feature = "otlp")]
    provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl Telemetry {
    /// Flush and stop the OTLP exporter, if one was installed.
    fn shutdown(self) {
        #[cfg(feature = "otlp")]
        if let Some(provider) = &self.provider {
            if let Err(error) = provider.shutdown() {
                tracing::warn!(%error, "stopping the OTLP tracer provider failed");
            }
        }
    }
}
