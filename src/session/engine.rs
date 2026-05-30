use crate::terminal::screen::ScreenBuffer;

pub struct TerminalEngine {
    screen: ScreenBuffer,
    pid: u32,
    rows: u16,
    cols: u16,
}

impl TerminalEngine {
    pub fn new(rows: u16, cols: u16, pid: u32) -> Self {
        Self {
            screen: ScreenBuffer::new(rows, cols),
            pid,
            rows,
            cols,
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.screen.process(bytes);
    }

    pub fn screen(&self) -> &ScreenBuffer {
        &self.screen
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.rows = rows;
        self.cols = cols;
        self.screen.resize(rows, cols);
    }
}
