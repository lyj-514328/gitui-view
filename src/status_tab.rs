use crate::git::{GitRepo, StatusEntry, StatusType};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cmp;

pub struct StatusTab {
    pub staged: Vec<StatusEntry>,
    pub unstaged: Vec<StatusEntry>,
    pub selected: usize,
    pub scroll: usize,
    pub focus_staged: bool,
}

impl StatusTab {
    pub fn new() -> Self {
        Self {
            staged: Vec::new(),
            unstaged: Vec::new(),
            selected: 0,
            scroll: 0,
            focus_staged: true,
        }
    }

    pub fn refresh(&mut self, repo: &GitRepo) {
        if let Ok((staged, unstaged)) = repo.get_status() {
            self.staged = staged;
            self.unstaged = unstaged;
            self.selected = 0;
            self.scroll = 0;
        }
    }

    pub fn total_items(&self) -> usize {
        self.staged.len() + self.unstaged.len()
    }

    pub fn current_file(&self) -> Option<String> {
        let all = self.all_items();
        all.get(self.selected).map(|e| e.path.clone())
    }

    pub fn current_staged(&self) -> bool {
        if self.selected < self.staged.len() {
            true
        } else {
            false
        }
    }

    fn all_items(&self) -> Vec<&StatusEntry> {
        let mut items: Vec<&StatusEntry> = self.staged.iter().collect();
        items.extend(self.unstaged.iter());
        items
    }

    pub fn move_down(&mut self) {
        let max = self.total_items().saturating_sub(1);
        self.selected = cmp::min(self.selected + 1, max);
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Status ")
            .borders(Borders::ALL)
            .border_style(theme.border_style());
        let inner = block.inner(area);
        f.render_widget(block, area);

        let width = inner.width as usize;
        let mut lines: Vec<Line> = Vec::new();

        if !self.staged.is_empty() {
            lines.push(Line::from(Span::styled(
                " Staged:",
                theme.title(),
            )));
            for (i, entry) in self.staged.iter().enumerate() {
                let is_selected = i == self.selected;
                let style = status_style(entry, is_selected, theme);
                let marker = if is_selected { ">" } else { " " };
                let status_char = status_char(&entry.status);
                let text = if is_selected {
                    format!(" {} {} {:w$}", marker, status_char, entry.path, w = width)
                } else {
                    format!(" {} {} {}", marker, status_char, entry.path)
                };
                lines.push(Line::from(Span::styled(text, style)));
            }
        }

        if !self.unstaged.is_empty() {
            lines.push(Line::from(Span::styled(
                " Unstaged:",
                theme.title(),
            )));
            for (i, entry) in self.unstaged.iter().enumerate() {
                let idx = self.staged.len() + i;
                let is_selected = idx == self.selected;
                let style = status_style(entry, is_selected, theme);
                let marker = if is_selected { ">" } else { " " };
                let status_char = status_char(&entry.status);
                let text = if is_selected {
                    format!(" {} {} {:w$}", marker, status_char, entry.path, w = width)
                } else {
                    format!(" {} {} {}", marker, status_char, entry.path)
                };
                lines.push(Line::from(Span::styled(text, style)));
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                " (clean)",
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
}

fn status_char(st: &StatusType) -> &'static str {
    match st {
        StatusType::Added => "A",
        StatusType::Modified => "M",
        StatusType::Deleted => "D",
        StatusType::Renamed => "R",
        StatusType::Copied => "C",
        StatusType::Untracked => "?",
        StatusType::TypeChange => "T",
    }
}

fn status_style(entry: &StatusEntry, selected: bool, theme: &Theme) -> Style {
    let base = if entry.staged {
        Style::default().fg(theme.file_entry_staged)
    } else {
        let fg = match entry.status {
            StatusType::Untracked => theme.file_entry_untracked,
            StatusType::Modified => theme.file_entry_modified,
            _ => theme.file_entry,
        };
        Style::default().fg(fg)
    };
    if selected {
        theme.selected_on(base)
    } else {
        base
    }
}