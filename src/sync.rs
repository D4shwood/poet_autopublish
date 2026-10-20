//! Keeps the poems folder in step with a git remote (a private repo), so poems
//! written elsewhere — on the phone — arrive here and get built, and poems
//! written here reach the phone. The built PDFs go to the same remote on their
//! own `pdf` branch, kept as a single commit so the history doesn't grow.
//!
//! Nothing is ever lost: when the same poem changed on both sides, the remote
//! version wins in place and this computer's version is kept next to it as
//! "<name> (masaüstü)".

use crate::config::{Config, data_dir};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What one sync did.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    /// Changes from the remote arrived: the PDFs need a build.
    pub pulled: bool,
    pub pushed: bool,
    /// Poems changed on both sides; each got a "(masaüstü)" copy.
    pub conflicts: Vec<String>,
}

/// This side's name, in commit messages and on conflict copies.
pub const SIDE: &str = "masaüstü";

/// Whether this folder syncs: turned on in the config, and a git repo.
pub fn enabled(cfg: &Config) -> bool {
    cfg.sync && cfg.poems_dir().join(".git").exists()
}

fn git(dir: &Path, args: &[&str]) -> Result<Output> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        // Never wait for a password prompt in the background.
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .context("running git")?;
    Ok(out)
}

fn git_ok(dir: &Path, args: &[&str]) -> Result<String> {
    let out = git(dir, args)?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn rev(dir: &Path, r: &str) -> Option<String> {
    git_ok(dir, &["rev-parse", "--verify", "-q", r]).ok()
}

/// Commits every change in the folder, named after the poems it touches.
fn commit_all(dir: &Path) -> Result<bool> {
    git_ok(dir, &["add", "-A"])?;
    let changed = git_ok(dir, &["diff", "--cached", "--name-only"])?;
    if changed.is_empty() {
        return Ok(false);
    }
    let names: Vec<&str> = changed
        .lines()
        .map(|p| p.rsplit('/').next().unwrap_or(p))
        .take(5)
        .collect();
    let more = changed.lines().count().saturating_sub(names.len());
    let mut msg = format!("{SIDE}: {}", names.join(", "));
    if more > 0 {
        msg.push_str(&format!(" +{more}"));
    }
    git_ok(dir, &["commit", "-q", "-m", &msg])?;
    Ok(true)
}

/// Files changed on both sides since the common ancestor.
fn both_changed(dir: &Path) -> Result<Vec<String>> {
    let base = git_ok(dir, &["merge-base", "HEAD", "@{u}"])?;
    let ours = git_ok(dir, &["diff", "--name-only", &base, "HEAD"])?;
    let theirs = git_ok(dir, &["diff", "--name-only", &base, "@{u}"])?;
    let theirs: Vec<&str> = theirs.lines().collect();
    Ok(ours
        .lines()
        .filter(|f| theirs.contains(f))
        .map(str::to_string)
        .collect())
}

/// "Liman" → "Liman (masaüstü)", or "(masaüstü 2)" if that is taken.
fn conflict_copy(dir: &Path, file: &str) -> PathBuf {
    let mut n = 1;
    loop {
        let suffix = if n == 1 {
            format!(" ({SIDE})")
        } else {
            format!(" ({SIDE} {n})")
        };
        let p = dir.join(format!("{file}{suffix}"));
        if !p.exists() {
            return p;
        }
        n += 1;
    }
}

/// Brings in the remote's changes when both sides have new commits and a
/// rebase can't: merge preferring theirs, and keep ours as copies.
fn merge_keeping_both(dir: &Path, conflicts: &[String]) -> Result<()> {
    // Our versions, read before the merge touches them.
    let ours: Vec<(String, Option<String>)> = conflicts
        .iter()
        .map(|f| (f.clone(), git_ok(dir, &["show", &format!("HEAD:{f}")]).ok()))
        .collect();
    let merged = git(dir, &["merge", "--no-edit", "-q", "-X", "theirs", "@{u}"])?;
    if !merged.status.success() {
        // Left over: a poem deleted on one side and edited on the other.
        // Take theirs where it exists; our text survives in the copy below.
        for f in git_ok(dir, &["diff", "--name-only", "--diff-filter=U"])?.lines() {
            if git(dir, &["checkout", "--theirs", "--", f])?
                .status
                .success()
            {
                git_ok(dir, &["add", "--", f])?;
            } else {
                git_ok(dir, &["rm", "-q", "--", f])?;
            }
        }
        git_ok(dir, &["commit", "-q", "--no-edit"])?;
    }
    for (f, text) in ours {
        let Some(text) = text else { continue };
        let theirs_now = std::fs::read_to_string(dir.join(&f)).unwrap_or_default();
        if theirs_now.trim_end() != text.trim_end() {
            std::fs::write(conflict_copy(dir, &f), format!("{}\n", text.trim_end()))?;
        }
    }
    commit_all(dir)?;
    Ok(())
}

/// Commits local changes, brings in the remote's, and pushes.
pub fn sync(dir: &Path) -> Result<Outcome> {
    let mut out = Outcome::default();
    commit_all(dir)?;
    if git(dir, &["remote"])?.stdout.is_empty() {
        return Ok(out);
    }
    git_ok(dir, &["fetch", "-q"])?;
    let Some(upstream) = rev(dir, "@{u}") else {
        // A branch that was never pushed: publish it.
        git_ok(dir, &["push", "-q", "-u", "origin", "HEAD"])?;
        out.pushed = true;
        return Ok(out);
    };
    let behind = git_ok(dir, &["rev-list", "--count", "HEAD..@{u}"])? != "0";
    if behind {
        let rebased = git(dir, &["rebase", "-q", "@{u}"])?;
        if !rebased.status.success() {
            git(dir, &["rebase", "--abort"])?;
            out.conflicts = both_changed(dir)?;
            merge_keeping_both(dir, &out.conflicts)?;
        }
        out.pulled = true;
    }
    if rev(dir, "HEAD").as_deref() != Some(upstream.as_str())
        && git_ok(dir, &["rev-list", "--count", "@{u}..HEAD"])? != "0"
    {
        git_ok(dir, &["push", "-q"])?;
        out.pushed = true;
    }
    Ok(out)
}

/// The PDFs, on the remote's `pdf` branch as one commit that is replaced each
/// time. Kept in a separate git folder so the published folder stays clean.
pub fn push_pdfs(cfg: &Config) -> Result<bool> {
    let poems = cfg.poems_dir();
    let url = git_ok(&poems, &["remote", "get-url", "origin"])?;
    let gitdir = data_dir().join("pdf.git");
    let out_dir = cfg.out_dir();
    let pdf = |args: &[&str]| -> Result<String> {
        let mut full = vec![
            format!("--git-dir={}", gitdir.display()),
            format!("--work-tree={}", out_dir.display()),
        ];
        full.extend(args.iter().map(|s| s.to_string()));
        let refs: Vec<&str> = full.iter().map(String::as_str).collect();
        git_ok(&out_dir, &refs)
    };
    if !gitdir.exists() {
        std::fs::create_dir_all(&gitdir)?;
        pdf(&["init", "-q", "-b", "pdf"])?;
        pdf(&["remote", "add", "origin", &url])?;
    }
    // Only the PDFs: no build state, locks or temporary files.
    std::fs::write(gitdir.join("info/exclude"), ".*\n*.tmp\n")?;
    if pdf(&["ls-files", "--", ".*"]).is_ok_and(|f| !f.is_empty()) {
        pdf(&["rm", "-q", "--cached", "--", ".*"])?;
    }
    // The same identity as the poems repo.
    for key in ["user.name", "user.email"] {
        if let Ok(v) = git_ok(&poems, &["config", key]) {
            pdf(&["config", key, &v])?;
        }
    }
    pdf(&["add", "-A"])?;
    let has_head = pdf(&["rev-parse", "--verify", "-q", "HEAD"]).is_ok();
    if has_head && pdf(&["diff", "--cached", "--name-only"])?.is_empty() {
        return Ok(false);
    }
    if has_head {
        pdf(&["commit", "-q", "--amend", "-m", "PDF'ler"])?;
    } else {
        pdf(&["commit", "-q", "-m", "PDF'ler"])?;
    }
    pdf(&["push", "-q", "-f", "origin", "HEAD:refs/heads/pdf"])?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn run(dir: &Path, args: &[&str]) {
        git_ok(dir, args).unwrap();
    }

    /// A bare remote and two clones of it: "desk" and "phone".
    fn setup(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("poet-sync-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let remote = root.join("remote.git");
        run(
            &root,
            &[
                "init",
                "-q",
                "--bare",
                "-b",
                "main",
                remote.to_str().unwrap(),
            ],
        );
        let clone = |n: &str| {
            let d = root.join(n);
            run(
                &root,
                &["clone", "-q", remote.to_str().unwrap(), d.to_str().unwrap()],
            );
            run(&d, &["config", "user.name", "test"]);
            run(&d, &["config", "user.email", "test@example.com"]);
            run(&d, &["symbolic-ref", "HEAD", "refs/heads/main"]);
            d
        };
        let desk = clone("desk");
        fs::write(desk.join("Liman"), "Liman\n\nbir\n").unwrap();
        sync(&desk).unwrap();
        let phone = clone("phone");
        (root, desk, phone)
    }

    #[test]
    fn brings_changes_both_ways() {
        let (root, desk, phone) = setup("both");
        fs::write(phone.join("Gece"), "Gece\n\niki\n").unwrap();
        sync(&phone).unwrap();
        fs::write(desk.join("Kum"), "Kum\n\nüç\n").unwrap();
        let out = sync(&desk).unwrap();
        assert!(out.pulled && out.pushed && out.conflicts.is_empty());
        assert!(desk.join("Gece").exists());
        sync(&phone).unwrap();
        assert!(phone.join("Kum").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn same_poem_on_both_sides_keeps_both() {
        let (root, desk, phone) = setup("conflict");
        fs::write(phone.join("Liman"), "Liman\n\ntelefondan\n").unwrap();
        sync(&phone).unwrap();
        fs::write(desk.join("Liman"), "Liman\n\nmasaüstünden\n").unwrap();
        let out = sync(&desk).unwrap();
        assert_eq!(out.conflicts, vec!["Liman"]);
        assert_eq!(
            fs::read_to_string(desk.join("Liman")).unwrap(),
            "Liman\n\ntelefondan\n"
        );
        assert_eq!(
            fs::read_to_string(desk.join("Liman (masaüstü)")).unwrap(),
            "Liman\n\nmasaüstünden\n"
        );
        // Both versions reach the other side too.
        sync(&phone).unwrap();
        assert!(phone.join("Liman (masaüstü)").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
