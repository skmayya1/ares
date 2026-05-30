use anyhow::Result;
use ratatui::DefaultTerminal;

use ares::App;

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> Result<()> {
    let size = terminal.size()?;
    let rows = size.height.saturating_sub(4).max(1);
    let cols = size.width.max(1);
    let mut app = App::new(rows, cols)?;

    loop {
        app.collect_events(terminal)?;
        app.process_events()?;
        app.render(terminal)?;

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
