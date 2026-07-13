use crate::git::{CommitInfo, GitRepo};
use crate::theme::Theme;
use chrono::{Local, TimeZone};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cmp;

pub struct LogTab {
    pub commits: Vec<CommitInfo>,
    pub selected: usize,
    pub scroll: usize,
}

impl LogTab {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            selected: 0,
            scroll: 0,
        }
    }

    pub fn refresh(&mut self, repo: &GitRepo) {
        if let Ok(commits) = repo.get_commits(200) {
            self.commits = commits;
            self.selected = 0;
            self.scroll = 0;
        }
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

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Log ")
            .borders(Borders::ALL)
            .border_style(theme.border);
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

            let hash_style = if is_selected {
                theme.selected
            } else {
                theme.commit_hash
            };
            let msg_style = if is_selected {
                theme.selected
            } else {
                theme.commit_msg
            };
            let meta_style = if is_selected {
                theme.selected
            } else {
                theme.dim_text
            };

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

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
