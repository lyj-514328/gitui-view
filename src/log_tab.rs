use crate::diff::DiffView;
use crate::git::{CommitId, CommitInfo, GitRepo, StatusType};
use crate::theme::Theme;
use chrono::{Local, TimeZone};
use indexmap::IndexSet;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cell::Cell;
use std::cmp;
use std::collections::HashMap;

const SLICE_SIZE: usize = 1200;
const SLICE_OFFSET_RELOAD_THRESHOLD: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogDepth {
    Commits,
    Details,
    FilesDiff,
    Diff,
}

pub struct LogTab {
    /// All commit IDs (lightweight, stored once).
    pub commit_ids: IndexSet<CommitId>,
    /// Sliding window of full commit details (≈ SLICE_SIZE entries).
    pub display_commits: Vec<CommitInfo>,
    /// Index into commit_ids where display_commits starts.
    pub display_offset: usize,
    /// Current selection index into commit_ids.
    pub selected: usize,
    pub files: Vec<(String, StatusType)>,
    pub file_selected: usize,
    pub file_scroll: usize,
    pub depth: LogDepth,
    pub tags_map: HashMap<String, Vec<String>>,
    pub branches_map: HashMap<String, Vec<(String, bool)>>,
    pub total_commits: usize,
    commit_list_height: Cell<usize>,
    file_list_height: Cell<usize>,
    scroll: Cell<usize>,
}

impl LogTab {
    pub fn new() -> Self {
        Self {
            commit_ids: IndexSet::new(),
            display_commits: Vec::new(),
            display_offset: 0,
            selected: 0,
            files: Vec::new(),
            file_selected: 0,
            file_scroll: 0,
            depth: LogDepth::Commits,
            tags_map: HashMap::new(),
            branches_map: HashMap::new(),
            total_commits: 0,
            commit_list_height: Cell::new(0),
            file_list_height: Cell::new(0),
            scroll: Cell::new(0),
        }
    }

    pub fn refresh(&mut self, repo: &GitRepo) {
        self.total_commits = repo.count_all_commits().unwrap_or(0);
        if let Ok(ids) = repo.get_all_commit_ids() {
            self.commit_ids = ids.into_iter().collect();
            self.selected = 0;
            self.fetch_commits(repo);
        } else {
            self.commit_ids.clear();
            self.display_commits.clear();
            self.display_offset = 0;
        }
        self.tags_map = repo.get_commit_tags().unwrap_or_default();
        self.branches_map = repo.get_commit_branches().unwrap_or_default();
        self.files.clear();
        self.file_selected = 0;
        self.file_scroll = 0;
        self.scroll.set(0);
        self.depth = LogDepth::Commits;
    }

    /// Reload the sliding window of CommitInfo centered on selection.
    fn fetch_commits(&mut self, repo: &GitRepo) {
        let total = self.commit_ids.len();
        if total == 0 {
            self.display_commits.clear();
            self.display_offset = 0;
            return;
        }
        let half = SLICE_SIZE / 2;
        let want_min = if self.selected > half {
            self.selected - half
        } else {
            0
        };
        let want_min = want_min.min(total.saturating_sub(SLICE_SIZE));
        let want_count = SLICE_SIZE.min(total - want_min);

        let ids: Vec<CommitId> = self
            .commit_ids
            .iter()
            .skip(want_min)
            .take(want_count)
            .copied()
            .collect();

        if let Ok(infos) = repo.get_commits_info(&ids) {
            self.display_commits = infos;
            self.display_offset = want_min;
        }
    }

    /// Check whether the sliding window needs to be recentered.
    fn needs_data(&self) -> bool {
        if self.commit_ids.is_empty() {
            return false;
        }
        if self.display_commits.is_empty() {
            return true;
        }
        let end = self.display_offset + self.display_commits.len();
        let thr = SLICE_OFFSET_RELOAD_THRESHOLD;
        self.selected < self.display_offset + thr
            || self.selected >= end.saturating_sub(thr)
    }

    fn ensure_data(&mut self, repo: &GitRepo) {
        if self.needs_data() {
            self.fetch_commits(repo);
        }
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
            LogDepth::Commits => {
                if !self.commit_ids.is_empty() {
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

    pub fn move_down(&mut self, repo: &GitRepo) {
        match self.depth {
            LogDepth::Commits | LogDepth::Details => {
                let max = self.commit_ids.len().saturating_sub(1);
                self.selected = cmp::min(self.selected + 1, max);
                self.ensure_data(repo);
            }
            LogDepth::FilesDiff => {
                let max = self.files.len().saturating_sub(1);
                self.file_selected = cmp::min(self.file_selected + 1, max);
                self.ensure_file_visible();
            }
            LogDepth::Diff => {}
        }
    }

    pub fn move_up(&mut self, repo: &GitRepo) {
        match self.depth {
            LogDepth::Commits | LogDepth::Details => {
                self.selected = self.selected.saturating_sub(1);
                self.ensure_data(repo);
            }
            LogDepth::FilesDiff => {
                self.file_selected = self.file_selected.saturating_sub(1);
                self.ensure_file_visible();
            }
            LogDepth::Diff => {}
        }
    }

    fn calc_scroll_top(&self) -> usize {
        let height = self.commit_list_height.get();
        if height == 0 || self.display_commits.is_empty() {
            self.scroll.set(0);
            return 0;
        }
        let max_scroll = self
            .display_commits
            .len()
            .saturating_sub(height);
        let local_sel = self
            .selected
            .saturating_sub(self.display_offset)
            .min(self.display_commits.len().saturating_sub(1));

        let current = self.scroll.get();
        let new_scroll = if local_sel < current {
            local_sel
        } else if local_sel >= current + height {
            local_sel.saturating_sub(height).saturating_add(1)
        } else {
            current
        };
        let new_scroll = new_scroll.min(max_scroll);
        self.scroll.set(new_scroll);
        new_scroll
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

    pub fn page_down(&mut self, repo: &GitRepo) {
        match self.depth {
            LogDepth::Commits | LogDepth::Details => {
                let page = self.commit_list_height.get().max(1);
                let max = self.commit_ids.len().saturating_sub(1);
                self.selected = cmp::min(self.selected + page, max);
                self.ensure_data(repo);
            }
            LogDepth::FilesDiff => {
                let page = self.file_list_height.get().max(1);
                let max = self.files.len().saturating_sub(1);
                self.file_selected = cmp::min(self.file_selected + page, max);
                self.ensure_file_visible();
            }
            LogDepth::Diff => {}
        }
    }

    pub fn page_up(&mut self, repo: &GitRepo) {
        match self.depth {
            LogDepth::Commits | LogDepth::Details => {
                let page = self.commit_list_height.get().max(1);
                self.selected = self.selected.saturating_sub(page);
                self.ensure_data(repo);
            }
            LogDepth::FilesDiff => {
                let page = self.file_list_height.get().max(1);
                self.file_selected = self.file_selected.saturating_sub(page);
                self.ensure_file_visible();
            }
            LogDepth::Diff => {}
        }
    }

    pub fn go_home(&mut self, repo: &GitRepo) {
        match self.depth {
            LogDepth::Commits | LogDepth::Details => {
                self.selected = 0;
                self.scroll.set(0);
                self.ensure_data(repo);
            }
            LogDepth::FilesDiff => {
                self.file_selected = 0;
                self.ensure_file_visible();
            }
            LogDepth::Diff => {}
        }
    }

    pub fn go_end(&mut self, repo: &GitRepo) {
        match self.depth {
            LogDepth::Commits | LogDepth::Details => {
                self.selected = self.commit_ids.len().saturating_sub(1);
                self.ensure_data(repo);
                let height = self.commit_list_height.get().max(1);
                let display_len = self.display_commits.len();
                self.scroll.set(display_len.saturating_sub(height));
            }
            LogDepth::FilesDiff => {
                self.file_selected = self.files.len().saturating_sub(1);
                self.ensure_file_visible();
            }
            LogDepth::Diff => {}
        }
    }

    pub fn current_commit_id(&self) -> Option<String> {
        self.commit_ids
            .get_index(self.selected)
            .map(|cid| cid.0.to_string())
    }

    pub fn current_file_path(&self) -> Option<String> {
        self.files.get(self.file_selected).map(|(path, _)| path.clone())
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
        let total = self.total_commits;
        let remaining = total.saturating_sub(self.selected);
        let title = if total > 0 {
            format!(" Log {}/{} ", remaining, total)
        } else {
            " Log ".to_string()
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.commit_list_height.set(inner.height as usize);

        let mut lines: Vec<Line> = Vec::new();

        for (i, commit) in self.display_commits.iter().enumerate() {
            let global_idx = self.display_offset + i;
            let is_selected = global_idx == self.selected;

            let dt = Local
                .timestamp_opt(commit.time, 0)
                .latest()
                .map(|t| t.format("%Y-%m-%d").to_string())
                .unwrap_or_default();

            let hash_style = theme.commit_hash(is_selected);
            let msg_style = theme.commit_msg(is_selected);
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
            let tag_style = if is_selected {
                theme.selected()
            } else {
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD)
            };
            let branch_style = if is_selected {
                theme.selected()
            } else {
                Style::default().fg(Color::LightYellow)
            };

            let mut spans: Vec<Span> = vec![
                Span::styled(format!(" {} ", commit.short_id), hash_style),
                Span::styled(format!(" {} ", dt), date_style),
                Span::styled(format!(" {} ", commit.author), author_style),
            ];

            // tags
            if let Some(tags) = self.tags_map.get(&commit.id) {
                for tag in tags {
                    spans.push(Span::styled(format!("<{}>", tag), tag_style));
                }
            }

            // branches
            if let Some(branches) = self.branches_map.get(&commit.id) {
                for (name, is_local) in branches {
                    if *is_local {
                        spans.push(Span::styled(format!("{{{}}}", name), branch_style));
                    } else {
                        spans.push(Span::styled(format!("[{}]", name), branch_style));
                    }
                }
            }

            // message (extra spacing before message)
            spans.push(Span::styled(format!("  {}", commit.summary), msg_style));

            lines.push(Line::from(spans));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                " (no commits)",
                theme.dim_text(),
            )));
        }

        let scroll_top = self.calc_scroll_top();
        let visible: Vec<Line> = lines
            .into_iter()
            .skip(scroll_top)
            .take(inner.height as usize)
            .collect();

        f.render_widget(Paragraph::new(visible), inner);
    }

    fn render_commit_info(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Info ")
            .borders(Borders::ALL)
            .border_style(theme.border_style());
        let inner = block.inner(area);
        f.render_widget(block, area);

        let display_idx = self.selected.checked_sub(self.display_offset);
        let commit = display_idx.and_then(|i| self.display_commits.get(i));

        if let Some(commit) = commit {
            let label_style = theme.dim_text();
            let value_style = Style::default();

            let author_dt = Local
                .timestamp_opt(commit.time, 0)
                .latest()
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default();

            let committer_dt = Local
                .timestamp_opt(commit.committer_time, 0)
                .latest()
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default();

            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Author: ", label_style),
                    Span::styled(
                        format!("{} <{}>", commit.author, commit.author_email),
                        value_style,
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Date:   ", label_style),
                    Span::styled(author_dt, value_style),
                ]),
                Line::from(vec![
                    Span::styled("Committer: ", label_style),
                    Span::styled(
                        format!("{} <{}>", commit.committer, commit.committer_email),
                        value_style,
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Date:   ", label_style),
                    Span::styled(committer_dt, value_style),
                ]),
            ];

            lines.push(Line::from(vec![
                Span::styled("Sha:    ", label_style),
                Span::styled(commit.id.clone(), value_style),
            ]));

            // blank line + summary
            lines.push(Line::from(Span::styled("", theme.normal())));
            lines.push(Line::from(Span::styled(
                commit.summary.clone(),
                Style::default().fg(theme.commit_msg),
            )));

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
