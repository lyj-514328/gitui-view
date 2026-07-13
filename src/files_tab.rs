use crate::git::{GitRepo, StatusEntry, StatusType};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cmp;

pub struct FilesTab {
    pub files: Vec<StatusEntry>,
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
        let mut files = Vec::new();
        if let Ok((staged, unstaged)) = repo.get_status() {
            files.extend(staged);
            files.extend(unstaged);
        }

        if let Ok(commits) = repo.get_commits(1) {
            if let Some(commit) = commits.first() {
                if let Ok(diffs) = repo.get_commit_diff(&commit.id) {
                    for diff in &diffs {
                        let path = if !diff.new_path.is_empty() {
                            diff.new_path.clone()
                        } else {
                            diff.old_path.clone()
                        };
                        if !files.iter().any(|f| f.path == path) {
                            files.push(StatusEntry {
                                path,
                                status: diff.status.clone(),
                                staged: false,
                            });
                        }
                    }
                }
            }
        }

        self.files = files;
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
        self.files.get(self.selected).map(|f| f.path.clone())
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
            let status_char = match file.status {
                StatusType::Added => "A",
                StatusType::Modified => "M",
                StatusType::Deleted => "D",
                StatusType::Renamed => "R",
                StatusType::Copied => "C",
                StatusType::Untracked => "?",
                StatusType::TypeChange => "T",
            };

            let style = if is_selected {
                theme.selected
            } else if file.staged {
                theme.file_entry_staged
            } else {
                match file.status {
                    StatusType::Untracked => theme.file_entry_untracked,
                    StatusType::Modified => theme.file_entry_modified,
                    _ => theme.file_entry,
                }
            };

            lines.push(Line::from(Span::styled(
                format!(" {} {} {}", marker, status_char, file.path),
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
