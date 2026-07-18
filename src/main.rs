mod align;
mod app;
mod diff;
mod diff_engine;
mod edits;
mod git;
mod log_tab;
mod stashes_tab;
mod status_tab;
use crate::status_tab::StatusFocus;
mod theme;

use crate::stashes_tab::StashDepth;

use std::time::Duration;

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
use terminal_colorsaurus::{color_scheme, QueryOptions};

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
            if is_dark_mode() { Theme::dark() } else { Theme::light() }
        })
    } else {
        if is_dark_mode() { Theme::dark() } else { Theme::light() }
    };

    diff_engine::init_bat_assets();

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
            let is_scroll = matches!(
                key.code,
                KeyCode::Up | KeyCode::Down | KeyCode::Char('k') | KeyCode::Char('j')
            );

            match key.code {
                KeyCode::Char('q') => {
                    return Ok(());
                }
                KeyCode::Char('h') => {
                    app.toggle_help();
                }
                KeyCode::Char('d') => {
                    match app.current_tab {
                        app::Tab::Status => {
                            if app.status_tab.focus == StatusFocus::Diff {
                                app.status_tab.focus = StatusFocus::Unstaged;
                                app.diff_fullscreen = false;
                            } else if app.status_tab.current_file().is_some() {
                                app.status_tab.focus = StatusFocus::Diff;
                                app.diff_fullscreen = true;
                            }
                        }
                        app::Tab::Log => {
                            if app.log_tab.depth >= log_tab::LogDepth::FilesDiff {
                                app.log_tab_back();
                            } else {
                                app.log_tab_enter();
                            }
                        }
                        app::Tab::Stashes => {
                            if app.stashes_tab.depth >= StashDepth::FilesDiff {
                                app.stash_tab_back();
                            } else {
                                app.stash_tab_enter();
                            }
                        }
                    }
                }
                KeyCode::Char('f') => {
                    if app.current_tab == app::Tab::Status && app.status_tab.focus == StatusFocus::Diff {
                        app.diff_fullscreen = !app.diff_fullscreen;
                    } else {
                        app.toggle_diff_fullscreen();
                    }
                }
                KeyCode::Char('m') => {
                    app.toggle_diff_mode();
                }
                KeyCode::Char('1') => {
                    app.current_tab = app::Tab::Status;
                    app.show_diff = false;
                    app.diff_fullscreen = false;
                    app.refresh_current_tab();
                }
                KeyCode::Char('2') => {
                    app.current_tab = app::Tab::Log;
                    app.show_diff = false;
                    app.diff_fullscreen = false;
                    app.refresh_current_tab();
                }
                KeyCode::Char('3') => {
                    app.current_tab = app::Tab::Stashes;
                    app.show_diff = false;
                    app.diff_fullscreen = false;
                    app.refresh_current_tab();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.move_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.move_down();
                }
                KeyCode::PageUp => {
                    app.page_up();
                }
                KeyCode::PageDown => {
                    app.page_down();
                }
                KeyCode::Home => {
                    app.go_home();
                }
                KeyCode::End => {
                    app.go_end();
                }
                KeyCode::Tab => {
                    app.next_tab();
                }
                KeyCode::BackTab => {
                    app.prev_tab();
                }
                KeyCode::Right => {
                    if app.current_tab == app::Tab::Status {
                        app.status_tab.focus_right();
                        if app.status_tab.focus == StatusFocus::Diff {
                            app.diff_fullscreen = true;
                        }
                    } else if app.current_tab == app::Tab::Log {
                        app.log_tab_enter();
                    } else if app.current_tab == app::Tab::Stashes {
                        app.stash_tab_enter();
                    }
                }
                KeyCode::Left => {
                    if app.current_tab == app::Tab::Status {
                        if app.status_tab.focus == StatusFocus::Diff {
                            app.status_tab.focus = StatusFocus::Unstaged;
                            app.diff_fullscreen = false;
                        } else {
                            app.status_tab.focus_left();
                        }
                    }
                }
                KeyCode::Esc => {
                    if app.show_help {
                        app.toggle_help();
                    } else if app.current_tab == app::Tab::Status
                        && app.status_tab.focus == StatusFocus::Diff
                    {
                        app.status_tab.focus = StatusFocus::Unstaged;
                        app.diff_fullscreen = false;
                    } else if app.current_tab == app::Tab::Log
                        && app.log_tab.depth != log_tab::LogDepth::Commits
                    {
                        app.log_tab_back();
                    } else if app.show_diff {
                        app.toggle_diff();
                    } else if app.current_tab == app::Tab::Stashes
                        && app.stashes_tab.depth != StashDepth::List
                    {
                        app.stash_tab_back();
                    }
                }
                KeyCode::Enter => {
                    match app.current_tab {
                        app::Tab::Status => {
                            if app.status_tab.focus == StatusFocus::Diff {
                                app.status_tab.focus = StatusFocus::Unstaged;
                                app.diff_fullscreen = false;
                            } else if app.status_tab.current_file().is_some() {
                                app.status_tab.focus = StatusFocus::Diff;
                                app.diff_fullscreen = true;
                            }
                        }
                        app::Tab::Log => {
                            app.log_tab_enter();
                        }
                        app::Tab::Stashes => {
                            app.stash_tab_enter();
                        }
                    }
                }
                _ => {
                    if app.show_help {
                        app.toggle_help();
                    }
                }
            }

            // coalesce: drain queued scroll events in same direction
            if is_scroll {
                let scroll_code = key.code;
                while event::poll(Duration::ZERO)? {
                    if let Event::Key(next) = event::read()? {
                        if next.kind != KeyEventKind::Press {
                            continue;
                        }
                        let same_dir = matches!(
                            (scroll_code, next.code),
                            (KeyCode::Up, KeyCode::Up)
                                | (KeyCode::Up, KeyCode::Char('k'))
                                | (KeyCode::Char('k'), KeyCode::Up)
                                | (KeyCode::Char('k'), KeyCode::Char('k'))
                                | (KeyCode::Down, KeyCode::Down)
                                | (KeyCode::Down, KeyCode::Char('j'))
                                | (KeyCode::Char('j'), KeyCode::Down)
                                | (KeyCode::Char('j'), KeyCode::Char('j'))
                        );
                        if !same_dir {
                            // put it back — can't, but this is very unlikely in practice
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn is_dark_mode() -> bool {
    color_scheme(QueryOptions::default())
        .map(|cs| matches!(cs, terminal_colorsaurus::ColorScheme::Dark))
        .unwrap_or(true)
}
