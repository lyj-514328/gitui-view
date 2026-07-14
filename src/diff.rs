use crate::git::{DiffLine, DiffLineType, FileDiff, Hunk};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cmp;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffViewMode {
    Inline,
    SideBySide,
}

pub struct DiffView {
    pub file_diff: Option<FileDiff>,
    pub scroll: usize,
    pub selected_line: usize,
    pub mode: DiffViewMode,
    pub focused: bool,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            file_diff: None,
            scroll: 0,
            selected_line: 0,
            mode: DiffViewMode::SideBySide,
            focused: false,
        }
    }

    pub fn set_diff(&mut self, diff: FileDiff) {
        self.file_diff = Some(diff);
        self.scroll = 0;
        self.selected_line = 0;
    }

    pub fn clear(&mut self) {
        self.file_diff = None;
        self.scroll = 0;
        self.selected_line = 0;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max = self.total_lines().saturating_sub(1);
        self.scroll = cmp::min(self.scroll + amount, max);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn total_lines(&self) -> usize {
        self.file_diff
            .as_ref()
            .map(|d| d.hunks.iter().map(|h| h.lines.len()).sum())
            .unwrap_or(0)
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let Some(file_diff) = &self.file_diff else {
            let block = Block::default()
                .title(" No file selected ")
                .borders(Borders::ALL)
                .border_style(theme.border_style());
            f.render_widget(block, area);
            return;
        };

        if area.width < 4 || area.height < 2 {
            return;
        }

        let block = Block::default()
            .title(format!(
                " {} {} ",
                if file_diff.status == crate::git::StatusType::Added {
                    "A"
                } else if file_diff.status == crate::git::StatusType::Deleted {
                    "D"
                } else {
                    "M"
                },
                file_diff.new_path
            ))
            .borders(Borders::ALL)
            .border_style(if self.focused {
                theme.border_focused_style()
            } else {
                theme.border_style()
            });

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        match self.mode {
            DiffViewMode::Inline => self.render_inline(f, inner_area, theme),
            DiffViewMode::SideBySide => self.render_side_by_side(f, inner_area, theme),
        }
    }

    fn render_inline(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let Some(file_diff) = &self.file_diff else { return };

        let mut lines: Vec<Line> = Vec::new();
        let mut line_idx = 0;

        for hunk in &file_diff.hunks {
            if !hunk.lines.is_empty() {
                let header_line = Line::from(Span::styled(
                    hunk.header.trim().to_string(),
                    theme.diff_header(),
                ));
                lines.push(header_line);
            }

            for diff_line in &hunk.lines {
                let is_selected = line_idx == self.selected_line;
                let style = self.line_style(diff_line, is_selected, theme);
                let prefix = match diff_line.line_type {
                    DiffLineType::Add => "+",
                    DiffLineType::Delete => "-",
                    DiffLineType::Header => " ",
                    DiffLineType::Context => " ",
                };

                let line_no = match diff_line.line_type {
                    DiffLineType::Add => format!("{:>4} ", diff_line.new_lineno.unwrap_or(0)),
                    DiffLineType::Delete => {
                        format!("{:>4} ", diff_line.old_lineno.unwrap_or(0))
                    }
                    _ => format!(
                        "{:>4} ",
                        diff_line.old_lineno.unwrap_or(diff_line.new_lineno.unwrap_or(0))
                    ),
                };

                let content = format!("{}{}{}", prefix, line_no, diff_line.content);
                lines.push(Line::from(Span::styled(content, style)));
                line_idx += 1;
            }
        }

        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll)
            .take(area.height as usize)
            .collect();

        let para = Paragraph::new(visible_lines);
        f.render_widget(para, area);
    }

    fn render_side_by_side(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let Some(file_diff) = &self.file_diff else { return };

        if area.width < 8 {
            return;
        }

        let half_width = area.width.saturating_sub(1) / 2;
        let left_area = Rect {
            x: area.x,
            y: area.y,
            width: half_width,
            height: area.height,
        };
        let right_area = Rect {
            x: area.x + half_width + 1,
            y: area.y,
            width: area.width.saturating_sub(half_width + 1),
            height: area.height,
        };

        let mut left_lines: Vec<Line> = Vec::new();
        let mut right_lines: Vec<Line> = Vec::new();

        for hunk in &file_diff.hunks {
            let header_text = hunk.header.trim().to_string();
            left_lines.push(Line::from(Span::styled(
                header_text.clone(),
                theme.diff_header(),
            )));
            right_lines.push(Line::from(Span::styled(header_text, theme.diff_header())));

            let (paired_left, paired_right) = self.pair_lines(hunk, theme);

            let max_lines = cmp::max(paired_left.len(), paired_right.len());
            for i in 0..max_lines {
                if i < paired_left.len() {
                    let (content, style) = &paired_left[i];
                    left_lines.push(Line::from(Span::styled(
                        truncate_str(content, half_width.saturating_sub(2) as usize),
                        *style,
                    )));
                } else {
                    left_lines.push(Line::from(Span::styled(
                        String::new(),
                        theme.dim_text(),
                    )));
                }

                if i < paired_right.len() {
                    let (content, style) = &paired_right[i];
                    right_lines.push(Line::from(Span::styled(
                        truncate_str(content, (area.width - half_width - 1).saturating_sub(2) as usize),
                        *style,
                    )));
                } else {
                    right_lines.push(Line::from(Span::styled(
                        String::new(),
                        theme.dim_text(),
                    )));
                }
            }
        }

        let visible_left: Vec<Line> = left_lines
            .into_iter()
            .skip(self.scroll)
            .take(area.height as usize)
            .collect();
        let visible_right: Vec<Line> = right_lines
            .into_iter()
            .skip(self.scroll)
            .take(area.height as usize)
            .collect();

        f.render_widget(Paragraph::new(visible_left), left_area);
        f.render_widget(Paragraph::new(visible_right), right_area);
    }

    fn pair_lines(&self, hunk: &Hunk, theme: &Theme) -> (Vec<(String, Style)>, Vec<(String, Style)>) {
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut delete_lines: Vec<&DiffLine> = Vec::new();
        let mut add_lines: Vec<&DiffLine> = Vec::new();

        for line in &hunk.lines {
            match line.line_type {
                DiffLineType::Delete => delete_lines.push(line),
                DiffLineType::Add => add_lines.push(line),
                _ => {}
            }
        }

        let pair_count = cmp::max(delete_lines.len(), add_lines.len());

        for i in 0..pair_count {
            if i < delete_lines.len() {
                let line = delete_lines[i];
                left.push((line.content.clone(), self.line_style(line, false, theme)));
            } else {
                left.push((String::new(), Style::default()));
            }

            if i < add_lines.len() {
                let line = add_lines[i];
                right.push((line.content.clone(), self.line_style(line, false, theme)));
            } else {
                right.push((String::new(), Style::default()));
            }
        }

        (left, right)
    }

    fn line_style(&self, line: &DiffLine, selected: bool, theme: &Theme) -> Style {
        match line.line_type {
            DiffLineType::Add => theme.diff_add(selected),
            DiffLineType::Delete => theme.diff_delete(selected),
            DiffLineType::Header => theme.diff_header(),
            DiffLineType::Context => theme.diff_context(selected),
        }
    }
}

fn truncate_str(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let width = UnicodeWidthStr::width(s);
    if width <= max_width {
        return s.to_string();
    }
    let mut result = String::new();
    let mut current_width = 0;
    for c in s.chars() {
        let cw = UnicodeWidthStr::width(c.to_string().as_str());
        if current_width + cw > max_width.saturating_sub(1) {
            result.push('\u{2026}');
            break;
        }
        result.push(c);
        current_width += cw;
    }
    result
}
