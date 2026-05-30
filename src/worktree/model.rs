use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worktree {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub repo_root: PathBuf,
    /// Main checkout — not a separate git worktree.
    #[serde(default)]
    pub primary: bool,
}

impl Worktree {
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn is_primary(&self) -> bool {
        self.primary
    }
}
