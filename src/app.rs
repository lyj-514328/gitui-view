use crate::diff::{DiffView, DiffViewMode};
use crate::git::GitRepo;
use crate::log_tab::{self, LogTab};
use crate::stashes_tab::{self, StashesTab};
use crate::status_tab::{StatusFocus, StatusTab};
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Status,
    Log,
    Stashes,
}

pub struct App {
    pub repo: GitRepo,
    pub current_tab: Tab,
    pub status_tab: StatusTab,
    pub log_tab: LogTab,
    pub stashes_tab: StashesTab,
    pub diff_view: DiffView,
    pub theme: Theme,
    pub show_diff: bool,
    pub diff_fullscreen: bool,
    pub show_help: bool,
    pub diff_mode: DiffViewMode,
}

impl App {
    pub fn new(repo_path: &Path, theme: Theme) -> anyhow::Result<Self> {
        let repo = GitRepo::open(repo_path)?;
        let mut app = Self {
            repo,
            current_tab: Tab::Status,
            status_tab: StatusTab::new(),
            log_tab: LogTab::new(),
            stashes_tab: StashesTab::new(),
            diff_view: DiffView::new(),
            theme,
            show_diff: false,
            diff_fullscreen: false,
            show_help: false,
            diff_mode: DiffViewMode::SideBySide,
        };
        app.refresh_current_tab();
        Ok(app)
    }

    pub fn refresh_current_tab(&mut self) {
        match self.current_tab {
            Tab::Status => {
                self.status_tab.refresh(&self.repo);
                self.load_diff_for_selection();
            }
            Tab::Log => self.log_tab.refresh(&self.repo),
            Tab::Stashes => self.stashes_tab.refresh(&mut self.repo),
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Status => Tab::Log,
            Tab::Log => Tab::Stashes,
            Tab::Stashes => Tab::Status,
        };
        self.show_diff = false;
        self.refresh_current_tab();
    }

    pub fn prev_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Status => Tab::Stashes,
            Tab::Log => Tab::Status,
            Tab::Stashes => Tab::Log,
        };
        self.show_diff = false;
        self.refresh_current_tab();
    }

    pub fn toggle_diff(&mut self) {
        self.show_diff = !self.show_diff;
        if self.show_diff {
            self.load_diff_for_selection();
        }
    }

    pub fn log_tab_enter(&mut self) {
        let prev = self.log_tab.depth;
        if self.log_tab.enter() {
            match (prev, self.log_tab.depth) {
                (log_tab::LogDepth::Commits, log_tab::LogDepth::Details) => {
                    self.log_tab.load_files(&self.repo);
                }
                (_, log_tab::LogDepth::FilesDiff) => {
                    self.log_tab.load_diff_for_file(&mut self.diff_view, &self.repo);
                }
                _ => {}
            }
        }
    }

    pub fn log_tab_back(&mut self) {
        self.log_tab.back();
    }

    pub fn stash_tab_enter(&mut self) {
        let prev = self.stashes_tab.depth;
        if self.stashes_tab.enter() {
            match (prev, self.stashes_tab.depth) {
                (stashes_tab::StashDepth::List, stashes_tab::StashDepth::Details) => {
                    self.stashes_tab.load_files(&mut self.repo);
                }
                (_, stashes_tab::StashDepth::FilesDiff) => {
                    self.stashes_tab.load_diff_for_file(&mut self.diff_view, &mut self.repo);
                }
                _ => {}
            }
        }
    }

    pub fn stash_tab_back(&mut self) {
        self.stashes_tab.back();
    }

    pub fn is_any_diff_active(&self) -> bool {
        self.show_diff
            || (self.current_tab == Tab::Status && self.status_tab.focus == StatusFocus::Diff)
            || (self.current_tab == Tab::Log && self.log_tab.depth >= log_tab::LogDepth::FilesDiff)
            || (self.current_tab == Tab::Stashes && self.stashes_tab.depth >= stashes_tab::StashDepth::FilesDiff)
    }

    pub fn toggle_diff_fullscreen(&mut self) {
        self.diff_fullscreen = !self.diff_fullscreen;
    }

    pub fn toggle_diff_mode(&mut self) {
        self.diff_mode = match self.diff_mode {
            DiffViewMode::Inline => DiffViewMode::SideBySide,
            DiffViewMode::SideBySide => DiffViewMode::Inline,
        };
        self.diff_view.mode = self.diff_mode;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn move_down(&mut self) {
        if self.show_diff {
            self.diff_view.scroll_down(1);
        } else if self.current_tab == Tab::Status && self.status_tab.focus == StatusFocus::Diff {
            self.diff_view.scroll_down(1);
        } else if self.current_tab == Tab::Log && self.log_tab.depth == log_tab::LogDepth::Diff {
            self.diff_view.scroll_down(1);
        } else if self.current_tab == Tab::Stashes && self.stashes_tab.depth == stashes_tab::StashDepth::Diff {
            self.diff_view.scroll_down(1);
        } else {
            match self.current_tab {
                Tab::Status => {
                    self.status_tab.move_down();
                    self.load_diff_for_selection();
                }
                Tab::Log => {
                    self.log_tab.move_down();
                    if self.log_tab.depth == log_tab::LogDepth::Details {
                        self.log_tab.load_files(&self.repo);
                    }
                    if self.log_tab.depth == log_tab::LogDepth::FilesDiff {
                        self.log_tab.load_diff_for_file(&mut self.diff_view, &self.repo);
                    }
                }
                Tab::Stashes => {
                    self.stashes_tab.move_down();
                    if self.stashes_tab.depth == stashes_tab::StashDepth::Details {
                        self.stashes_tab.load_files(&mut self.repo);
                    }
                    if self.stashes_tab.depth == stashes_tab::StashDepth::FilesDiff {
                        self.stashes_tab.load_diff_for_file(&mut self.diff_view, &mut self.repo);
                    }
                }
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.show_diff {
            self.diff_view.scroll_up(1);
        } else if self.current_tab == Tab::Status && self.status_tab.focus == StatusFocus::Diff {
            self.diff_view.scroll_up(1);
        } else if self.current_tab == Tab::Log && self.log_tab.depth == log_tab::LogDepth::Diff {
            self.diff_view.scroll_up(1);
        } else if self.current_tab == Tab::Stashes && self.stashes_tab.depth == stashes_tab::StashDepth::Diff {
            self.diff_view.scroll_up(1);
        } else {
            match self.current_tab {
                Tab::Status => {
                    self.status_tab.move_up();
                    self.load_diff_for_selection();
                }
                Tab::Log => {
                    self.log_tab.move_up();
                    if self.log_tab.depth == log_tab::LogDepth::Details {
                        self.log_tab.load_files(&self.repo);
                    }
                    if self.log_tab.depth == log_tab::LogDepth::FilesDiff {
                        self.log_tab.load_diff_for_file(&mut self.diff_view, &self.repo);
                    }
                }
                Tab::Stashes => {
                    self.stashes_tab.move_up();
                    if self.stashes_tab.depth == stashes_tab::StashDepth::Details {
                        self.stashes_tab.load_files(&mut self.repo);
                    }
                    if self.stashes_tab.depth == stashes_tab::StashDepth::FilesDiff {
                        self.stashes_tab.load_diff_for_file(&mut self.diff_view, &mut self.repo);
                    }
                }
            }
        }
    }

    pub fn page_down(&mut self) {
        if self.show_diff {
            self.diff_view.page_down();
        } else if self.current_tab == Tab::Status && self.status_tab.focus == StatusFocus::Diff {
            self.diff_view.page_down();
        } else if self.current_tab == Tab::Log && self.log_tab.depth == log_tab::LogDepth::Diff {
            self.diff_view.page_down();
        } else if self.current_tab == Tab::Stashes && self.stashes_tab.depth == stashes_tab::StashDepth::Diff {
            self.diff_view.page_down();
        } else {
            match self.current_tab {
                Tab::Status => {}
                Tab::Log => {
                    self.log_tab.page_down();
                    if self.log_tab.depth == log_tab::LogDepth::Details {
                        self.log_tab.load_files(&self.repo);
                    }
                    if self.log_tab.depth == log_tab::LogDepth::FilesDiff {
                        self.log_tab.load_diff_for_file(&mut self.diff_view, &self.repo);
                    }
                }
                Tab::Stashes => {
                    self.stashes_tab.page_down();
                    if self.stashes_tab.depth == stashes_tab::StashDepth::Details {
                        self.stashes_tab.load_files(&mut self.repo);
                    }
                    if self.stashes_tab.depth == stashes_tab::StashDepth::FilesDiff {
                        self.stashes_tab.load_diff_for_file(&mut self.diff_view, &mut self.repo);
                    }
                }
            }
        }
    }

    pub fn page_up(&mut self) {
        if self.show_diff {
            self.diff_view.page_up();
        } else if self.current_tab == Tab::Status && self.status_tab.focus == StatusFocus::Diff {
            self.diff_view.page_up();
        } else if self.current_tab == Tab::Log && self.log_tab.depth == log_tab::LogDepth::Diff {
            self.diff_view.page_up();
        } else if self.current_tab == Tab::Stashes && self.stashes_tab.depth == stashes_tab::StashDepth::Diff {
            self.diff_view.page_up();
        } else {
            match self.current_tab {
                Tab::Status => {}
                Tab::Log => {
                    self.log_tab.page_up();
                    if self.log_tab.depth == log_tab::LogDepth::Details {
                        self.log_tab.load_files(&self.repo);
                    }
                    if self.log_tab.depth == log_tab::LogDepth::FilesDiff {
                        self.log_tab.load_diff_for_file(&mut self.diff_view, &self.repo);
                    }
                }
                Tab::Stashes => {
                    self.stashes_tab.page_up();
                    if self.stashes_tab.depth == stashes_tab::StashDepth::Details {
                        self.stashes_tab.load_files(&mut self.repo);
                    }
                    if self.stashes_tab.depth == stashes_tab::StashDepth::FilesDiff {
                        self.stashes_tab.load_diff_for_file(&mut self.diff_view, &mut self.repo);
                    }
                }
            }
        }
    }

    pub fn load_diff_for_selection(&mut self) {
        match self.current_tab {
            Tab::Status => {
                if let Some(path) = self.status_tab.current_file() {
                    let staged = self.status_tab.current_staged();
                    if let Ok(diff) = self.repo.get_workdir_diff(&path, staged) {
                        self.diff_view.set_diff(diff);
                        return;
                    }
                }
                self.diff_view.clear();
            }
            Tab::Log => {
                self.diff_view.clear();
            }
            Tab::Stashes => {
                self.stashes_tab.load_diff_for_file(&mut self.diff_view, &mut self.repo);
            }
        }
    }

    pub fn render(&self, f: &mut Frame) {
        let area = f.area();

        let diff_focused = (self.current_tab == Tab::Status && self.status_tab.focus == StatusFocus::Diff)
            || self.show_diff
            || (self.current_tab == Tab::Log && self.log_tab.depth == log_tab::LogDepth::Diff)
            || (self.current_tab == Tab::Stashes && self.stashes_tab.depth == stashes_tab::StashDepth::Diff);
        self.diff_view.focused.set(diff_focused);

        let tab_titles = vec![
            " Status [1] ",
            " Log [2] ",
            " Stashes [3] ",
        ];

        let tab_index = match self.current_tab {
            Tab::Status => 0,
            Tab::Log => 1,
            Tab::Stashes => 2,
        };

        let tabs = Tabs::new(
            tab_titles
                .iter()
                .enumerate()
                .map(|(i, title)| {
                    let style = if i == tab_index {
                        self.theme.tab_active_style()
                    } else {
                        self.theme.tab_inactive_style()
                    };
                    Line::from(Span::styled(*title, style))
                })
                .collect::<Vec<_>>(),
        )
        .select(tab_index);

        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.tab_bar_style());
        let header_inner = header_block.inner(main_layout[0]);
        f.render_widget(header_block, main_layout[0]);

        let repo_path = self.repo.workdir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        const DIVIDER_PAD_SPACES: usize = 2;
        const SIDE_PADS: usize = 2;
        const MARGIN_LEFT_AND_RIGHT: usize = 2;
        let tabs_natural_width: usize =
            tab_titles.iter().map(|t| t.len()).sum::<usize>()
                + tab_titles.len().saturating_sub(1)
                    * (1 + DIVIDER_PAD_SPACES)
                + SIDE_PADS + MARGIN_LEFT_AND_RIGHT;
        let tabs_width = (tabs_natural_width as u16).min(header_inner.width);
        let path_width = header_inner.width.saturating_sub(tabs_width);

        let header_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(tabs_width), Constraint::Fill(1)])
            .split(header_inner);

        f.render_widget(tabs, header_split[0]);

        if path_width > 3 && !repo_path.is_empty() {
            let truncated = if repo_path.len() as u16 > path_width {
                let keep = (path_width as usize).saturating_sub(3);
                format!("...{}", &repo_path[repo_path.len().saturating_sub(keep)..])
            } else {
                repo_path.clone()
            };
            let path_text = Line::from(Span::styled(truncated, self.theme.dim_text()));
            f.render_widget(
                Paragraph::new(path_text).alignment(ratatui::layout::Alignment::Right),
                header_split[1],
            );
        }

        let content_area = main_layout[1];

        if self.current_tab == Tab::Status {
            if self.status_tab.focus == StatusFocus::Diff && self.diff_fullscreen {
                self.diff_view.render(f, content_area, &self.theme);
            } else {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                    .split(content_area);

                self.status_tab.render(f, split[0], &self.theme);
                self.diff_view.render(f, split[1], &self.theme);
            }
        } else if self.show_diff {
            let split = if self.diff_fullscreen {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(0), Constraint::Min(1)])
                    .split(content_area)
            } else {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                    .split(content_area)
            };

            self.render_tab_content(f, split[0]);
            self.diff_view.render(f, split[1], &self.theme);
        } else {
            self.render_tab_content(f, content_area);
        }

        let mode_text = if self.current_tab == Tab::Status {
            if self.status_tab.focus == StatusFocus::Diff {
                format!(
                    " [{}]{} | q:quit | h:help | f:fullscreen | m:toggle mode({}) | \u{2191}\u{2193}:scroll ",
                    match self.diff_mode {
                        DiffViewMode::Inline => "inline",
                        DiffViewMode::SideBySide => "side-by-side",
                    },
                    if self.diff_fullscreen { " [F]" } else { "" },
                    match self.diff_mode {
                        DiffViewMode::Inline => "inline",
                        DiffViewMode::SideBySide => "side-by-side",
                    },
                )
            } else {
                " q:quit | h:help | \u{2191}\u{2193}:navigate | PgUp/PgDn:page | \u{2190}\u{2192}:switch panel | Enter:open diff | 1-3:goto tab ".to_string()
            }
        } else if self.is_any_diff_active() {
            format!(
                " [{}]{} | q:quit | h:help | d:toggle diff | f:fullscreen | m:toggle mode({}) | \u{2191}\u{2193}:scroll ",
                match self.diff_mode {
                    DiffViewMode::Inline => "inline",
                    DiffViewMode::SideBySide => "side-by-side",
                },
                if self.diff_fullscreen { " [F]" } else { "" },
                match self.diff_mode {
                    DiffViewMode::Inline => "inline",
                    DiffViewMode::SideBySide => "side-by-side",
                },
            )
        } else {
            " q:quit | h:help | d:show diff | ↑↓:navigate | PgUp/PgDn:page | Tab:next | 1-3:goto tab ".to_string()
        };

        let status_line = Line::from(Span::styled(mode_text, self.theme.dim_text()));
        f.render_widget(
            Paragraph::new(status_line).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style()),
            ),
            main_layout[2],
        );

        if self.show_help {
            self.render_help(f);
        }
    }

    fn render_tab_content(&self, f: &mut Frame, area: Rect) {
        match self.current_tab {
            Tab::Status => self.status_tab.render(f, area, &self.theme),
            Tab::Log => {
                match self.log_tab.depth {
                    log_tab::LogDepth::Commits | log_tab::LogDepth::Details => {
                        self.log_tab.render(f, area, &self.theme);
                    }
                    log_tab::LogDepth::FilesDiff => {
                        let split = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                            .split(area);
                        self.log_tab.render(f, split[0], &self.theme);
                        self.diff_view.render(f, split[1], &self.theme);
                    }
                    log_tab::LogDepth::Diff => {
                        self.diff_view.render(f, area, &self.theme);
                    }
                }
            }
            Tab::Stashes => {
                match self.stashes_tab.depth {
                    stashes_tab::StashDepth::List | stashes_tab::StashDepth::Details => {
                        self.stashes_tab.render(f, area, &self.theme);
                    }
                    stashes_tab::StashDepth::FilesDiff => {
                        let split = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                            .split(area);
                        self.stashes_tab.render(f, split[0], &self.theme);
                        self.diff_view.render(f, split[1], &self.theme);
                    }
                    stashes_tab::StashDepth::Diff => {
                        self.diff_view.render(f, area, &self.theme);
                    }
                }
            }
        }
    }

    fn render_help(&self, f: &mut Frame) {
        let area = f.area();
        let help_area = Rect {
            x: area.width.saturating_sub(50) / 2,
            y: area.height.saturating_sub(19) / 2,
            width: 50.min(area.width),
            height: 19.min(area.height),
        };

        let help_lines = vec![
            Line::from(Span::styled(" Help ", self.theme.title())),
            Line::from(Span::styled("", self.theme.normal())),
            Line::from(vec![
                Span::styled("  q          ", self.theme.help_key()),
                Span::styled("Quit", self.theme.help_desc()),
            ]),
            Line::from(vec![
                Span::styled("  h          ", self.theme.help_key()),
                Span::styled("Toggle this help", self.theme.help_desc()),
            ]),
            Line::from(vec![
                Span::styled("  Tab / \u{2190} \u{2192}  ", self.theme.help_key()),
                Span::styled("Switch tabs", self.theme.help_desc()),
            ]),
            Line::from(vec![
                Span::styled("  1-3        ", self.theme.help_key()),
                Span::styled("Go to tab by number", self.theme.help_desc()),
            ]),
            Line::from(vec![
                Span::styled("  ↑/↓        ", self.theme.help_key()),
                Span::styled("Navigate / scroll", self.theme.help_desc()),
            ]),
            Line::from(vec![
                Span::styled("  PgUp/PgDn  ", self.theme.help_key()),
                Span::styled("Page up / page down", self.theme.help_desc()),
            ]),
            Line::from(vec![
                Span::styled("  d          ", self.theme.help_key()),
                Span::styled("Toggle diff view", self.theme.help_desc()),
            ]),
            Line::from(vec![
                Span::styled("  m          ", self.theme.help_key()),
                Span::styled("Toggle inline/side-by-side", self.theme.help_desc()),
            ]),
            Line::from(Span::styled("", self.theme.normal())),
            Line::from(Span::styled(" Press any key to close ", self.theme.dim_text())),
        ];

        let help_block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_focused_style())
            .style(self.theme.normal());

        f.render_widget(
            Paragraph::new(help_lines)
                .block(help_block)
                .alignment(ratatui::layout::Alignment::Left),
            help_area,
        );
    }
}
