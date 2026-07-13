use crate::git::{GitRepo, StashInfo};
use crate::theme::Theme;
use chrono::{Local, TimeZone};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cmp;

pub struct StashesTab {
    pub stashes: Vec<StashInfo>,
    pub selected: usize,
    pub scroll: usize,
}

impl StashesTab {
    pub fn new() -> Self {
        Self {
            stashes: Vec::new(),
            selected: 0,
            scroll: 0,
        }
    }

    pub fn refresh(&mut self, repo: &mut GitRepo) {
        if let Ok(stashes) = repo.get_stashes() {
            self.stashes = stashes;
            self.selected = 0;
            self.scroll = 0;
        }
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

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Stashes ")
            .borders(Borders::ALL)
            .border_style(theme.border);
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

            let msg_style = if is_selected {
                theme.selected
            } else {
                theme.stash_msg
            };
            let meta_style = if is_selected {
                theme.selected
            } else {
                theme.dim_text
            };

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
