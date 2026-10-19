mod app;
mod preview;
mod ui;
pub mod writer;

use crate::config::Config;
use anyhow::Result;
use app::{Action, App};
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind,
};
use ratatui::crossterm::execute;
use std::io::stdout;
use std::process::Command;
use std::time::Duration;

pub fn run(start_writing: bool) -> Result<()> {
    let cfg = Config::load()?;
    let mut terminal = ratatui::init();
    let _ = execute!(stdout(), EnableBracketedPaste);
    let result = (|| -> Result<()> {
        let mut app = App::new(cfg)?;
        if start_writing {
            app.new_poem();
        }
        while !app.quit {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            if event::poll(Duration::from_millis(250))? {
                match event::read()? {
                    Event::Key(k) if k.kind == KeyEventKind::Press => {
                        if let Action::ExternalEditor(path) = app.on_key(k) {
                            let _ = execute!(stdout(), DisableBracketedPaste);
                            ratatui::restore();
                            let editor = std::env::var("VISUAL")
                                .or_else(|_| std::env::var("EDITOR"))
                                .unwrap_or_else(|_| "xdg-open".into());
                            let _ = Command::new(editor).arg(&path).status();
                            terminal = ratatui::init();
                            let _ = execute!(stdout(), EnableBracketedPaste);
                            terminal.clear()?;
                            app.refresh();
                        }
                    }
                    Event::Paste(s) => app.on_paste(&s),
                    _ => {}
                }
            }
            app.tick();
        }
        // Never lose words on quit.
        if let app::Mode::Writer(ed) = &mut app.mode
            && ed.dirty
        {
            ed.save(&app.cfg)?;
        }
        Ok(())
    })();

    let _ = execute!(stdout(), DisableBracketedPaste);
    ratatui::restore();
    result
}
