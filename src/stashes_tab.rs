use crate::diff::DiffView;
use crate::git::{GitRepo, StashInfo, StatusType};
use crate::theme::Theme;
use chrono::{Local, TimeZone};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cell::Cell;
use std::cmp;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StashDepth {
    List,
    Details,
    FilesDiff,
    Diff,
}

pub struct StashesTab {
    pub stashes: Vec<StashInfo>,
    pub selected: usize,
    pub scroll: usize,
    pub files: Vec<(String, StatusType)>,
    pub file_selected: usize,
    pub file_scroll: usize,
    pub depth: StashDepth,
    stash_list_height: Cell<usize>,
    file_list_height: Cell<usize>,
}

impl StashesTab {
    pub fn new() -> Self {
        Self {
            stashes: Vec::new(),
            selected: 0,
            scroll: 0,
            files: Vec::new(),
            file_selected: 0,
            file_scroll: 0,
            depth: StashDepth::List,
            stash_list_height: Cell::new(0),
            file_list_height: Cell::new(0),
        }
    }

    pub fn refresh(&mut self, repo: &mut GitRepo) {
        if let Ok(stashes) = repo.get_stashes() {
            self.stashes = stashes;
            self.selected = 0;
            self.scroll = 0;
        }
        self.files.clear();
        self.file_selected = 0;
        self.file_scroll = 0;
        self.depth = StashDepth::List;
    }

    pub fn load_files(&mut self, repo: &mut GitRepo) {
        if let Some(index) = self.current_stash_index() {
            if let Ok(diffs) = repo.get_stash_diff(index) {
                let mut files = Vec::new();
                for diff in &diffs {
                    let path = if !diff.new_path.is_empty() {
                        diff.new_path.clone()
                    } else {
                        diff.old_path.clone()
                    };
                    files.push((path, diff.status.clone()));
                }
                self.files = files;
                self.file_selected = 0;
                self.file_scroll = 0;
            }
        }
    }

    pub fn enter(&mut self) -> bool {
        match self.depth {
            StashDepth::List => {
                if !self.stashes.is_empty() {
                    self.depth = StashDepth::Details;
                    true
                } else {
                    false
                }
            }
            StashDepth::Details => {
                if !self.files.is_empty() {
                    self.depth = StashDepth::FilesDiff;
                    true
                } else {
                    false
                }
            }
            StashDepth::FilesDiff => {
                if !self.files.is_empty() {
                    self.depth = StashDepth::Diff;
                    true
                } else {
                    false
                }
            }
            StashDepth::Diff => false,
        }
    }

    pub fn back(&mut self) {
        self.depth = match self.depth {
            StashDepth::List => StashDepth::List,
            StashDepth::Details => StashDepth::List,
            StashDepth::FilesDiff => StashDepth::Details,
            StashDepth::Diff => StashDepth::FilesDiff,
        };
    }

    pub fn move_down(&mut self) {
        match self.depth {
            StashDepth::List | StashDepth::Details => {
                let max = self.stashes.len().saturating_sub(1);
                self.selected = cmp::min(self.selected + 1, max);
                self.ensure_stash_visible();
            }
            StashDepth::FilesDiff => {
                let max = self.files.len().saturating_sub(1);
                self.file_selected = cmp::min(self.file_selected + 1, max);
                self.ensure_file_visible();
            }
            StashDepth::Diff => {}
        }
    }

    pub fn move_up(&mut self) {
        match self.depth {
            StashDepth::List | StashDepth::Details => {
                self.selected = self.selected.saturating_sub(1);
                self.ensure_stash_visible();
            }
            StashDepth::FilesDiff => {
                self.file_selected = self.file_selected.saturating_sub(1);
                self.ensure_file_visible();
            }
            StashDepth::Diff => {}
        }
    }

    fn ensure_stash_visible(&mut self) {
        let height = self.stash_list_height.get();
        if height == 0 {
            return;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + height {
            self.scroll = self.selected + 1 - height;
        }
    }

    fn ensure_file_visible(&mut self) {
        let height = self.file_list_height.get();
        if height == 0 {
            return;
        }
        if self.file_selected < self.file_scroll {
            self.file_scroll = self.file_selected;
        } else if self.file_selected >= self.file_scroll + height {
            self.file_scroll = self.file_selected + 1 - height;
        }
    }

    pub fn page_down(&mut self) {
        match self.depth {
            StashDepth::List | StashDepth::Details => {
                let page = self.stash_list_height.get().max(1);
                let max = self.stashes.len().saturating_sub(1);
                self.selected = cmp::min(self.selected + page, max);
                self.ensure_stash_visible();
            }
            StashDepth::FilesDiff => {
                let page = self.file_list_height.get().max(1);
                let max = self.files.len().saturating_sub(1);
                self.file_selected = cmp::min(self.file_selected + page, max);
                self.ensure_file_visible();
            }
            StashDepth::Diff => {}
        }
    }

    pub fn page_up(&mut self) {
        match self.depth {
            StashDepth::List | StashDepth::Details => {
                let page = self.stash_list_height.get().max(1);
                self.selected = self.selected.saturating_sub(page);
                self.ensure_stash_visible();
            }
            StashDepth::FilesDiff => {
                let page = self.file_list_height.get().max(1);
                self.file_selected = self.file_selected.saturating_sub(page);
                self.ensure_file_visible();
            }
            StashDepth::Diff => {}
        }
    }

    pub fn go_home(&mut self) {
        match self.depth {
            StashDepth::List | StashDepth::Details => {
                self.selected = 0;
                self.ensure_stash_visible();
            }
            StashDepth::FilesDiff => {
                self.file_selected = 0;
                self.ensure_file_visible();
            }
            StashDepth::Diff => {}
        }
    }

    pub fn go_end(&mut self) {
        match self.depth {
            StashDepth::List | StashDepth::Details => {
                self.selected = self.stashes.len().saturating_sub(1);
                self.ensure_stash_visible();
            }
            StashDepth::FilesDiff => {
                self.file_selected = self.files.len().saturating_sub(1);
                self.ensure_file_visible();
            }
            StashDepth::Diff => {}
        }
    }

    pub fn current_stash_index(&self) -> Option<usize> {
        self.stashes.get(self.selected).map(|s| s.index)
    }

    pub fn current_file_path(&self) -> Option<String> {
        self.files.get(self.file_selected).map(|(path, _)| path.clone())
    }

    pub fn load_diff_for_file(&self, diff_view: &mut DiffView, repo: &mut GitRepo) {
        if let Some(path) = self.current_file_path() {
            if let Some(index) = self.current_stash_index() {
                if let Ok(diffs) = repo.get_stash_diff(index) {
                    for diff in diffs {
                        let dp = if !diff.new_path.is_empty() {
                            diff.new_path.clone()
                        } else {
                            diff.old_path.clone()
                        };
                        if dp == path {
                            diff_view.set_diff(diff);
                            return;
                        }
                    }
                }
            }
        }
        diff_view.clear();
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        match self.depth {
            StashDepth::List => {
                self.render_stash_list(f, area, theme, true);
            }
            StashDepth::Details => {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                    .split(area);

                self.render_stash_list(f, split[0], theme, true);
                let right = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                    .split(split[1]);
                self.render_stash_info(f, right[0], theme);
                self.render_file_list(f, right[1], theme, false);
            }
            StashDepth::FilesDiff | StashDepth::Diff => {
                let split = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                    .split(area);
                self.render_stash_info(f, split[0], theme);
                self.render_file_list(f, split[1], theme, self.depth == StashDepth::FilesDiff);
            }
        }
    }

    fn render_stash_list(&self, f: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let border_style = if focused {
            theme.border_focused_style()
        } else {
            theme.border_style()
        };
        let total = self.stashes.len();
        let remaining = total.saturating_sub(self.selected);
        let title = if total > 0 {
            format!(" Stashes {}/{} ", remaining, total)
        } else {
            " Stashes ".to_string()
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.stash_list_height.set(inner.height as usize);

        let mut lines: Vec<Line> = Vec::new();

        for (i, stash) in self.stashes.iter().enumerate() {
            let is_selected = i == self.selected;

            let dt = Local
                .timestamp_opt(stash.time, 0)
                .latest()
                .map(|t| t.format("%Y-%m-%d").to_string())
                .unwrap_or_default();

            let number_style = theme.stash_msg(is_selected);
            let date_style = if is_selected {
                theme.selected()
            } else {
                Style::default().fg(theme.commit_date)
            };
            let author_style = if is_selected {
                theme.selected()
            } else {
                Style::default().fg(theme.commit_author)
            };
            let msg_style = theme.commit_msg(is_selected);

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" stash@{{{}}} ", stash.index),
                    number_style,
                ),
                Span::styled(format!(" {} ", dt), date_style),
                Span::styled(format!(" {} ", stash.author), author_style),
                Span::styled(format!("  {}", stash.message), msg_style),
            ]));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                " (no stashes)",
                theme.dim_text(),
            )));
        }

        let visible: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll)
            .take(inner.height as usize)
            .collect();

        f.render_widget(Paragraph::new(visible), inner);
    }

    fn render_stash_info(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Info ")
            .borders(Borders::ALL)
            .border_style(theme.border_style());
        let inner = block.inner(area);
        f.render_widget(block, area);

        if let Some(stash) = self.stashes.get(self.selected) {
            let dt = Local
                .timestamp_opt(stash.time, 0)
                .latest()
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default();

            let label_style = theme.dim_text();
            let value_style = Style::default().fg(theme.commit_hash);

            let lines = vec![
                Line::from(vec![
                    Span::styled("Index:  ", label_style),
                    Span::styled(format!("stash@{{{}}}", stash.index), value_style),
                ]),
                Line::from(vec![
                    Span::styled("Date:   ", label_style),
                    Span::styled(dt, value_style),
                ]),
                Line::from(vec![
                    Span::styled("Hash:   ", label_style),
                    Span::styled(stash.commit_id.clone(), value_style),
                ]),
                Line::from(Span::styled("", theme.normal())),
                Line::from(Span::styled(
                    stash.message.clone(),
                    Style::default().fg(theme.commit_msg),
                )),
            ];

            f.render_widget(Paragraph::new(lines), inner);
        } else {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " (no stash selected)",
                    theme.dim_text(),
                ))),
                inner,
            );
        }
    }

    fn status_char(st: &StatusType) -> &'static str {
        match st {
            StatusType::Added => "+",
            StatusType::Modified => "M",
            StatusType::Deleted => "-",
            StatusType::Renamed => "R",
            StatusType::Copied => "C",
            StatusType::Untracked => "?",
            StatusType::TypeChange => "T",
        }
    }

    fn file_style(st: &StatusType, selected: bool, theme: &Theme) -> Style {
        if selected {
            return theme.selected();
        }
        let fg = match st {
            StatusType::Added => Color::LightGreen,
            StatusType::Modified => Color::Yellow,
            StatusType::Deleted => Color::LightRed,
            StatusType::Renamed => Color::LightMagenta,
            StatusType::Copied => Color::LightMagenta,
            StatusType::Untracked => Color::DarkGray,
            StatusType::TypeChange => Color::Yellow,
        };
        Style::default().fg(fg)
    }

    fn render_file_list(&self, f: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let border_style = if focused {
            theme.border_focused_style()
        } else {
            theme.border_style()
        };
        let total = self.files.len();
        let title = if total > 0 {
            format!(" Files {}/{} ", self.file_selected + 1, total)
        } else {
            " Files ".to_string()
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.file_list_height.set(inner.height as usize);

        let mut lines: Vec<Line> = Vec::new();

        for (i, (path, status)) in self.files.iter().enumerate() {
            let is_selected = i == self.file_selected && focused;
            let marker = if is_selected { ">" } else { " " };
            let sc = Self::status_char(status);
            let style = Self::file_style(status, is_selected, theme);

            lines.push(Line::from(Span::styled(
                format!(" {} {} {}", marker, sc, path),
                style,
            )));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                " (no files)",
                theme.dim_text(),
            )));
        }

        let visible: Vec<Line> = lines
            .into_iter()
            .skip(self.file_scroll)
            .take(inner.height as usize)
            .collect();

        f.render_widget(Paragraph::new(visible), inner);
    }
}
