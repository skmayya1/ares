use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};
use tui_term::widget::{Cursor, PseudoTerminal};

use crate::app::App;
use crate::focus::FocusTarget;
use crate::workspace::SessionStatus;

use super::theme::Theme;

const STATUS: &str = "^T new tab  ^W close  PgUp/PgDn cycle  ^Q quit";

const STATUS_STYLE: Style = Style::new()
    .fg(Color::Rgb(80, 80, 80))
    .add_modifier(Modifier::DIM);

pub fn render(frame: &mut Frame, app: &App) {
    let theme = app.theme();
    let [tab_area, terminal_area, status_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_tabs(frame, tab_area, app, theme);
    if matches!(app.focus(), FocusTarget::NewTabDialog) {
        render_new_tab_dialog(frame, terminal_area, app);
    } else {
        render_terminal(frame, terminal_area, app);
    }
    render_status(frame, status_area, app);
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let workspace = app.workspace();
    let active_session = app.display_session_id();

    let titles: Vec<Line> = workspace
        .sessions()
        .iter()
        .map(|session| {
            let is_active = active_session == Some(session.id);
            let style = match session.status {
                SessionStatus::Crashed | SessionStatus::Exited => {
                    Style::new().fg(Color::Red).add_modifier(Modifier::DIM)
                }
                SessionStatus::Starting => Style::new().fg(Color::Yellow),
                _ if is_active => Style::new()
                    .fg(theme.active_tab)
                    .add_modifier(Modifier::BOLD),
                _ => Style::new().fg(theme.inactive_tab),
            };
            let label = match session.status {
                SessionStatus::Crashed => format!("✗ {}", theme.tab_label(&session.title)),
                SessionStatus::Exited => format!("○ {}", theme.tab_label(&session.title)),
                SessionStatus::Starting => format!("… {}", theme.tab_label(&session.title)),
                _ => theme.tab_label(&session.title),
            };
            Line::from(Span::styled(label, style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::bordered()
                .border_style(
                    Style::new()
                        .fg(theme.tab_border)
                        .add_modifier(Modifier::DIM),
                ),
        )
        .select(
            active_session
                .and_then(|id| workspace.session_index(id))
                .unwrap_or(0),
        )
        .highlight_style(Style::new())
        .divider(Span::raw("  "));

    frame.render_widget(tabs, area);
}

fn render_new_tab_dialog(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered().title("New Tab");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from("Task Name:"),
        Line::from(""),
        Line::from(vec![
            Span::raw("> "),
            Span::styled(app.new_tab_name(), Style::new().add_modifier(Modifier::BOLD)),
            Span::styled("▌", Style::new().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Enter confirm  Esc cancel",
            Style::new().add_modifier(Modifier::DIM),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_terminal(frame: &mut Frame, area: Rect, app: &App) {
    let Some(session_id) = app.display_session_id() else {
        let empty = Paragraph::new("No focused session");
        frame.render_widget(empty, area);
        return;
    };

    let Some(workspace_session) = app.workspace().get(session_id) else {
        let empty = Paragraph::new("Session not found");
        frame.render_widget(empty, area);
        return;
    };

    if !workspace_session.is_focusable() {
        let message = match workspace_session.status {
            SessionStatus::Exited => "Codex exited — close tab with ^W",
            SessionStatus::Crashed => "Session crashed — close tab with ^W",
            _ => "Session unavailable",
        };
        frame.render_widget(Paragraph::new(message), area);
        return;
    }

    let Some(session) = app.sessions().get(session_id) else {
        let empty = Paragraph::new("Session not found");
        frame.render_widget(empty, area);
        return;
    };

    let is_active = app.terminal_focused(session_id);
    let screen_buffer = session.screen();
    let screen = screen_buffer.screen();
    let show_cursor = is_active && !screen_buffer.cursor_hidden();

    let mut cursor = Cursor::default();
    if !show_cursor {
        cursor.hide();
    }

    let pseudo = PseudoTerminal::new(screen)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .cursor(cursor);

    frame.render_widget(pseudo, area);
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let text = app.status_message().unwrap_or(STATUS);
    let style = if app.status_message().is_some() {
        Style::new().fg(Color::Red)
    } else {
        STATUS_STYLE
    };
    let status = Paragraph::new(text).style(style);
    frame.render_widget(status, area);
}
