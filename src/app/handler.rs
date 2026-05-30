use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::event::{Event, EventBus, SessionId};
use crate::focus::FocusTarget;
use crate::session::SessionManager;
use crate::terminal::input::{is_app_shortcut, key_event_to_bytes, tab_cycle_direction, TabCycleDirection};
use crate::worktree::WorktreeManager;
use crate::workspace::{SessionStatus, Workspace};

pub struct EventHandler;

impl EventHandler {
    pub fn handle(
        event: Event,
        bus: &mut EventBus,
        focus: &mut FocusTarget,
        last_terminal: &mut Option<SessionId>,
        workspace: &mut Workspace,
        sessions: &mut SessionManager,
        worktrees: &mut WorktreeManager,
        rows: u16,
        cols: u16,
        events: &crate::event::EventSender,
        should_quit: &mut bool,
        status_message: &mut Option<String>,
        new_tab_name: &mut String,
    ) -> Result<()> {
        match event {
            Event::KeyPress(key) => Self::handle_key_press(
                key,
                bus,
                focus,
                last_terminal,
                sessions,
                should_quit,
                new_tab_name,
            )?,
            Event::Tick => {}
            Event::Resize { width, height } => {
                sessions.resize_all(height, width)?;
            }
            Event::CreateTab { name } => {
                Self::handle_create_tab(
                    name,
                    bus,
                    focus,
                    last_terminal,
                    workspace,
                    sessions,
                    worktrees,
                    rows,
                    cols,
                    events,
                    status_message,
                )?;
            }
            Event::CloseTab { session_id } => {
                Self::handle_close_tab(
                    session_id,
                    bus,
                    focus,
                    last_terminal,
                    workspace,
                    sessions,
                    worktrees,
                    should_quit,
                    status_message,
                )?;
            }
            Event::FocusNextTab => {
                Self::focus_next_tab(focus, last_terminal, workspace);
            }
            Event::FocusPreviousTab => {
                Self::focus_prev_tab(focus, last_terminal, workspace);
            }
            Event::FocusTab { index } => {
                if let Some(session) = workspace.sessions().get(index) {
                    Self::focus_terminal(session.id, focus, last_terminal, workspace);
                }
            }
            Event::FocusTerminal { session_id } => {
                Self::focus_terminal(session_id, focus, last_terminal, workspace);
            }
            Event::OpenCommandPalette => {
                *focus = FocusTarget::CommandPalette;
            }
            Event::CloseCommandPalette => {
                Self::restore_terminal_focus(focus, last_terminal);
            }
            Event::RenameTab { session_id, name } => {
                workspace.rename_session(session_id, name);
                Self::persist(workspace, worktrees, status_message);
            }
            Event::PtyOutput { session_id, bytes } => {
                if let Some(session) = sessions.get_mut(session_id) {
                    session.process_output(&bytes);
                }
                if workspace
                    .get(session_id)
                    .is_some_and(|session| session.status == SessionStatus::Starting)
                {
                    workspace.set_status(session_id, SessionStatus::Running);
                }
            }
            Event::PtyExit { session_id } => {
                workspace.set_status(session_id, SessionStatus::Exited);
                Self::persist(workspace, worktrees, status_message);
            }
            Event::SessionCreated { .. } => {}
            Event::SessionRestored { .. } => {}
            Event::SessionClosed { .. } => {}
            Event::TabCreated { .. } => {}
            Event::Notify { message } => {
                *status_message = Some(message);
            }
        }

        Ok(())
    }

    fn persist(
        workspace: &Workspace,
        worktrees: &WorktreeManager,
        status_message: &mut Option<String>,
    ) {
        if let Err(error) = worktrees.persist(&workspace.to_persisted()) {
            *status_message = Some(error.to_string());
        }
    }

    fn focus_terminal(
        session_id: SessionId,
        focus: &mut FocusTarget,
        last_terminal: &mut Option<SessionId>,
        workspace: &Workspace,
    ) {
        let Some(session) = workspace.get(session_id) else {
            return;
        };
        if !session.is_focusable() {
            return;
        }

        *focus = FocusTarget::Terminal(session_id);
        *last_terminal = Some(session_id);
    }

    fn focus_next_tab(focus: &mut FocusTarget, last_terminal: &mut Option<SessionId>, workspace: &Workspace) {
        let Some(current) = workspace.active_session(focus.terminal(), *last_terminal) else {
            return;
        };
        if let Some(next) = workspace.next_focusable(current) {
            Self::focus_terminal(next, focus, last_terminal, workspace);
        }
    }

    fn focus_prev_tab(focus: &mut FocusTarget, last_terminal: &mut Option<SessionId>, workspace: &Workspace) {
        let Some(current) = workspace.active_session(focus.terminal(), *last_terminal) else {
            return;
        };
        if let Some(prev) = workspace.prev_focusable(current) {
            Self::focus_terminal(prev, focus, last_terminal, workspace);
        }
    }

    fn restore_terminal_focus(focus: &mut FocusTarget, last_terminal: &Option<SessionId>) {
        if let Some(session_id) = last_terminal {
            *focus = FocusTarget::Terminal(*session_id);
        }
    }

    fn handle_key_press(
        key: KeyEvent,
        bus: &mut EventBus,
        focus: &mut FocusTarget,
        last_terminal: &Option<SessionId>,
        sessions: &mut SessionManager,
        should_quit: &mut bool,
        new_tab_name: &mut String,
    ) -> Result<()> {
        if *focus == FocusTarget::NewTabDialog {
            Self::handle_new_tab_dialog_key(key, bus, focus, last_terminal, new_tab_name);
            return Ok(());
        }

        if is_app_shortcut(key) {
            Self::handle_shortcut(key, bus, focus, new_tab_name, should_quit);
            return Ok(());
        }

        if !focus.accepts_terminal_input() {
            return Ok(());
        }

        let Some(session_id) = focus.terminal() else {
            return Ok(());
        };

        let bytes = key_event_to_bytes(key);
        if !bytes.is_empty() {
            if let Some(session) = sessions.get_mut(session_id) {
                session.write_input(&bytes)?;
            }
        }

        Ok(())
    }

    fn handle_new_tab_dialog_key(
        key: KeyEvent,
        bus: &mut EventBus,
        focus: &mut FocusTarget,
        last_terminal: &Option<SessionId>,
        new_tab_name: &mut String,
    ) {
        use KeyCode::*;

        match key.code {
            Esc => {
                new_tab_name.clear();
                Self::restore_terminal_focus(focus, last_terminal);
            }
            Enter => {
                let name = new_tab_name.trim().to_string();
                new_tab_name.clear();
                Self::restore_terminal_focus(focus, last_terminal);
                bus.push(Event::CreateTab { name });
            }
            Backspace => {
                new_tab_name.pop();
            }
            Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                new_tab_name.push(ch);
            }
            _ => {}
        }
    }

    fn handle_shortcut(
        key: KeyEvent,
        bus: &mut EventBus,
        focus: &mut FocusTarget,
        new_tab_name: &mut String,
        should_quit: &mut bool,
    ) {
        use KeyCode::*;

        match key.code {
            Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                new_tab_name.clear();
                *focus = FocusTarget::NewTabDialog;
            }
            Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(session_id) = focus.terminal() {
                    bus.push(Event::CloseTab { session_id });
                }
            }
            Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                *should_quit = true;
            }
            _ if tab_cycle_direction(key) == Some(TabCycleDirection::Next) => {
                bus.push(Event::FocusNextTab);
            }
            _ if tab_cycle_direction(key) == Some(TabCycleDirection::Previous) => {
                bus.push(Event::FocusPreviousTab);
            }
            _ => {}
        }
    }

    fn handle_create_tab(
        name: String,
        bus: &mut EventBus,
        focus: &mut FocusTarget,
        last_terminal: &mut Option<SessionId>,
        workspace: &mut Workspace,
        sessions: &mut SessionManager,
        worktrees: &mut WorktreeManager,
        rows: u16,
        cols: u16,
        events: &crate::event::EventSender,
        status_message: &mut Option<String>,
    ) -> Result<()> {
        let title = if workspace.session_count() == 0 {
            "Main".into()
        } else if name.trim().is_empty() {
            workspace.next_default_name()
        } else {
            name.trim().to_string()
        };

        let worktree = if workspace.session_count() == 0 {
            worktrees.primary()
        } else {
            match worktrees.create(&title) {
                Ok(worktree) => worktree,
                Err(error) => {
                    bus.push(Event::Notify {
                        message: error.to_string(),
                    });
                    return Ok(());
                }
            }
        };

        let session_id = SessionId::new();

        workspace.add_session(
            session_id,
            title.clone(),
            worktree.clone(),
            SessionStatus::Starting,
        );

        if let Err(error) = sessions.create(session_id, &worktree.path, rows, cols, events.clone())
        {
            workspace.remove_session(session_id);
            if !worktree.is_primary() {
                let _ = worktrees.delete(&worktree);
            }
            bus.push(Event::Notify {
                message: format!("failed to launch codex: {error}"),
            });
            return Ok(());
        }

        workspace.set_status(session_id, SessionStatus::Running);
        *focus = FocusTarget::Terminal(session_id);
        *last_terminal = Some(session_id);
        *status_message = None;

        Self::persist(workspace, worktrees, status_message);

        bus.push(Event::SessionCreated { session_id });
        bus.push(Event::TabCreated {
            session_id,
            name: title,
        });

        Ok(())
    }

    fn handle_close_tab(
        session_id: SessionId,
        bus: &mut EventBus,
        focus: &mut FocusTarget,
        last_terminal: &mut Option<SessionId>,
        workspace: &mut Workspace,
        sessions: &mut SessionManager,
        worktrees: &mut WorktreeManager,
        should_quit: &mut bool,
        status_message: &mut Option<String>,
    ) -> Result<()> {
        let Some(session) = workspace.get(session_id) else {
            return Ok(());
        };
        let worktree = session.worktree.clone();
        let closing_focused = focus.is_terminal(session_id);
        let last_tab = workspace.session_count() <= 1;

        sessions.close(session_id);

        if let Err(error) = worktrees.delete(&worktree) {
            bus.push(Event::Notify {
                message: format!("worktree cleanup failed: {error}"),
            });
        }

        workspace.remove_session(session_id);

        if !last_tab {
            Self::persist(workspace, worktrees, status_message);
        } else {
            let _ = worktrees.persist(&[]);
            *should_quit = true;
            return Ok(());
        }

        if closing_focused {
            if let Some(next) = workspace
                .sessions()
                .iter()
                .find(|session| session.is_focusable())
                .map(|session| session.id)
            {
                *focus = FocusTarget::Terminal(next);
                *last_terminal = Some(next);
            }
        }

        bus.push(Event::SessionClosed { session_id });

        Ok(())
    }
}
