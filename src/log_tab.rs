use crate::diff::DiffView;
use crate::git::{CommitInfo, GitRepo};
use crate::theme::Theme;
use chrono::{Local, TimeZone};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cell::Cell;
use std::cmp;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogDepth {
    Commits,
    Details,
    FilesDiff,
    Diff,
}

pub struct LogTab {
    pub commits: Vec<CommitInfo>,
    pub selected: usize,
    pub scroll: usize,
    pub files: Vec<String>,
    pub file_selected: usize,
    pub file_scroll: usize,
    pub depth: LogDepth,
    commit_list_height: Cell<usize>,
    file_list_height: Cell<usize>,
}

impl LogTab {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            selected: 0,
            scroll: 0,
            files: Vec::new(),
            file_selected: 0,
            file_scroll: 0,
            depth: LogDepth::Commits,
            commit_list_height: Cell::new(0),
            file_list_height: Cell::new(0),
        }
    }

    pub fn refresh(&mut self, repo: &GitRepo) {
        if let Ok(commits) = repo.get_commits(200) {
            self.commits = commits;
            self.selected = 0;
            self.scroll = 0;
        }
        self.files.clear();
        self.file_selected = 0;
        self.file_scroll = 0;
        self.depth = LogDepth::Commits;
    }

    pub fn load_files(&mut self, repo: &GitRepo) {
        if let Some(commit_id) = self.current_commit_id() {
            if let Ok(diffs) = repo.get_commit_diff(&commit_id) {
                let mut files = Vec::new();
                for diff in &diffs {
                    let path = if !diff.new_path.is_empty() {
                        diff.new_path.clone()
                    } else {
                        diff.old_path.clone()
                    };
                    files.push(path);
                }
                self.files = files;
                self.file_selected = 0;
                self.file_scroll = 0;
            }
        }
    }

    pub fn enter(&mut self) -> bool {
        match self.depth {
            LogDepth::Commits => {
                if !self.commits.is_empty() {
                    self.depth = LogDepth::Details;
                    true
                } else {
                    false
                }
            }
            LogDepth::Details => {
                if !self.files.is_empty() {
                    self.depth = LogDepth::FilesDiff;
                    true
                } else {
                    false
                }
            }
            LogDepth::FilesDiff => {
                if !self.files.is_empty() {
                    self.depth = LogDepth::Diff;
                    true
                } else {
                    false
                }
            }
            LogDepth::Diff => false,
        }
    }

    pub fn back(&mut self) {
        self.depth = match self.depth {
            LogDepth::Commits => LogDepth::Commits,
            LogDepth::Details => LogDepth::Commits,
            LogDepth::FilesDiff => LogDepth::Details,
            LogDepth::Diff => LogDepth::FilesDiff,
        };
    }

    pub fn move_down(&mut self) {
        match self.depth {
            LogDepth::Commits | LogDepth::Details => {
                let max = self.commits.len().saturating_sub(1);
                self.selected = cmp::min(self.selected + 1, max);
                self.ensure_commit_visible();
            }
            LogDepth::FilesDiff => {
                let max = self.files.len().saturating_sub(1);
                self.file_selected = cmp::min(self.file_selected + 1, max);
                self.ensure_file_visible();
            }
            LogDepth::Diff => {}
        }
    }

    pub fn move_up(&mut self) {
        match self.depth {
            LogDepth::Commits | LogDepth::Details => {
                self.selected = self.selected.saturating_sub(1);
                self.ensure_commit_visible();
            }
            LogDepth::FilesDiff => {
                self.file_selected = self.file_selected.saturating_sub(1);
                self.ensure_file_visible();
            }
            LogDepth::Diff => {}
        }
    }

    fn ensure_commit_visible(&mut self) {
        let height = self.commit_list_height.get();
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

    pub fn current_commit_id(&self) -> Option<String> {
        self.commits.get(self.selected).map(|c| c.id.clone())
    }

    pub fn current_file_path(&self) -> Option<String> {
        self.files.get(self.file_selected).cloned()
    }

    pub fn load_diff_for_file(&self, diff_view: &mut DiffView, repo: &GitRepo) {
        if let Some(path) = self.current_file_path() {
            if let Some(commit_id) = self.current_commit_id() {
                if let Ok(diffs) = repo.get_commit_diff(&commit_id) {
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
            LogDepth::Commits => {
                self.render_commit_list(f, area, theme, true);
            }
            LogDepth::Details => {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
                    .split(area);

                self.render_commit_list(f, split[0], theme, true);
                let right = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                    .split(split[1]);
                self.render_commit_info(f, right[0], theme);
                self.render_file_list(f, right[1], theme, false);
            }
            LogDepth::FilesDiff | LogDepth::Diff => {
                let split = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                    .split(area);
                self.render_commit_info(f, split[0], theme);
                self.render_file_list(f, split[1], theme, self.depth == LogDepth::FilesDiff);
            }
        }
    }

    fn render_commit_list(&self, f: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let border_style = if focused {
            theme.border_focused_style()
        } else {
            theme.border_style()
        };
        let block = Block::default()
            .title(" Log ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.commit_list_height.set(inner.height as usize);

        let mut lines: Vec<Line> = Vec::new();

        for (i, commit) in self.commits.iter().enumerate() {
            let is_selected = i == self.selected;
            let marker = if is_selected { ">" } else { " " };

            let dt = Local
                .timestamp_opt(commit.time, 0)
                .latest()
                .map(|t| t.format("%m-%d %H:%M").to_string())
                .unwrap_or_default();

            let hash_style = theme.commit_hash(is_selected);
            let msg_style = theme.commit_msg(is_selected);
            let meta_style = theme.dim_text();

            let line = Line::from(vec![
                Span::styled(format!("{} {} ", marker, commit.short_id), hash_style),
                Span::styled(
                    format!("{} ", truncate_str(&commit.summary, 50)),
                    msg_style,
                ),
                Span::styled(format!("{} ", commit.author), meta_style),
                Span::styled(dt, meta_style),
            ]);
            lines.push(line);
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                " (no commits)",
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

    fn render_commit_info(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(theme.border_style());
        let inner = block.inner(area);
        f.render_widget(block, area);

        if let Some(commit) = self.commits.get(self.selected) {
            let dt = Local
                .timestamp_opt(commit.time, 0)
                .latest()
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default();

            let label_style = theme.dim_text();
            let value_style = Style::default().fg(theme.commit_hash);

            let lines = vec![
                Line::from(vec![
                    Span::styled("Author: ", label_style),
                    Span::styled(commit.author.clone(), value_style),
                ]),
                Line::from(vec![
                    Span::styled("Date:   ", label_style),
                    Span::styled(dt, value_style),
                ]),
                Line::from(vec![
                    Span::styled("Hash:   ", label_style),
                    Span::styled(commit.id.clone(), value_style),
                ]),
                Line::from(Span::styled("", theme.normal())),
                Line::from(Span::styled(
                    commit.summary.clone(),
                    Style::default().fg(theme.commit_msg),
                )),
            ];

            f.render_widget(Paragraph::new(lines), inner);
        } else {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " (no commit selected)",
                    theme.dim_text(),
                ))),
                inner,
            );
        }
    }

    fn render_file_list(&self, f: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let border_style = if focused {
            theme.border_focused_style()
        } else {
            theme.border_style()
        };
        let block = Block::default()
            .title(" Files ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.file_list_height.set(inner.height as usize);

        let mut lines: Vec<Line> = Vec::new();

        for (i, file) in self.files.iter().enumerate() {
            let is_selected = i == self.file_selected && focused;
            let marker = if is_selected { ">" } else { " " };
            let style = if is_selected {
                theme.selected()
            } else {
                Style::default().fg(theme.file_entry).bg(theme.light_bg)
            };

            lines.push(Line::from(Span::styled(
                format!(" {} {}", marker, file),
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

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}
