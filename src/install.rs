//! `poet install` / `poet uninstall`: systemd user service, `poet` symlink, KDE menu entry.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const SERVICE: &str = "poetautopublish.service";

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn service_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home().join(".config"))
        .join("systemd/user")
        .join(SERVICE)
}

fn desktop_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| home().join(".local/share"))
        .join("applications/poetautopublish.desktop")
}

fn bin_dir() -> PathBuf {
    home().join(".local/bin")
}

fn systemctl(args: &[&str]) -> Result<String> {
    let out = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .context("running systemctl")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn service_active() -> bool {
    systemctl(&["is-active", SERVICE])
        .map(|s| s == "active")
        .unwrap_or(false)
}

pub fn service(action: &str) -> Result<()> {
    systemctl(&[action, SERVICE])?;
    Ok(())
}

pub fn install() -> Result<Vec<String>> {
    let mut done = vec![];
    let exe = std::env::current_exe()?.canonicalize()?;

    let svc = service_path();
    fs::create_dir_all(svc.parent().unwrap())?;
    fs::write(
        &svc,
        format!(
            "[Unit]\nDescription=poetautopublish: rebuild poem PDFs when poems change\n\n\
             [Service]\nExecStart={} watch\nRestart=on-failure\nRestartSec=5\n\n\
             [Install]\nWantedBy=default.target\n",
            exe.display()
        ),
    )?;
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", "--now", SERVICE])?;
    systemctl(&["restart", SERVICE])?;
    done.push(format!("service: {} (enabled)", svc.display()));

    let link = bin_dir().join("poet");
    fs::create_dir_all(bin_dir())?;
    if link.symlink_metadata().is_ok() {
        fs::remove_file(&link)?;
    }
    std::os::unix::fs::symlink(&exe, &link)?;
    done.push(format!("command: {} -> {}", link.display(), exe.display()));

    let desktop = desktop_path();
    fs::create_dir_all(desktop.parent().unwrap())?;
    fs::write(
        &desktop,
        format!(
            "[Desktop Entry]\nType=Application\nName=poetautopublish\nComment=Write and publish poems\n\
             Exec={exe}\nTerminal=true\nIcon=accessories-text-editor\nCategories=Office;\n\
             Actions=new;\n\n[Desktop Action new]\nName=New poem\nExec={exe} new\n",
            exe = exe.display()
        ),
    )?;
    let _ = Command::new("update-desktop-database")
        .arg(desktop.parent().unwrap())
        .output();
    done.push(format!("menu entry: {}", desktop.display()));
    Ok(done)
}

pub fn uninstall() -> Result<Vec<String>> {
    let mut done = vec![];
    let _ = systemctl(&["disable", "--now", SERVICE]);
    for p in [service_path(), bin_dir().join("poet"), desktop_path()] {
        if p.symlink_metadata().is_ok() {
            fs::remove_file(&p)?;
            done.push(format!("removed {}", p.display()));
        }
    }
    let _ = systemctl(&["daemon-reload"]);
    Ok(done)
}
