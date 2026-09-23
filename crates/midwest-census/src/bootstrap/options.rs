//! The `midwest-serve` command line: the parsed surface and the parser that refuses typos.

use std::ffi::OsStr;
use std::iter::Peekable;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::BrowserSettings;
use url::Url;

use super::error::BootstrapError;
use super::{DEFAULT_DRAIN_TIMEOUT, DEFAULT_MAX_CONCURRENT};

/// The origin the headed profile reads. Compiled in: a profile that may navigate anywhere is not the
/// one this lane is for, and the census's own `RequestSpec` addresses this same site.
const LANE_ORIGIN: &str = "https://www.athletic.net";

/// The browser a deployment gets when it enables the lane without naming one. A deployment with a
/// different Chromium build passes `--browser-executable`.
const DEFAULT_BROWSER_EXECUTABLE: &str = "chromium";

/// How long one page may take, and how long a challenge interstitial may be waited out before the
/// engine reports it. Both are the engine's own ceilings for a source read, not the run's.
const LANE_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const LANE_CHALLENGE_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    pub max_concurrent: usize,
    pub drain_timeout: Duration,
    /// The headed browser lane, when the deployment serves one: `--browser-profile` enables it.
    /// `None` leaves the endpoint without a `BrowserSession` object, which is what a deployment
    /// that never reads the source through a profile wants.
    pub lane: Option<BrowserSettings>,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from(([127, 0, 0, 1], 9080)),
            data_dir: PathBuf::from("var/midwest-census"),
            max_concurrent: DEFAULT_MAX_CONCURRENT,
            drain_timeout: DEFAULT_DRAIN_TIMEOUT,
            lane: None,
        }
    }
}

impl ServeOptions {
    /// Parse `--flag value` pairs. Unknown flags are rejected instead of ignored: a typo in a
    /// deployment script must not silently serve the wrong directory.
    ///
    /// The lane's three flags are read into locals and assembled after the loop, so
    /// `--browser-executable` or `--browser-headless` without `--browser-profile` is refused rather
    /// than silently configuring a lane the deployment does not serve.
    pub fn from_env(args: impl Iterator<Item = String>) -> Result<Self, BootstrapError> {
        Self::from_env_with_path(args, std::env::var_os("PATH").as_deref())
    }

    /// `from_env` with the environment's `PATH` passed in, because which browser the lane's default
    /// resolves to is an environment fact: a test that read the process's own `PATH` would assert
    /// about the machine it runs on rather than about the parser.
    pub(super) fn from_env_with_path(
        args: impl Iterator<Item = String>,
        path: Option<&OsStr>,
    ) -> Result<Self, BootstrapError> {
        let mut options = ServeOptions::default();
        let mut lane_flags = LaneFlags::default();
        let mut args = args.peekable();
        while let Some(flag) = args.next() {
            apply_flag(&flag, &mut args, &mut options, &mut lane_flags)?;
        }
        lane_flags.install(&mut options, path)?;
        Ok(options)
    }
}
/// The lane's three flags, held apart from the options until the loop ends so a half-configured lane
/// is refused rather than installed.
#[derive(Default)]
struct LaneFlags {
    profile: Option<PathBuf>,
    executable: Option<PathBuf>,
    headless: bool,
}

impl LaneFlags {
    /// Refuse a lane flag without a profile, then build the settings the deployment serves.
    fn install(
        self,
        options: &mut ServeOptions,
        path: Option<&OsStr>,
    ) -> Result<(), BootstrapError> {
        if self.profile.is_none() && (self.executable.is_some() || self.headless) {
            return Err(BootstrapError::LaneFlagWithoutProfile {
                flag: if self.executable.is_some() {
                    "--browser-executable"
                } else {
                    "--browser-headless"
                }
                .to_string(),
            });
        }
        if let Some(profile_dir) = self.profile {
            options.lane = Some(lane(profile_dir, self.executable, self.headless, path)?);
        }
        Ok(())
    }
}

/// Apply one flag: a value flag reads its own argument here, everything else sets its field.
fn apply_flag<I: Iterator<Item = String>>(
    flag: &str,
    args: &mut Peekable<I>,
    options: &mut ServeOptions,
    lane: &mut LaneFlags,
) -> Result<(), BootstrapError> {
    let mut value = |flag: &str| -> Result<String, BootstrapError> {
        args.next().ok_or_else(|| BootstrapError::MissingValue {
            flag: flag.to_string(),
        })
    };
    match flag {
        "--listen" => {
            let raw = value("--listen")?;
            options.listen = raw
                .parse()
                .map_err(|source| BootstrapError::ListenNotAnAddress { raw, source })?;
        }
        "--data-dir" => options.data_dir = PathBuf::from(value("--data-dir")?),
        "--max-concurrent" => {
            let raw = value("--max-concurrent")?;
            let parsed: usize = raw
                .parse()
                .map_err(|source| BootstrapError::ConcurrencyNotANumber { raw, source })?;
            if parsed == 0 {
                return Err(BootstrapError::ConcurrencyIsZero);
            }
            options.max_concurrent = parsed;
        }
        "--drain-timeout" => {
            let raw = value("--drain-timeout")?;
            let seconds: u64 = raw
                .parse()
                .map_err(|source| BootstrapError::DrainTimeoutNotASeconds { raw, source })?;
            options.drain_timeout = Duration::from_secs(seconds);
        }
        "--browser-profile" => lane.profile = Some(PathBuf::from(value("--browser-profile")?)),
        "--browser-executable" => {
            lane.executable = Some(PathBuf::from(value("--browser-executable")?))
        }
        "--browser-headless" => lane.headless = true,
        "--help" | "-h" => return Err(BootstrapError::HelpRequested),
        other => {
            return Err(BootstrapError::UnknownFlag {
                flag: other.to_string(),
            })
        }
    }
    Ok(())
}

/// The lane's settings once its flags are known: the headed profile the source expects, two tabs
/// pooled behind it, and the engine's own timeouts.
fn lane(
    profile_dir: PathBuf,
    executable: Option<PathBuf>,
    headless: bool,
    path: Option<&OsStr>,
) -> Result<BrowserSettings, BootstrapError> {
    Ok(BrowserSettings {
        cdp_endpoint: None,
        executable: match executable {
            Some(executable) => executable,
            None => browser_on_path(path).ok_or(BootstrapError::LaneBrowserNotOnPath {
                program: DEFAULT_BROWSER_EXECUTABLE.to_string(),
            })?,
        },
        profile_dir,
        source_origin: lane_origin()?,
        tabs: 2,
        request_timeout: LANE_REQUEST_TIMEOUT,
        challenge_wait: LANE_CHALLENGE_WAIT,
        headed: !headless,
    })
}

/// The default browser, resolved against `PATH`. The lane refuses a relative executable - the
/// browser would resolve one against a working directory this process does not control - so the
/// default is looked up rather than handed through, and an entry that is itself relative is no
/// candidate.
fn browser_on_path(path: Option<&OsStr>) -> Option<PathBuf> {
    std::env::split_paths(path?)
        .map(|dir| dir.join(DEFAULT_BROWSER_EXECUTABLE))
        .find(|candidate| candidate.is_absolute() && candidate.is_file())
}

/// The lane's origin, parsed once: the failure is a build that cannot read its own constant.
fn lane_origin() -> Result<Url, BootstrapError> {
    Url::parse(LANE_ORIGIN).map_err(|source| BootstrapError::LaneOriginUnusable {
        origin: LANE_ORIGIN.to_string(),
        source,
    })
}
