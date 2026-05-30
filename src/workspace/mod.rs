use crate::event::SessionId;
use crate::worktree::Worktree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Starting,
    Running,
    Exited,
    #[serde(alias = "invalid")]
    Crashed,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub title: String,
    pub worktree: Worktree,
    pub status: SessionStatus,
}

pub struct Workspace {
    sessions: Vec<Session>,
    next_index: usize,
}

impl Session {
    pub fn is_focusable(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Starting | SessionStatus::Running
        )
    }
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            next_index: 1,
        }
    }

    pub fn next_default_name(&mut self) -> String {
        let name = format!("Task {}", self.next_index);
        self.next_index += 1;
        name
    }

    pub fn add_session(
        &mut self,
        id: SessionId,
        title: String,
        worktree: Worktree,
        status: SessionStatus,
    ) {
        self.sessions.push(Session {
            id,
            title,
            worktree,
            status,
        });
    }

    pub fn remove_session(&mut self, id: SessionId) -> Option<SessionId> {
        let index = self.sessions.iter().position(|session| session.id == id)?;
        self.sessions.remove(index);

        if self.sessions.is_empty() {
            return None;
        }

        Some(self.sessions[index.min(self.sessions.len() - 1)].id)
    }

    pub fn rename_session(&mut self, id: SessionId, title: String) -> bool {
        let Some(session) = self.sessions.iter_mut().find(|session| session.id == id) else {
            return false;
        };
        session.title = title;
        true
    }

    pub fn set_status(&mut self, id: SessionId, status: SessionStatus) -> bool {
        let Some(session) = self.sessions.iter_mut().find(|session| session.id == id) else {
            return false;
        };
        session.status = status;
        true
    }

    pub fn get(&self, id: SessionId) -> Option<&Session> {
        self.sessions.iter().find(|session| session.id == id)
    }

    pub fn next_session(&self, current: SessionId) -> Option<SessionId> {
        let index = self.session_index(current)?;
        let next = (index + 1) % self.sessions.len();
        Some(self.sessions[next].id)
    }

    pub fn prev_session(&self, current: SessionId) -> Option<SessionId> {
        let index = self.session_index(current)?;
        let prev = (index + self.sessions.len() - 1) % self.sessions.len();
        Some(self.sessions[prev].id)
    }

    pub fn active_session(
        &self,
        focus: Option<SessionId>,
        last_terminal: Option<SessionId>,
    ) -> Option<SessionId> {
        for candidate in [focus, last_terminal].into_iter().flatten() {
            if self
                .get(candidate)
                .is_some_and(|session| session.is_focusable())
            {
                return Some(candidate);
            }
        }

        self.sessions
            .iter()
            .find(|session| session.is_focusable())
            .map(|session| session.id)
    }

    pub fn next_focusable(&self, current: SessionId) -> Option<SessionId> {
        let start = self.session_index(current)?;
        let len = self.sessions.len();
        for offset in 1..=len {
            let session = &self.sessions[(start + offset) % len];
            if session.is_focusable() {
                return Some(session.id);
            }
        }
        None
    }

    pub fn prev_focusable(&self, current: SessionId) -> Option<SessionId> {
        let start = self.session_index(current)?;
        let len = self.sessions.len();
        for offset in 1..=len {
            let session = &self.sessions[(start + len - offset) % len];
            if session.is_focusable() {
                return Some(session.id);
            }
        }
        None
    }

    pub fn session_index(&self, id: SessionId) -> Option<usize> {
        self.sessions.iter().position(|session| session.id == id)
    }

    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn to_persisted(&self) -> Vec<crate::worktree::PersistedSession> {
        self.sessions
            .iter()
            .filter(|session| {
                matches!(
                    session.status,
                    SessionStatus::Starting | SessionStatus::Running
                )
            })
            .map(|session| crate::worktree::PersistedSession {
                session_id: session.id,
                title: session.title.clone(),
                worktree: session.worktree.clone(),
                status: session.status,
            })
            .collect()
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn id(n: u8) -> SessionId {
        SessionId(uuid::Uuid::from_bytes([n; 16]))
    }

    fn sample_worktree(name: &str) -> Worktree {
        Worktree {
            id: format!("repo-{name}"),
            name: name.into(),
            path: PathBuf::from(format!("/tmp/{name}")),
            branch: format!("ares/{name}"),
            repo_root: PathBuf::from("/tmp/repo"),
            primary: false,
        }
    }

    #[test]
    fn cycles_sessions_in_order() {
        let mut workspace = Workspace::new();
        workspace.add_session(
            id(1),
            "a".into(),
            sample_worktree("a"),
            SessionStatus::Running,
        );
        workspace.add_session(
            id(2),
            "b".into(),
            sample_worktree("b"),
            SessionStatus::Running,
        );
        workspace.add_session(
            id(3),
            "c".into(),
            sample_worktree("c"),
            SessionStatus::Running,
        );

        assert_eq!(workspace.next_session(id(3)), Some(id(1)));
        assert_eq!(workspace.prev_session(id(1)), Some(id(3)));
    }

    #[test]
    fn remove_returns_neighbor_for_focus() {
        let mut workspace = Workspace::new();
        workspace.add_session(
            id(1),
            "a".into(),
            sample_worktree("a"),
            SessionStatus::Running,
        );
        workspace.add_session(
            id(2),
            "b".into(),
            sample_worktree("b"),
            SessionStatus::Running,
        );

        assert_eq!(workspace.remove_session(id(2)), Some(id(1)));
    }
}
