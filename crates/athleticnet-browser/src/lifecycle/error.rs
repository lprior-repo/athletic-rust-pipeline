#[derive(Debug, thiserror::Error)]
pub enum BrowserStartupError {
    #[error("browser profile directory must be a real directory")]
    ProfileNotDirectory,
    #[error("browser profile directory is accessible by other users")]
    ProfilePermissions,
    #[error("browser tab count must be between 1 and 8")]
    InvalidTabCount,
    #[error("browser executable and profile paths must be absolute")]
    InvalidPaths,
    #[error("browser source origin must be an HTTP URL with a host")]
    InvalidOrigin,
    #[error("browser timeouts must be positive")]
    InvalidTimeout,
    #[error("CDP endpoint must use http or https scheme")]
    CdpBadScheme,
    #[error("CDP endpoint must have a host")]
    CdpNoHost,
    #[error("CDP endpoint must resolve to loopback only")]
    CdpNotLoopback,
    #[error("CDP endpoint must have an explicit port")]
    CdpNoPort,
    #[error("CDP endpoint path must be root (/)")]
    CdpBadPath,
    #[error("CDP endpoint must have no credentials")]
    CdpHasCredentials,
    #[error("CDP endpoint must have no query or fragment")]
    CdpHasQueryFragment,
    #[error("browser launch configuration failed")]
    ConfigBuildFailed,
    #[error("browser launch failed: {0}")]
    LaunchFailed(String),
    #[error("browser connection failed: {0}")]
    ConnectFailed(String),
    #[error("browser bootstrap failed")]
    BootstrapFailed,
    #[error("browser actor stopped during startup")]
    ActorStopped,
    #[error("browser handler stopped")]
    HandlerStopped,
    #[error("browser handler stopped; cleanup failed: {0}")]
    HandlerStoppedCleanupFailed(String),
    #[error("browser shutdown failed: {0}")]
    ShutdownFailed(String),
}
