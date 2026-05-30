use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, MouseEventKind};
use ratatui::DefaultTerminal;

use crate::event::{Event, EventBus, EventSender, SessionId};
use crate::focus::FocusTarget;
use crate::session::SessionManager;
use crate::ui::Theme;
use crate::ui::render;
use crate::workspace::{SessionStatus, Workspace};
use crate::worktree::{PersistedSession, WorktreeManager};

use super::handler::{EventContext, EventHandler};

pub struct App {
    bus: EventBus,
    events: EventSender,
    focus: FocusTarget,
    last_terminal: Option<SessionId>,
    workspace: Workspace,
    sessions: SessionManager,
    worktrees: WorktreeManager,
    theme: Theme,
    rows: u16,
    cols: u16,
    new_tab_name: String,
    pub should_quit: bool,
    pub status_message: Option<String>,
}

impl App {
    pub fn new(rows: u16, cols: u16) -> Result<Self> {
        let (bus, events) = EventBus::new();
        let worktrees = WorktreeManager::new()?;
        let mut app = Self {
            bus,
            events: events.clone(),
            focus: FocusTarget::CommandPalette,
            last_terminal: None,
            workspace: Workspace::new(),
            sessions: SessionManager::new(),
            worktrees,
            theme: Theme::default(),
            rows,
            cols,
            new_tab_name: String::new(),
            should_quit: false,
            status_message: None,
        };

        let persisted = app.worktrees.load_sessions().unwrap_or_default();
        if persisted.is_empty() {
            app.bus.push(Event::CreateTab {
                name: String::new(),
            });
        } else {
            app.restore_sessions(persisted)?;
        }

        app.process_events()?;
        Ok(app)
    }

    fn restore_sessions(&mut self, persisted: Vec<PersistedSession>) -> Result<()> {
        let mut first_running = None;

        for record in persisted {
            let status = if self.worktrees.verify(&record.worktree) {
                SessionStatus::Starting
            } else {
                SessionStatus::Crashed
            };

            if status == SessionStatus::Starting {
                if let Err(error) = self.sessions.create(
                    record.session_id,
                    &record.worktree.path,
                    self.rows,
                    self.cols,
                    self.events.clone(),
                ) {
                    self.status_message =
                        Some(format!("failed to restore `{}`: {error}", record.title));
                    self.workspace.add_session(
                        record.session_id,
                        record.title,
                        record.worktree,
                        SessionStatus::Crashed,
                    );
                    continue;
                }

                first_running.get_or_insert(record.session_id);
            }

            self.workspace.add_session(
                record.session_id,
                record.title,
                record.worktree,
                if status == SessionStatus::Starting {
                    SessionStatus::Running
                } else {
                    status
                },
            );
        }

        if let Some(session_id) = first_running {
            self.focus = FocusTarget::Terminal(session_id);
            self.last_terminal = Some(session_id);
            self.bus.push(Event::SessionRestored { session_id });
        } else {
            self.bus.push(Event::CreateTab {
                name: String::new(),
            });
        }

        Ok(())
    }

    pub fn collect_events(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.bus.push(Event::Tick);
        self.bus.drain_external();

        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                CrosstermEvent::Key(key) => self.bus.push(Event::KeyPress(key)),
                CrosstermEvent::Mouse(mouse) => {
                    if matches!(mouse.kind, MouseEventKind::Down(_)) {
                        if mouse.row < 3 {
                            let count = self.workspace.session_count();
                            if count > 0 {
                                let width = terminal.size()?.width.max(1) as usize;
                                let index = (mouse.column as usize * count / width).min(count - 1);
                                self.bus.push(Event::FocusTab { index });
                            }
                        } else if let Some(session_id) = self.display_session_id() {
                            self.bus.push(Event::FocusTerminal { session_id });
                        }
                    }
                }
                CrosstermEvent::Resize(width, height) => {
                    terminal.resize(ratatui::layout::Rect::new(0, 0, width, height))?;
                    let (rows, cols) = content_size(height, width);
                    self.rows = rows;
                    self.cols = cols;
                    self.bus.push(Event::Resize {
                        width: cols,
                        height: rows,
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn process_events(&mut self) -> Result<()> {
        while let Some(event) = self.bus.pop() {
            let context = EventContext {
                bus: &mut self.bus,
                focus: &mut self.focus,
                last_terminal: &mut self.last_terminal,
                workspace: &mut self.workspace,
                sessions: &mut self.sessions,
                worktrees: &mut self.worktrees,
                rows: self.rows,
                cols: self.cols,
                events: &self.events,
                should_quit: &mut self.should_quit,
                status_message: &mut self.status_message,
                new_tab_name: &mut self.new_tab_name,
            };
            EventHandler::new(context).handle(event)?;
        }
        Ok(())
    }

    pub fn render(&self, terminal: &mut DefaultTerminal) -> Result<()> {
        terminal.draw(|frame| render(frame, self))?;
        Ok(())
    }

    pub fn focus(&self) -> &FocusTarget {
        &self.focus
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn sessions(&self) -> &SessionManager {
        &self.sessions
    }

    pub fn new_tab_name(&self) -> &str {
        &self.new_tab_name
    }

    pub fn display_session_id(&self) -> Option<SessionId> {
        self.workspace
            .active_session(self.focus.terminal(), self.last_terminal)
    }

    pub fn terminal_focused(&self, session_id: SessionId) -> bool {
        self.focus.is_terminal(session_id)
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }
}

fn content_size(rows: u16, cols: u16) -> (u16, u16) {
    (rows.saturating_sub(4).max(1), cols.max(1))
}
