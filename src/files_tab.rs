use crate::git::GitRepo;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cmp;

pub struct FilesTab {
    pub files: Vec<String>,
    pub selected: usize,
    pub scroll: usize,
}

impl FilesTab {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            selected: 0,
            scroll: 0,
        }
    }

    pub fn refresh(&mut self, repo: &GitRepo) {
        if let Ok(files) = repo.get_tree_files() {
            self.files = files;
        } else {
            self.files.clear();
        }
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn move_down(&mut self) {
        let max = self.files.len().saturating_sub(1);
        self.selected = cmp::min(self.selected + 1, max);
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn current_file(&self) -> Option<String> {
        self.files.get(self.selected).cloned()
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Files ")
            .borders(Borders::ALL)
            .border_style(theme.border);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        for (i, file) in self.files.iter().enumerate() {
            let is_selected = i == self.selected;
            let marker = if is_selected { ">" } else { " " };
            let style = if is_selected {
                theme.selected
            } else {
                theme.file_entry
            };

            lines.push(Line::from(Span::styled(
                format!(" {} {}", marker, file),
                style,
            )));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                " (no files)",
                theme.dim_text,
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
