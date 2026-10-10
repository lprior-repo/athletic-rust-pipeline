use std::ffi::OsStr;
use std::iter::Peekable;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::BrowserSettings;
use url::Url;

use super::error::BootstrapError;
use super::{
    DEFAULT_DRAIN_TIMEOUT, DEFAULT_MAX_CONCURRENT, DEFAULT_MEMORY_BUDGET_BYTES,
    MAX_CONCURRENT_CEILING, MEMORY_BUDGET_CEILING_GIB,
};

const LANE_ORIGIN: &str = "https://www.athletic.net";

const DEFAULT_BROWSER_EXECUTABLE: &str = "chromium";

const LANE_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const LANE_CHALLENGE_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    pub max_concurrent: usize,
    pub drain_timeout: Duration,
    pub memory_budget_bytes: u64,
    pub lane: Option<BrowserSettings>,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from(([127, 0, 0, 1], 9080)),
            data_dir: PathBuf::from("var/census-service"),
            max_concurrent: DEFAULT_MAX_CONCURRENT,
            drain_timeout: DEFAULT_DRAIN_TIMEOUT,
            memory_budget_bytes: DEFAULT_MEMORY_BUDGET_BYTES,
            lane: None,
        }
    }
}

impl ServeOptions {
    pub fn from_env(args: impl Iterator<Item = String>) -> Result<Self, BootstrapError> {
        Self::from_env_with_path(args, std::env::var_os("PATH").as_deref())
    }

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
#[derive(Default)]
struct LaneFlags {
    profile: Option<PathBuf>,
    executable: Option<PathBuf>,
    headless: bool,
}

impl LaneFlags {
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
            if parsed > MAX_CONCURRENT_CEILING {
                return Err(BootstrapError::ConcurrencyTooLarge {
                    value: parsed,
                    ceiling: MAX_CONCURRENT_CEILING,
                });
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
        "--memory-budget-gib" => {
            let raw = value("--memory-budget-gib")?;
            let gib: u64 = raw
                .parse()
                .map_err(|source| BootstrapError::MemoryBudgetNotAGib { raw, source })?;
            if gib == 0 {
                return Err(BootstrapError::MemoryBudgetIsZero);
            }
            if gib > MEMORY_BUDGET_CEILING_GIB {
                return Err(BootstrapError::MemoryBudgetTooLarge {
                    value: gib,
                    ceiling: MEMORY_BUDGET_CEILING_GIB,
                });
            }
            options.memory_budget_bytes = gib * 1024 * 1024 * 1024;
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

fn lane(
    profile_dir: PathBuf,
    executable: Option<PathBuf>,
    headless: bool,
    path: Option<&OsStr>,
) -> Result<BrowserSettings, BootstrapError> {
    let profile_dir = absolute(profile_dir)?;
    Ok(BrowserSettings {
        cdp_endpoint: None,
        executable: match executable {
            Some(executable) => absolute(executable)?,
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

fn absolute(path: PathBuf) -> Result<PathBuf, BootstrapError> {
    std::path::absolute(&path)
        .map_err(|source| BootstrapError::LanePathUnresolvable { path, source })
}

fn browser_on_path(path: Option<&OsStr>) -> Option<PathBuf> {
    std::env::split_paths(path?)
        .map(|dir| dir.join(DEFAULT_BROWSER_EXECUTABLE))
        .find(|candidate| candidate.is_absolute() && candidate.is_file())
}
fn lane_origin() -> Result<Url, BootstrapError> {
    #[cfg(feature = "native-fault-injection")]
    if let Ok(override_origin) = std::env::var("CENSUS_BROWSER_SOURCE_ORIGIN") {
        if !override_origin.is_empty() {
            return Url::parse(&override_origin).map_err(|source| {
                BootstrapError::LaneOriginUnusable {
                    origin: override_origin,
                    source,
                }
            });
        }
    }
    Url::parse(LANE_ORIGIN).map_err(|source| BootstrapError::LaneOriginUnusable {
        origin: LANE_ORIGIN.to_string(),
        source,
    })
}
