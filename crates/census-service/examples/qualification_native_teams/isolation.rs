use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::net::ToSocketAddrs;
use std::path::Path;

use super::artifacts::{read_bounded, write_json, write_new};
use super::process::command;

pub const FAULT_ADDRESS: &str = "1.1.1.1";

pub fn prepare(root: &Path) -> Result<Value> {
    let network = std::fs::read_link("/proc/self/ns/net")?;
    let host_pid: u32 = std::env::var("QUALIFICATION_HOST_PID")
        .context("pass the live pre-unshare launcher PID in QUALIFICATION_HOST_PID")?
        .parse()?;
    ensure!(
        host_pid > 1 && host_pid != std::process::id(),
        "host launcher PID must be a distinct live process"
    );
    let host_network = std::fs::read_link("/proc/self/fd/3")
        .context("inherit the pre-unshare network namespace descriptor at fd 3")?;
    let mount = std::fs::read_link("/proc/self/ns/mnt")?;
    let host_mount = std::fs::read_link("/proc/self/fd/4")
        .context("inherit the pre-unshare mount namespace descriptor at fd 4")?;
    ensure!(
        host_network.to_string_lossy().starts_with("net:[")
            && host_mount.to_string_lossy().starts_with("mnt:["),
        "inherited host capabilities are not namespace descriptors"
    );
    ensure!(network != host_network && mount != host_mount, "launch through unshare --user --map-root-user --net --mount --fork --kill-child=TERM; host namespaces are forbidden");
    let interfaces = namespace_interfaces(root)?;
    ensure!(
        interfaces.len() == 1 && interfaces.first().is_some_and(|name| name == "lo"),
        "namespace is not loopback-only: {interfaces:?}"
    );
    let routes_before = routes(root, "before")?;
    let private = make_private(root)?;
    install_hosts(root)?;
    install_nss(root)?;
    install_fault_address(root)?;
    let routes_after = routes(root, "after")?;
    let addresses = ("oh.milesplit.com", 443)
        .to_socket_addrs()?
        .collect::<Vec<_>>();
    ensure!(
        !addresses.is_empty()
            && addresses
                .iter()
                .all(|address| address.ip().to_string() == FAULT_ADDRESS),
        "private source resolution differs from isolated fault route: {addresses:?}"
    );
    let evidence = json!({"network_namespace":network, "host_network_namespace":host_network, "host_launcher_pid":host_pid, "mount_namespace":mount, "host_mount_namespace":host_mount, "interfaces":interfaces, "routes_before_address_assignment":routes_before, "routes_after_address_assignment":routes_after, "recursive_private_mount_exit":0, "recursive_private_mount_output":private, "hosts_bound_only_after_private_mount":true, "source_resolution":addresses.iter().map(ToString::to_string).collect::<Vec<_>>(), "source_host":"oh.milesplit.com", "fault_address":FAULT_ADDRESS, "address_semantics":"synthetic transport-routing fault address on namespace-only lo; NOT authentic MileSplit DNS/source IP/source body", "guard_authorization":"none; unmodified public-address destination guard applies normally", "external_network_interfaces":0, "external_gateway_or_default_route":false, "host_hosts_modified":false});
    write_json(&root.join("isolation.json"), &evidence)?;
    Ok(evidence)
}

fn make_private(root: &Path) -> Result<String> {
    let output = command(
        root,
        "mount-private",
        Path::new("/usr/bin/mount"),
        &["--make-rprivate".into(), "/".into()],
    )?;
    let mountinfo = read_bounded(Path::new("/proc/self/mountinfo"))?;
    ensure!(
        !mountinfo
            .split_whitespace()
            .any(|word| word.starts_with("shared:")),
        "namespace mount propagation remains shared"
    );
    write_new(&root.join("mountinfo-private.txt"), mountinfo.as_bytes())?;
    Ok(output)
}

fn install_hosts(root: &Path) -> Result<()> {
    let hosts = root.join("namespace-hosts");
    let original = read_bounded(Path::new("/etc/hosts"))?;
    write_new(&root.join("original-hosts.txt"), original.as_bytes())?;
    let isolated = original
        .lines()
        .filter(|line| {
            !line
                .split_whitespace()
                .any(|word| word == "oh.milesplit.com")
        })
        .collect::<Vec<_>>()
        .join("\n");
    write_new(
        &hosts,
        format!("{isolated}\n{FAULT_ADDRESS} oh.milesplit.com\n").as_bytes(),
    )?;
    command(
        root,
        "mount-private-hosts",
        Path::new("/usr/bin/mount"),
        &[
            "--bind".into(),
            hosts.display().to_string(),
            "/etc/hosts".into(),
        ],
    )?;
    Ok(())
}

fn install_nss(root: &Path) -> Result<()> {
    let original = read_bounded(Path::new("/etc/nsswitch.conf"))?;
    write_new(&root.join("original-nsswitch.conf"), original.as_bytes())?;
    let is_hosts = |line: &str| {
        line.split_once(':')
            .is_some_and(|(key, _)| key.trim() == "hosts")
    };
    ensure!(
        original.lines().filter(|line| is_hosts(line)).count() == 1,
        "NSS must have exactly one hosts policy"
    );
    let isolated = original
        .lines()
        .map(|line| if is_hosts(line) { "hosts: files" } else { line })
        .collect::<Vec<_>>()
        .join("\n");
    let nss = root.join("namespace-nsswitch.conf");
    write_new(&nss, format!("{isolated}\n").as_bytes())?;
    command(
        root,
        "mount-private-nss",
        Path::new("/usr/bin/mount"),
        &[
            "--bind".into(),
            nss.display().to_string(),
            "/etc/nsswitch.conf".into(),
        ],
    )?;
    Ok(())
}

fn install_fault_address(root: &Path) -> Result<()> {
    command(
        root,
        "fault-loopback-address",
        Path::new("/usr/bin/ip"),
        &[
            "address".into(),
            "add".into(),
            format!("{FAULT_ADDRESS}/32"),
            "dev".into(),
            "lo".into(),
        ],
    )?;
    command(
        root,
        "loopback-up",
        Path::new("/usr/bin/ip"),
        &["link".into(), "set".into(), "lo".into(), "up".into()],
    )?;
    Ok(())
}

fn namespace_interfaces(root: &Path) -> Result<Vec<String>> {
    let links = command(
        root,
        "namespace-interfaces",
        Path::new("/usr/bin/ip"),
        &["-j".into(), "link".into(), "show".into()],
    )?;
    serde_json::from_str::<Value>(&links)?
        .as_array()
        .context("ip link JSON is not an array")?
        .iter()
        .map(|link| {
            link.get("ifname")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .context("network interface name absent")
        })
        .collect()
}

fn routes(root: &Path, label: &str) -> Result<Value> {
    let ipv4 = command(
        root,
        &format!("routes-{label}-ipv4"),
        Path::new("/usr/bin/ip"),
        &[
            "-j".into(),
            "route".into(),
            "show".into(),
            "table".into(),
            "all".into(),
        ],
    )?;
    let ipv6 = command(
        root,
        &format!("routes-{label}-ipv6"),
        Path::new("/usr/bin/ip"),
        &[
            "-j".into(),
            "-6".into(),
            "route".into(),
            "show".into(),
            "table".into(),
            "all".into(),
        ],
    )?;
    let routes = [
        serde_json::from_str::<Value>(&ipv4)?,
        serde_json::from_str::<Value>(&ipv6)?,
    ];
    routes.iter().try_for_each(|table| -> Result<()> {
        let entries = table
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("ip route JSON is not an array"))?;
        ensure!(
            entries.iter().all(|entry| entry.get("gateway").is_none()
                && entry.get("dst").and_then(Value::as_str) != Some("default")
                && entry.get("dev").and_then(Value::as_str) == Some("lo")),
            "namespace has a non-loopback route or gateway: {table}"
        );
        Ok(())
    })?;
    Ok(json!(routes))
}
