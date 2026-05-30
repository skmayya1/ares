use crate::event::SessionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Terminal(SessionId),
    CommandPalette,
    NewTabDialog,
}

impl FocusTarget {
    pub fn terminal(&self) -> Option<SessionId> {
        match self {
            Self::Terminal(id) => Some(*id),
            _ => None,
        }
    }

    pub fn is_terminal(&self, session_id: SessionId) -> bool {
        matches!(self, Self::Terminal(id) if *id == session_id)
    }

    pub fn accepts_terminal_input(&self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}
