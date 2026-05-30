use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use portable_pty::{Child, MasterPty};

use crate::event::{EventSender, SessionId};
use crate::terminal::pty::{resize_pty, spawn_codex};

use super::engine::TerminalEngine;

pub struct TerminalSession {
    pub id: SessionId,
    engine: TerminalEngine,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send>,
}

impl TerminalSession {
    pub fn engine(&self) -> &TerminalEngine {
        &self.engine
    }

    pub fn screen(&self) -> &crate::terminal::screen::ScreenBuffer {
        self.engine.screen()
    }

    pub fn process_output(&mut self, bytes: &[u8]) {
        self.engine.process(bytes);
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<()> {
        self.writer
            .write_all(data)
            .context("failed to write to PTY")?;
        self.writer.flush().context("failed to flush PTY writer")?;
        Ok(())
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        resize_pty(self.master.as_ref(), rows, cols)?;
        self.engine.resize(rows, cols);
        Ok(())
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

pub struct SessionManager {
    sessions: HashMap<SessionId, TerminalSession>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn create(
        &mut self,
        session_id: SessionId,
        worktree_path: &Path,
        rows: u16,
        cols: u16,
        events: EventSender,
    ) -> Result<SessionId> {
        let pty = spawn_codex(
            session_id,
            rows,
            cols,
            Some(worktree_path.to_str().unwrap_or(".")),
            events,
        )?;

        let pid = pty.child.process_id().unwrap_or(0);

        self.sessions.insert(
            session_id,
            TerminalSession {
                id: session_id,
                engine: TerminalEngine::new(rows, cols, pid),
                writer: pty.writer,
                master: pty.master,
                child: pty.child,
            },
        );

        Ok(session_id)
    }

    pub fn close(&mut self, session_id: SessionId) -> bool {
        if let Some(mut session) = self.sessions.remove(&session_id) {
            session.kill();
            return true;
        }
        false
    }

    pub fn get(&self, session_id: SessionId) -> Option<&TerminalSession> {
        self.sessions.get(&session_id)
    }

    pub fn get_mut(&mut self, session_id: SessionId) -> Option<&mut TerminalSession> {
        self.sessions.get_mut(&session_id)
    }

    pub fn resize_all(&mut self, rows: u16, cols: u16) -> Result<()> {
        for session in self.sessions.values_mut() {
            session
                .resize(rows, cols)
                .context("failed to resize session")?;
        }
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
