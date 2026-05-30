use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::git;
use super::model::Worktree;
use super::slug::{repo_name, task_slug};
use super::store::{worktree_path, PersistedSession, PersistedState, WorktreeStore};

pub struct WorktreeManager {
    repo_root: PathBuf,
    store: WorktreeStore,
    worktrees: Vec<Worktree>,
}

impl WorktreeManager {
    pub fn new() -> Result<Self> {
        let repo_root = git::repo_root().context("open a Git repository before starting Ares")?;
        let store = WorktreeStore::new();
        let persisted = store.load().unwrap_or_default();

        let worktrees = persisted
            .sessions
            .iter()
            .map(|session| session.worktree.clone())
            .collect();

        Ok(Self {
            repo_root,
            store,
            worktrees,
        })
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    pub fn load_sessions(&self) -> Result<Vec<PersistedSession>> {
        let state = self.store.load()?;
        Ok(state.sessions)
    }

    /// Main repo checkout for the first tab — no branch or worktree is created.
    pub fn primary(&self) -> Worktree {
        let name = repo_name(&self.repo_root);
        let branch = git::current_branch(&self.repo_root).unwrap_or_else(|_| "HEAD".into());

        Worktree {
            id: format!("{name}-primary"),
            name: name.clone(),
            path: self.repo_root.clone(),
            branch,
            repo_root: self.repo_root.clone(),
            primary: true,
        }
    }

    pub fn create(&mut self, title: &str) -> Result<Worktree> {
        let slug = self.unique_slug(title);
        let branch = format!("ares/{slug}");
        let path = worktree_path(&self.repo_root, &slug);
        let id = format!("{}-{}", repo_name(&self.repo_root), slug);

        git::create_branch(&self.repo_root, &branch)?;
        git::add_worktree(&self.repo_root, &path, &branch)?;

        let worktree = Worktree {
            id,
            name: slug,
            path,
            branch,
            repo_root: self.repo_root.clone(),
            primary: false,
        };

        self.worktrees.push(worktree.clone());
        Ok(worktree)
    }

    pub fn delete(&mut self, worktree: &Worktree) -> Result<()> {
        if worktree.is_primary() {
            return Ok(());
        }

        git::remove_worktree(&self.repo_root, &worktree.path)?;
        git::delete_branch(&self.repo_root, &worktree.branch)?;
        self.worktrees.retain(|item| item.id != worktree.id);
        Ok(())
    }

    pub fn exists(&self, worktree: &Worktree) -> bool {
        worktree.exists()
    }

    pub fn list(&self) -> &[Worktree] {
        &self.worktrees
    }

    pub fn verify(&self, worktree: &Worktree) -> bool {
        if worktree.is_primary() {
            return worktree.repo_root.exists();
        }

        worktree.path.exists() && worktree.repo_root.exists()
    }

    pub fn persist(&self, sessions: &[PersistedSession]) -> Result<()> {
        let state = PersistedState {
            repo_root: Some(self.repo_root.clone()),
            sessions: sessions.to_vec(),
        };
        self.store.save(&state)
    }

    fn unique_slug(&self, title: &str) -> String {
        let base = task_slug(title);
        let mut slug = base.clone();
        let mut counter = 2;

        while self.slug_taken(&slug) {
            slug = format!("{base}-{counter}");
            counter += 1;
        }

        slug
    }

    fn slug_taken(&self, slug: &str) -> bool {
        let path = worktree_path(&self.repo_root, slug);
        if path.exists() {
            return true;
        }

        let branch = format!("ares/{slug}");
        self.worktrees
            .iter()
            .any(|worktree| worktree.name == slug || worktree.branch == branch)
    }

    pub fn taken_slugs(&self) -> HashSet<String> {
        self.worktrees.iter().map(|w| w.name.clone()).collect()
    }
}
