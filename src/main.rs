mod app;
mod diff;
mod git;
mod log_tab;
mod stashes_tab;
mod status_tab;
mod theme;

use crate::app::App;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::Path};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut repo_path = std::env::current_dir()?;
    let mut theme_path = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--theme" | "-t" => {
                i += 1;
                theme_path = args.get(i).map(|s| Path::new(s).to_path_buf());
            }
            _ => {
                if !args[i].starts_with('-') {
                    repo_path = Path::new(&args[i]).to_path_buf();
                }
            }
        }
        i += 1;
    }

    let theme = if let Some(path) = &theme_path {
        Theme::from_path(path).unwrap_or_else(|e| {
            eprintln!("Warning: failed to load theme from {:?}: {e}", path);
            eprintln!("Using default theme");
            Theme::dark()
        })
    } else {
        Theme::dark()
    };

    let app = App::new(&repo_path, theme)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') => {
                    return Ok(());
                }
                KeyCode::Char('h') => {
                    app.toggle_help();
                }
                KeyCode::Char('d') => {
                    app.toggle_diff();
                }
                KeyCode::Char('m') => {
                    app.toggle_diff_mode();
                }
                KeyCode::Char('1') => {
                    app.current_tab = app::Tab::Status;
                    app.show_diff = false;
                    app.refresh_current_tab();
                }
                KeyCode::Char('2') => {
                    app.current_tab = app::Tab::Log;
                    app.show_diff = false;
                    app.refresh_current_tab();
                }
                KeyCode::Char('3') => {
                    app.current_tab = app::Tab::Stashes;
                    app.show_diff = false;
                    app.refresh_current_tab();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.move_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.move_down();
                }
                KeyCode::Tab => {
                    app.next_tab();
                }
                KeyCode::BackTab => {
                    app.prev_tab();
                }
                KeyCode::Right => {
                    app.next_tab();
                }
                KeyCode::Left => {
                    app.prev_tab();
                }
                KeyCode::Esc => {
                    if app.show_help {
                        app.toggle_help();
                    } else if app.show_diff {
                        app.toggle_diff();
                    } else if matches!(app.current_tab, app::Tab::Log) && app.log_tab.show_files {
                        app.log_tab.close_files();
                    } else if matches!(app.current_tab, app::Tab::Stashes) && app.stashes_tab.show_files {
                        app.stashes_tab.close_files();
                    }
                }
                KeyCode::Enter => {
                    if matches!(app.current_tab, app::Tab::Log) && !app.log_tab.show_files {
                        app.log_tab.toggle_files(&app.repo);
                    } else if matches!(app.current_tab, app::Tab::Stashes) && !app.stashes_tab.show_files {
                        app.stashes_tab.toggle_files(&mut app.repo);
                    } else {
                        app.toggle_diff();
                    }
                }
                _ => {
                    if app.show_help {
                        app.toggle_help();
                    }
                }
            }
        }
    }
}
