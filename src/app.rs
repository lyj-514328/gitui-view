use crate::diff::{DiffView, DiffViewMode};
use crate::git::GitRepo;
use crate::log_tab::LogTab;
use crate::stashes_tab::StashesTab;
use crate::status_tab::StatusTab;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use std::path::Path;

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
            show_help: false,
            diff_mode: DiffViewMode::SideBySide,
        };
        app.refresh_current_tab();
        Ok(app)
    }

    pub fn refresh_current_tab(&mut self) {
        match self.current_tab {
            Tab::Status => self.status_tab.refresh(&self.repo),
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
        } else {
            match self.current_tab {
                Tab::Status => self.status_tab.move_down(),
                Tab::Log => {
                    if self.log_tab.show_files {
                        self.log_tab.file_move_down();
                    } else {
                        self.log_tab.move_down();
                    }
                }
                Tab::Stashes => {
                    if self.stashes_tab.show_files {
                        self.stashes_tab.file_move_down();
                    } else {
                        self.stashes_tab.move_down();
                    }
                }
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.show_diff {
            self.diff_view.scroll_up(1);
        } else {
            match self.current_tab {
                Tab::Status => self.status_tab.move_up(),
                Tab::Log => {
                    if self.log_tab.show_files {
                        self.log_tab.file_move_up();
                    } else {
                        self.log_tab.move_up();
                    }
                }
                Tab::Stashes => {
                    if self.stashes_tab.show_files {
                        self.stashes_tab.file_move_up();
                    } else {
                        self.stashes_tab.move_up();
                    }
                }
            }
        }
    }

    fn load_diff_for_selection(&mut self) {
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
                if self.log_tab.show_files {
                    if let Some(path) = self.log_tab.current_file_path() {
                        if let Some(commit_id) = self.log_tab.current_commit_id() {
                            if let Ok(diffs) = self.repo.get_commit_diff(&commit_id) {
                                for diff in diffs {
                                    let diff_path = if !diff.new_path.is_empty() {
                                        diff.new_path.clone()
                                    } else {
                                        diff.old_path.clone()
                                    };
                                    if diff_path == path {
                                        self.diff_view.set_diff(diff);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
                self.diff_view.clear();
            }
            Tab::Stashes => {
                if self.stashes_tab.show_files {
                    if let Some(path) = self.stashes_tab.current_file_path() {
                        if let Some(index) = self.stashes_tab.current_stash_index() {
                            if let Ok(diffs) = self.repo.get_stash_diff(index) {
                                for diff in diffs {
                                    let diff_path = if !diff.new_path.is_empty() {
                                        diff.new_path.clone()
                                    } else {
                                        diff.old_path.clone()
                                    };
                                    if diff_path == path {
                                        self.diff_view.set_diff(diff);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
                self.diff_view.clear();
            }
        }
    }

    pub fn render(&self, f: &mut Frame) {
        let area = f.area();

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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.tab_bar_style()),
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

        f.render_widget(tabs, main_layout[0]);

        let content_area = main_layout[1];

        if self.show_diff {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
                .split(content_area);

            self.render_tab_content(f, split[0]);
            self.diff_view.render(f, split[1], &self.theme);
        } else {
            self.render_tab_content(f, content_area);
        }

        let mode_text = if self.show_diff {
            format!(
                " [{}] | q:quit | h:help | d:toggle diff | m:toggle mode({}) | \u{2191}\u{2193}:scroll ",
                match self.diff_mode {
                    DiffViewMode::Inline => "inline",
                    DiffViewMode::SideBySide => "side-by-side",
                },
                match self.diff_mode {
                    DiffViewMode::Inline => "inline",
                    DiffViewMode::SideBySide => "side-by-side",
                },
            )
        } else {
            " q:quit | h:help | d:show diff | \u{2191}\u{2193}:navigate | Tab:next | 1-3:goto tab ".to_string()
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
            Tab::Log => self.log_tab.render(f, area, &self.theme),
            Tab::Stashes => self.stashes_tab.render(f, area, &self.theme),
        }
    }

    fn render_help(&self, f: &mut Frame) {
        let area = f.area();
        let help_area = Rect {
            x: area.width.saturating_sub(50) / 2,
            y: area.height.saturating_sub(18) / 2,
            width: 50.min(area.width),
            height: 18.min(area.height),
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
                Span::styled("  \u{2191}/\u{2193}        ", self.theme.help_key()),
                Span::styled("Navigate / scroll", self.theme.help_desc()),
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
