use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::event::{Event, EventBus, SessionId};
use crate::focus::FocusTarget;
use crate::session::SessionManager;
use crate::terminal::input::{
    TabCycleDirection, is_app_shortcut, key_event_to_bytes, tab_cycle_direction,
};
use crate::workspace::{SessionStatus, Workspace};
use crate::worktree::WorktreeManager;

pub struct EventContext<'a> {
    pub bus: &'a mut EventBus,
    pub focus: &'a mut FocusTarget,
    pub last_terminal: &'a mut Option<SessionId>,
    pub workspace: &'a mut Workspace,
    pub sessions: &'a mut SessionManager,
    pub worktrees: &'a mut WorktreeManager,
    pub rows: u16,
    pub cols: u16,
    pub events: &'a crate::event::EventSender,
    pub should_quit: &'a mut bool,
    pub status_message: &'a mut Option<String>,
    pub new_tab_name: &'a mut String,
}

pub struct EventHandler<'a> {
    context: EventContext<'a>,
}

impl<'a> EventHandler<'a> {
    pub fn new(context: EventContext<'a>) -> Self {
        Self { context }
    }

    pub fn handle(&mut self, event: Event) -> Result<()> {
        match event {
            Event::KeyPress(key) => self.handle_key_press(key)?,
            Event::Tick => {}
            Event::Resize { width, height } => {
                self.context.sessions.resize_all(height, width)?;
            }
            Event::CreateTab { name } => {
                self.handle_create_tab(name)?;
            }
            Event::CloseTab { session_id } => {
                self.handle_close_tab(session_id)?;
            }
            Event::FocusNextTab => {
                self.focus_next_tab();
            }
            Event::FocusPreviousTab => {
                self.focus_prev_tab();
            }
            Event::FocusTab { index } => {
                if let Some(session_id) = self
                    .context
                    .workspace
                    .sessions()
                    .get(index)
                    .map(|session| session.id)
                {
                    self.focus_terminal(session_id);
                }
            }
            Event::FocusTerminal { session_id } => {
                self.focus_terminal(session_id);
            }
            Event::OpenCommandPalette => {
                *self.context.focus = FocusTarget::CommandPalette;
            }
            Event::CloseCommandPalette => {
                self.restore_terminal_focus();
            }
            Event::RenameTab { session_id, name } => {
                self.context.workspace.rename_session(session_id, name);
                self.persist();
            }
            Event::PtyOutput { session_id, bytes } => {
                if let Some(session) = self.context.sessions.get_mut(session_id) {
                    session.process_output(&bytes);
                }
                if self
                    .context
                    .workspace
                    .get(session_id)
                    .is_some_and(|session| session.status == SessionStatus::Starting)
                {
                    self.context
                        .workspace
                        .set_status(session_id, SessionStatus::Running);
                }
            }
            Event::PtyExit { session_id } => {
                self.context
                    .workspace
                    .set_status(session_id, SessionStatus::Exited);
                self.persist();
            }
            Event::SessionCreated { .. } => {}
            Event::SessionRestored { .. } => {}
            Event::SessionClosed { .. } => {}
            Event::TabCreated { .. } => {}
            Event::Notify { message } => {
                *self.context.status_message = Some(message);
            }
        }

        Ok(())
    }

    fn persist(&mut self) {
        if let Err(error) = self
            .context
            .worktrees
            .persist(&self.context.workspace.to_persisted())
        {
            *self.context.status_message = Some(error.to_string());
        }
    }

    fn focus_terminal(&mut self, session_id: SessionId) {
        let Some(session) = self.context.workspace.get(session_id) else {
            return;
        };
        if !session.is_focusable() {
            return;
        }

        *self.context.focus = FocusTarget::Terminal(session_id);
        *self.context.last_terminal = Some(session_id);
    }

    fn focus_next_tab(&mut self) {
        let Some(current) = self
            .context
            .workspace
            .active_session(self.context.focus.terminal(), *self.context.last_terminal)
        else {
            return;
        };
        if let Some(next) = self.context.workspace.next_focusable(current) {
            self.focus_terminal(next);
        }
    }

    fn focus_prev_tab(&mut self) {
        let Some(current) = self
            .context
            .workspace
            .active_session(self.context.focus.terminal(), *self.context.last_terminal)
        else {
            return;
        };
        if let Some(prev) = self.context.workspace.prev_focusable(current) {
            self.focus_terminal(prev);
        }
    }

    fn restore_terminal_focus(&mut self) {
        if let Some(session_id) = self.context.last_terminal {
            *self.context.focus = FocusTarget::Terminal(*session_id);
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> Result<()> {
        if *self.context.focus == FocusTarget::NewTabDialog {
            self.handle_new_tab_dialog_key(key);
            return Ok(());
        }

        if is_app_shortcut(key) {
            self.handle_shortcut(key);
            return Ok(());
        }

        if !self.context.focus.accepts_terminal_input() {
            return Ok(());
        }

        let Some(session_id) = self.context.focus.terminal() else {
            return Ok(());
        };

        let bytes = key_event_to_bytes(key);
        if !bytes.is_empty() {
            if let Some(session) = self.context.sessions.get_mut(session_id) {
                session.write_input(&bytes)?;
            }
        }

        Ok(())
    }

    fn handle_new_tab_dialog_key(&mut self, key: KeyEvent) {
        use KeyCode::*;

        match key.code {
            Esc => {
                self.context.new_tab_name.clear();
                self.restore_terminal_focus();
            }
            Enter => {
                let name = self.context.new_tab_name.trim().to_string();
                self.context.new_tab_name.clear();
                self.restore_terminal_focus();
                self.context.bus.push(Event::CreateTab { name });
            }
            Backspace => {
                self.context.new_tab_name.pop();
            }
            Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.context.new_tab_name.push(ch);
            }
            _ => {}
        }
    }

    fn handle_shortcut(&mut self, key: KeyEvent) {
        use KeyCode::*;

        match key.code {
            Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.context.new_tab_name.clear();
                *self.context.focus = FocusTarget::NewTabDialog;
            }
            Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(session_id) = self.context.focus.terminal() {
                    self.context.bus.push(Event::CloseTab { session_id });
                }
            }
            Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                *self.context.should_quit = true;
            }
            _ if tab_cycle_direction(key) == Some(TabCycleDirection::Next) => {
                self.context.bus.push(Event::FocusNextTab);
            }
            _ if tab_cycle_direction(key) == Some(TabCycleDirection::Previous) => {
                self.context.bus.push(Event::FocusPreviousTab);
            }
            _ => {}
        }
    }

    fn handle_create_tab(&mut self, name: String) -> Result<()> {
        let title = if self.context.workspace.session_count() == 0 {
            "Main".into()
        } else if name.trim().is_empty() {
            self.context.workspace.next_default_name()
        } else {
            name.trim().to_string()
        };

        let worktree = if self.context.workspace.session_count() == 0 {
            self.context.worktrees.primary()
        } else {
            match self.context.worktrees.create(&title) {
                Ok(worktree) => worktree,
                Err(error) => {
                    self.context.bus.push(Event::Notify {
                        message: error.to_string(),
                    });
                    return Ok(());
                }
            }
        };

        let session_id = SessionId::new();

        self.context.workspace.add_session(
            session_id,
            title.clone(),
            worktree.clone(),
            SessionStatus::Starting,
        );

        if let Err(error) = self.context.sessions.create(
            session_id,
            &worktree.path,
            self.context.rows,
            self.context.cols,
            self.context.events.clone(),
        ) {
            self.context.workspace.remove_session(session_id);
            if !worktree.is_primary() {
                let _ = self.context.worktrees.delete(&worktree);
            }
            self.context.bus.push(Event::Notify {
                message: format!("failed to launch codex: {error}"),
            });
            return Ok(());
        }

        self.context
            .workspace
            .set_status(session_id, SessionStatus::Running);
        *self.context.focus = FocusTarget::Terminal(session_id);
        *self.context.last_terminal = Some(session_id);
        *self.context.status_message = None;

        self.persist();

        self.context.bus.push(Event::SessionCreated { session_id });
        self.context.bus.push(Event::TabCreated {
            session_id,
            name: title,
        });

        Ok(())
    }

    fn handle_close_tab(&mut self, session_id: SessionId) -> Result<()> {
        let Some(session) = self.context.workspace.get(session_id) else {
            return Ok(());
        };
        let worktree = session.worktree.clone();
        let closing_focused = self.context.focus.is_terminal(session_id);
        let last_tab = self.context.workspace.session_count() <= 1;

        self.context.sessions.close(session_id);

        if let Err(error) = self.context.worktrees.delete(&worktree) {
            self.context.bus.push(Event::Notify {
                message: format!("worktree cleanup failed: {error}"),
            });
        }

        self.context.workspace.remove_session(session_id);

        if !last_tab {
            self.persist();
        } else {
            let _ = self.context.worktrees.persist(&[]);
            *self.context.should_quit = true;
            return Ok(());
        }

        if closing_focused {
            if let Some(next) = self
                .context
                .workspace
                .sessions()
                .iter()
                .find(|session| session.is_focusable())
                .map(|session| session.id)
            {
                *self.context.focus = FocusTarget::Terminal(next);
                *self.context.last_terminal = Some(next);
            }
        }

        self.context.bus.push(Event::SessionClosed { session_id });

        Ok(())
    }
}
