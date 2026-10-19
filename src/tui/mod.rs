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
use ratatui_image::picker::cap_parser::QueryStdioOptions;
use ratatui_image::picker::{Picker, ProtocolType};
use std::io::stdout;
use std::process::Command;
use std::time::Duration;

pub fn run(start_writing: bool) -> Result<()> {
    let cfg = Config::load()?;
    let mut terminal = ratatui::init();
    let _ = execute!(stdout(), EnableBracketedPaste);
    // Image previews only where the terminal speaks sixel/kitty/iTerm2. The query is
    // only sent to terminals known to answer it: on others the library's reader
    // thread outlives its timeout and swallows keystrokes.
    let picker = if graphics_terminal() {
        let options = QueryStdioOptions {
            timeout: Duration::from_millis(500),
            ..Default::default()
        };
        Picker::from_query_stdio_with_options(options)
            .ok()
            .filter(|p| p.protocol_type() != ProtocolType::Halfblocks)
    } else {
        None
    };

    let result = (|| -> Result<()> {
        let mut app = App::new(cfg, picker)?;
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

fn graphics_terminal() -> bool {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    [
        "KONSOLE_VERSION",
        "KITTY_WINDOW_ID",
        "WEZTERM_EXECUTABLE",
        "GHOSTTY_RESOURCES_DIR",
    ]
    .iter()
    .any(|k| !env(k).is_empty())
        || ["WezTerm", "iTerm.app", "ghostty"].contains(&env("TERM_PROGRAM").as_str())
        || ["xterm-kitty", "foot", "xterm-ghostty"].contains(&env("TERM").as_str())
}
