mod app;
#[cfg(feature = "preview")]
mod preview;
mod setup;
mod signatures;
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
use std::time::Duration;

/// First run: ask who the user is before anything else. False if they quit.
fn first_run(terminal: &mut ratatui::DefaultTerminal) -> Result<bool> {
    let mut s = setup::Setup::first_run();
    loop {
        terminal.draw(|f| {
            f.render_widget(
                ratatui::widgets::Block::default()
                    .style(ratatui::style::Style::default().fg(ui::FG).bg(ui::BG)),
                f.area(),
            );
            setup::draw(f, f.area(), &s)
        })?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => match s.key(k) {
                setup::Outcome::Stay => {}
                setup::Outcome::Cancel => return Ok(false),
                setup::Outcome::Done => match s.apply(false) {
                    Ok(_) => return Ok(true),
                    Err(e) => s.set_error(format!("{e:#}")),
                },
            },
            Event::Paste(p) => s.paste(&p),
            _ => {}
        }
    }
}

pub fn run(start_writing: bool) -> Result<()> {
    let mut terminal = ratatui::init();
    let _ = execute!(stdout(), EnableBracketedPaste);
    let result = (|| -> Result<()> {
        if !Config::exists() && !first_run(&mut terminal)? {
            return Ok(());
        }
        let cfg = Config::load()?;
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
                            let _ = crate::open::editor(&path).status();
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
