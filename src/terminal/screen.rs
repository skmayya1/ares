//! ANSI bytes from the PTY are parsed into an in-memory screen model.

use vt100::{Parser, Screen};

const SCROLLBACK_LINES: usize = 1_000;

pub struct ScreenBuffer {
    rows: u16,
    cols: u16,
    parser: Parser,
}

impl ScreenBuffer {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            rows,
            cols,
            parser: Parser::new(rows, cols, SCROLLBACK_LINES),
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn screen(&self) -> &Screen {
        self.parser.screen()
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
        self.parser.screen_mut().set_size(rows, cols);
    }

    pub fn cursor_position(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    pub fn cursor_hidden(&self) -> bool {
        self.parser.screen().hide_cursor()
    }

    pub fn size(&self) -> (u16, u16) {
        self.parser.screen().size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vt100::Color;

    #[test]
    fn parses_ansi_color_sequences() {
        let mut screen = ScreenBuffer::new(24, 80);
        screen.process(b"hello \x1b[31mworld\x1b[m");

        let cell = screen.screen().cell(0, 6).unwrap();
        assert_eq!(cell.contents(), "w");
        assert_eq!(cell.fgcolor(), Color::Idx(1));
    }
}
