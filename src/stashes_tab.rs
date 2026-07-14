use crate::git::{GitRepo, StashInfo};
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

pub struct StashesTab {
    pub stashes: Vec<StashInfo>,
    pub selected: usize,
    pub scroll: usize,
    pub show_files: bool,
    pub files: Vec<String>,
    pub file_selected: usize,
    pub file_scroll: usize,
}

impl StashesTab {
    pub fn new() -> Self {
        Self {
            stashes: Vec::new(),
            selected: 0,
            scroll: 0,
            show_files: false,
            files: Vec::new(),
            file_selected: 0,
            file_scroll: 0,
        }
    }

    pub fn refresh(&mut self, repo: &mut GitRepo) {
        if let Ok(stashes) = repo.get_stashes() {
            self.stashes = stashes;
            self.selected = 0;
            self.scroll = 0;
        }
        self.show_files = false;
        self.files.clear();
        self.file_selected = 0;
        self.file_scroll = 0;
    }

    pub fn move_down(&mut self) {
        let max = self.stashes.len().saturating_sub(1);
        self.selected = cmp::min(self.selected + 1, max);
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn current_stash_index(&self) -> Option<usize> {
        self.stashes.get(self.selected).map(|s| s.index)
    }

    pub fn toggle_files(&mut self, repo: &mut GitRepo) {
        if self.show_files {
            self.show_files = false;
            self.files.clear();
            self.file_selected = 0;
            self.file_scroll = 0;
        } else {
            if let Some(index) = self.current_stash_index() {
                if let Ok(diffs) = repo.get_stash_diff(index) {
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

            self.render_stash_list(f, split[0], theme);
            self.render_file_list(f, split[1], theme);
        } else {
            self.render_stash_list(f, area, theme);
        }
    }

    fn render_stash_list(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Stashes ")
            .borders(Borders::ALL)
            .border_style(theme.border_style());
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        for (i, stash) in self.stashes.iter().enumerate() {
            let is_selected = i == self.selected;
            let marker = if is_selected { ">" } else { " " };

            let dt = Local
                .timestamp_opt(stash.time, 0)
                .latest()
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();

            let msg_style = theme.stash_msg(is_selected);
            let meta_style = theme.dim_text();

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} stash@{{{}}} ", marker, stash.index),
                    msg_style,
                ),
                Span::styled(
                    format!("{} ", truncate_str(&stash.message, 60)),
                    msg_style,
                ),
                Span::styled(dt, meta_style),
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
