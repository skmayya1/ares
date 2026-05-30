//! Maps keyboard input from the TUI into bytes for the PTY.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn is_app_shortcut(key: KeyEvent) -> bool {
    if tab_cycle_direction(key).is_some() {
        return true;
    }

    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }

    matches!(
        key.code,
        KeyCode::Char('t') | KeyCode::Char('w') | KeyCode::Char('q')
    )
}

/// Tab cycle direction, if this key should switch Ares tabs rather than go to the shell.
pub fn tab_cycle_direction(key: KeyEvent) -> Option<TabCycleDirection> {
    use KeyCode::*;

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        PageDown | F(2) => Some(TabCycleDirection::Next),
        PageUp | F(1) => Some(TabCycleDirection::Previous),
        Tab if ctrl => Some(if shift {
            TabCycleDirection::Previous
        } else {
            TabCycleDirection::Next
        }),
        // Ctrl+Tab is often reported as Ctrl+I (Tab == ^I).
        Char('i') | Char('\t') if ctrl => Some(if shift {
            TabCycleDirection::Previous
        } else {
            TabCycleDirection::Next
        }),
        BackTab if ctrl => Some(TabCycleDirection::Previous),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabCycleDirection {
    Next,
    Previous,
}

pub fn key_event_to_bytes(key: KeyEvent) -> Vec<u8> {
    use KeyCode::*;

    match key.code {
        Char(ch) => encode_char(ch, key.modifiers),
        Enter => b"\r".to_vec(),
        Backspace => b"\x7f".to_vec(),
        Left => b"\x1b[D".to_vec(),
        Right => b"\x1b[C".to_vec(),
        Up => b"\x1b[A".to_vec(),
        Down => b"\x1b[B".to_vec(),
        Home => b"\x1b[H".to_vec(),
        End => b"\x1b[F".to_vec(),
        PageUp => b"\x1b[5~".to_vec(),
        PageDown => b"\x1b[6~".to_vec(),
        Tab => b"\t".to_vec(),
        Esc => b"\x1b".to_vec(),
        Delete => b"\x1b[3~".to_vec(),
        Insert => b"\x1b[2~".to_vec(),
        F(1) => b"\x1bOP".to_vec(),
        F(2) => b"\x1bOQ".to_vec(),
        F(3) => b"\x1bOR".to_vec(),
        F(4) => b"\x1bOS".to_vec(),
        F(n @ 5..=12) => format!("\x1b[{n}~").into_bytes(),
        _ => Vec::new(),
    }
}

fn encode_char(ch: char, modifiers: KeyModifiers) -> Vec<u8> {
    if modifiers.contains(KeyModifiers::CONTROL) {
        if let Some(byte) = ctrl_char(ch) {
            return vec![byte];
        }
    }

    if modifiers.contains(KeyModifiers::ALT) {
        let mut bytes = b"\x1b".to_vec();
        bytes.extend(ch.to_string().into_bytes());
        return bytes;
    }

    ch.to_string().into_bytes()
}

fn ctrl_char(ch: char) -> Option<u8> {
    let upper = ch.to_ascii_uppercase();
    if upper.is_ascii_alphabetic() {
        Some((upper as u8) - b'A' + 1)
    } else if upper == '@' {
        Some(0)
    } else if upper == '[' {
        Some(27)
    } else if upper == '\\' {
        Some(28)
    } else if upper == ']' {
        Some(29)
    } else if upper == '^' {
        Some(30)
    } else if upper == '_' {
        Some(31)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_enter_as_carriage_return() {
        let bytes = key_event_to_bytes(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(bytes, b"\r");
    }

    #[test]
    fn encodes_control_c() {
        let bytes = key_event_to_bytes(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(bytes, vec![3]);
    }

    #[test]
    fn ctrl_i_cycles_next_tab() {
        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL);
        assert_eq!(tab_cycle_direction(key), Some(TabCycleDirection::Next));
        assert!(is_app_shortcut(key));
    }

    #[test]
    fn page_down_cycles_next_tab() {
        let key = KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE);
        assert_eq!(tab_cycle_direction(key), Some(TabCycleDirection::Next));
    }
}
