//! Spawns Codex (or configured command) inside a pseudo-terminal.

use std::io::{Read, Write};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use crate::event::{Event, EventSender, SessionId};

pub struct PtyHandle {
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    pub reader_thread: JoinHandle<()>,
    pub child: Box<dyn Child + Send>,
}

pub fn spawn_codex(
    session_id: SessionId,
    rows: u16,
    cols: u16,
    cwd: Option<&str>,
    events: EventSender,
) -> Result<PtyHandle> {
    let command = default_codex_command();
    spawn_command(session_id, rows, cols, &command, cwd, events)
}

pub fn spawn_command(
    session_id: SessionId,
    rows: u16,
    cols: u16,
    command: &str,
    cwd: Option<&str>,
    events: EventSender,
) -> Result<PtyHandle> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("failed to open PTY")?;

    let mut cmd = CommandBuilder::new(command);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    if let Some(cwd) = cwd {
        cmd.cwd(cwd);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .with_context(|| format!("failed to spawn `{command}`"))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .context("failed to clone PTY reader")?;

    let reader_thread = thread::spawn(move || {
        let mut buffer = [0_u8; 8_192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    events.send(Event::PtyExit { session_id });
                    break;
                }
                Ok(count) => {
                    events.send(Event::PtyOutput {
                        session_id,
                        bytes: buffer[..count].to_vec(),
                    });
                }
                Err(_) => break,
            }
        }
    });

    let writer = pair.master.take_writer().context("failed to open PTY writer")?;

    Ok(PtyHandle {
        master: pair.master,
        writer,
        reader_thread,
        child,
    })
}

pub fn resize_pty(master: &dyn MasterPty, rows: u16, cols: u16) -> Result<()> {
    master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("failed to resize PTY")
}

pub fn default_codex_command() -> String {
    std::env::var("ARES_CODEX").unwrap_or_else(|_| "codex".into())
}
