use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    KeyPress(KeyEvent),

    Tick,

    Resize {
        width: u16,
        height: u16,
    },

    CreateTab {
        name: String,
    },

    CloseTab {
        session_id: SessionId,
    },

    FocusNextTab,

    FocusPreviousTab,

    FocusTab {
        index: usize,
    },

    FocusTerminal {
        session_id: SessionId,
    },

    OpenCommandPalette,

    CloseCommandPalette,

    RenameTab {
        session_id: SessionId,
        name: String,
    },

    PtyOutput {
        session_id: SessionId,
        bytes: Vec<u8>,
    },

    PtyExit {
        session_id: SessionId,
    },

    SessionCreated {
        session_id: SessionId,
    },

    SessionRestored {
        session_id: SessionId,
    },

    SessionClosed {
        session_id: SessionId,
    },

    TabCreated {
        session_id: SessionId,
        name: String,
    },

    Notify {
        message: String,
    },
}
