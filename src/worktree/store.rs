use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::event::SessionId;
use crate::workspace::SessionStatus;

use super::model::Worktree;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub session_id: SessionId,
    pub title: String,
    pub worktree: Worktree,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedState {
    pub repo_root: Option<PathBuf>,
    pub sessions: Vec<PersistedSession>,
}

pub struct WorktreeStore {
    path: PathBuf,
}

impl WorktreeStore {
    pub fn new() -> Self {
        Self {
            path: ares_dir().join("sessions.json"),
        }
    }

    pub fn load(&self) -> Result<PersistedState> {
        if !self.path.exists() {
            return Ok(PersistedState::default());
        }

        let contents = fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        let state = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", self.path.display()))?;
        Ok(state)
    }

    pub fn save(&self, state: &PersistedState) -> Result<()> {
        let dir = ares_dir();
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

        let contents = serde_json::to_string_pretty(state).context("failed to serialize sessions")?;
        fs::write(&self.path, contents)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }
}

pub fn ares_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".ares"))
        .unwrap_or_else(|| PathBuf::from(".ares"))
}

pub fn worktrees_dir() -> PathBuf {
    ares_dir().join("worktrees")
}

pub fn worktree_path(repo_root: &Path, slug: &str) -> PathBuf {
    let name = format!("{}-{}", super::slug::repo_name(repo_root), slug);
    worktrees_dir().join(name)
}
