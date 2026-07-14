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
use std::cmp;

pub struct LogTab {
    pub commits: Vec<CommitInfo>,
    pub selected: usize,
    pub scroll: usize,
    pub show_files: bool,
    pub files: Vec<String>,
    pub file_selected: usize,
    pub file_scroll: usize,
}

impl LogTab {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            selected: 0,
            scroll: 0,
            show_files: false,
            files: Vec::new(),
            file_selected: 0,
            file_scroll: 0,
        }
    }

    pub fn refresh(&mut self, repo: &GitRepo) {
        if let Ok(commits) = repo.get_commits(200) {
            self.commits = commits;
            self.selected = 0;
            self.scroll = 0;
        }
        self.show_files = false;
        self.files.clear();
        self.file_selected = 0;
        self.file_scroll = 0;
    }

    pub fn move_down(&mut self) {
        let max = self.commits.len().saturating_sub(1);
        self.selected = cmp::min(self.selected + 1, max);
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn current_commit_id(&self) -> Option<String> {
        self.commits.get(self.selected).map(|c| c.id.clone())
    }

    pub fn toggle_files(&mut self, repo: &GitRepo) {
        if self.show_files {
            self.show_files = false;
            self.files.clear();
            self.file_selected = 0;
            self.file_scroll = 0;
        } else {
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
                    self.show_files = true;
                    return;
                }
            }
        }
    }

    pub fn close_files(&mut self) {
        self.show_files = false;
        self.files.clear();
        self.file_selected = 0;
        self.file_scroll = 0;
    }

    pub fn file_move_down(&mut self) {
        let max = self.files.len().saturating_sub(1);
        self.file_selected = cmp::min(self.file_selected + 1, max);
    }

    pub fn file_move_up(&mut self) {
        self.file_selected = self.file_selected.saturating_sub(1);
    }

    pub fn current_file_path(&self) -> Option<String> {
        self.files.get(self.file_selected).cloned()
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        if self.show_files {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(area);

            self.render_commit_list(f, split[0], theme);
            self.render_file_list(f, split[1], theme);
        } else {
            self.render_commit_list(f, area, theme);
        }
    }

    fn render_commit_list(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Log ")
            .borders(Borders::ALL)
            .border_style(theme.border_style());
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        for (i, commit) in self.commits.iter().enumerate() {
            let is_selected = i == self.selected;
            let marker = if is_selected { ">" } else { " " };

            let dt = Local
                .timestamp_opt(commit.time, 0)
                .latest()
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
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

    fn render_file_list(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Files ")
            .borders(Borders::ALL)
            .border_style(theme.border_style());
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        for (i, file) in self.files.iter().enumerate() {
            let is_selected = i == self.file_selected;
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
