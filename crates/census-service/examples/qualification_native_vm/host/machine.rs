use super::super::{artifacts, process, Host};
use anyhow::{ensure, Result};
use serde_json::json;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn qmp_socket(root: &Path) -> Result<PathBuf> {
    let socket = root.join("qmp.sock");
    let bytes = socket.as_os_str().as_bytes().len();
    ensure!(
        bytes < 108,
        "QEMU UNIX socket path too long: {bytes} bytes; Linux sockaddr_un.sun_path requires at most 107 pathname bytes plus NUL: {}",
        socket.display()
    );
    Ok(socket)
}

pub(super) fn key(root: &Path) -> Result<String> {
    process::command(
        root,
        "ssh-keygen",
        Command::new("/usr/bin/ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-f"])
            .arg(root.join("ssh-key")),
        100,
    )?;
    Ok(String::from_utf8(artifacts::read(
        &root.join("ssh-key.pub"),
    )?)?)
}

fn tools(config: &Host, command: &str) -> Result<Command> {
    let prefix = std::fs::canonicalize(config.tools.join("prefix"))?;
    let mut binary = Command::new(prefix.join(format!("usr/bin/{command}")));
    binary.env("LD_LIBRARY_PATH", prefix.join("usr/lib"));
    Ok(binary)
}

pub(super) fn create_disks(config: &Host, root: &Path) -> Result<()> {
    let version = process::command(
        root,
        "qemu-version",
        tools(config, "qemu-system-x86_64")?.arg("--version"),
        100,
    )?;
    ensure!(
        version.contains("11.1.1"),
        "qualification requires actual supplied QEMU 11.1.1: {version}"
    );
    let image = std::fs::canonicalize(&config.base_image)?;
    process::command(
        root,
        "root-overlay",
        tools(config, "qemu-img")?
            .args(["create", "-f", "qcow2", "-F", "qcow2", "-b"])
            .arg(&image)
            .arg(root.join("root.qcow2")),
        300,
    )?;
    process::command(
        root,
        "data-disk",
        tools(config, "qemu-img")?
            .args(["create", "-f", "qcow2"])
            .arg(root.join("data.qcow2"))
            .arg("8G"),
        300,
    )?;
    artifacts::publish(
        &root.join("disks.json"),
        &json!({"base":image,"root_overlay":root.join("root.qcow2"),"data":root.join("data.qcow2"),"capacity":"8 GiB","cache":"directsync","acceleration":"TCG only"}),
    )
}

pub(super) fn qemu(config: &Host, root: &Path, ssh: u16, seed: u16) -> Result<Command> {
    let socket = qmp_socket(root)?;
    let firmware = std::fs::canonicalize(config.tools.join("prefix/usr/share/qemu"))?;
    let mut command = tools(config, "qemu-system-x86_64")?;
    command
        .args([
            "-machine",
            "pc",
            "-accel",
            "tcg,thread=multi",
            "-cpu",
            "max",
            "-smp",
            "2",
            "-m",
            "4096",
            "-display",
            "none",
            "-monitor",
            "none",
            "-L",
        ])
        .arg(&firmware)
        .arg("-bios")
        .arg(firmware.join("bios-256k.bin"))
        .args([
            "-serial",
            "stdio",
            "-qmp",
            &format!("unix:{},server=on,wait=off", socket.display()),
        ]);
    ["root.qcow2", "data.qcow2"].into_iter().for_each(|disk| {
        command.args([
            "-drive",
            &format!(
                "file={},if=virtio,format=qcow2,cache=directsync,aio=threads",
                root.join(disk).display()
            ),
        ]);
    });
    command.args([
        "-netdev",
        &format!("user,id=bootstrap,hostfwd=tcp:127.0.0.1:{ssh}-:22"),
        "-device",
        "virtio-net-pci,netdev=bootstrap",
        "-smbios",
        &format!("type=1,serial=ds=nocloud;s=http://10.0.2.2:{seed}/"),
        "-rtc",
        "base=utc,clock=rt",
    ]);
    Ok(command)
}
