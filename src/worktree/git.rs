use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};

pub fn repo_root() -> Result<PathBuf> {
    let output = run_git(Path::new("."), &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(output))
}

pub fn current_branch(repo_root: &Path) -> Result<String> {
    run_git(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn create_branch(repo_root: &Path, branch: &str) -> Result<()> {
    if branch_exists(repo_root, branch)? {
        return Err(anyhow!("branch `{branch}` already exists"));
    }

    run_git(repo_root, &["branch", branch])?;
    Ok(())
}

pub fn add_worktree(repo_root: &Path, path: &Path, branch: &str) -> Result<()> {
    if path.exists() {
        return Err(anyhow!("worktree path `{}` already exists", path.display()));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    run_git(
        repo_root,
        &[
            "worktree",
            "add",
            path.to_str().context("invalid worktree path")?,
            branch,
        ],
    )?;
    Ok(())
}

pub fn remove_worktree(repo_root: &Path, path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let status = Command::new("git")
        .current_dir(repo_root)
        .args([
            "worktree",
            "remove",
            path.to_str().context("invalid worktree path")?,
            "--force",
        ])
        .status()
        .context("failed to run git worktree remove")?;

    if !status.success() {
        return Err(anyhow!("git worktree remove failed for {}", path.display()));
    }

    let _ = run_git(repo_root, &["worktree", "prune"]);
    Ok(())
}

pub fn delete_branch(repo_root: &Path, branch: &str) -> Result<()> {
    if !branch_exists(repo_root, branch)? {
        return Ok(());
    }

    let status = Command::new("git")
        .current_dir(repo_root)
        .args(["branch", "-D", branch])
        .status()
        .context("failed to run git branch -D")?;

    if !status.success() {
        return Err(anyhow!("git branch -D failed for `{branch}`"));
    }

    Ok(())
}

fn branch_exists(repo_root: &Path, branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["show-ref", "--verify", &format!("refs/heads/{branch}")])
        .output()
        .context("failed to check branch existence")?;

    Ok(output.status.success())
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            stderr
        };
        return Err(anyhow!(message));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
