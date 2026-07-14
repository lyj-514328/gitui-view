use crate::diff_engine::DiffEngine;
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
use std::path::Path;
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

        let extension = Path::new(&file_diff.new_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut line_idx = 0;

        for hunk in &file_diff.hunks {
            let header_text = hunk.header.trim();
            lines.push(Line::from(Span::styled(header_text.to_string(), theme.diff_header())));

            let mut minus_buffer: Vec<&DiffLine> = Vec::new();
            let mut plus_buffer: Vec<&DiffLine> = Vec::new();

            for diff_line in &hunk.lines {
                match diff_line.line_type {
                    DiffLineType::Context | DiffLineType::Header => {
                        flush_buffer_inline(&mut lines, &mut line_idx, &mut minus_buffer, &mut plus_buffer, extension, theme, self.selected_line, area.width);

                        let is_selected = line_idx == self.selected_line;
                        let style = if is_selected {
                            theme.selected()
                        } else {
                            theme.diff_context(is_selected)
                        };
                        let content_spans = DiffEngine::highlight_line(
                            &diff_line.content,
                            diff_line.line_type.clone(),
                            extension,
                            theme,
                        );
                        let lineno = diff_line.old_lineno.unwrap_or(diff_line.new_lineno.unwrap_or(0));
                        let prefix = format!(" │{:^4}│ ", lineno);
                        wrap_and_push(&mut lines, &prefix, &content_spans, area.width as usize, style);
                        line_idx += 1;
                    }
                    DiffLineType::Delete => minus_buffer.push(diff_line),
                    DiffLineType::Add => plus_buffer.push(diff_line),
                }
            }

            flush_buffer_inline(&mut lines, &mut line_idx, &mut minus_buffer, &mut plus_buffer, extension, theme, self.selected_line, area.width);
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

        let extension = Path::new(&file_diff.new_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut left_lines: Vec<Line<'static>> = Vec::new();
        let mut right_lines: Vec<Line<'static>> = Vec::new();

        for hunk in &file_diff.hunks {
            let header_text = hunk.header.trim().to_string();
            left_lines.push(Line::from(Span::styled(header_text.clone(), theme.diff_header())));
            let underline = "─".repeat(left_area.width.saturating_sub(1) as usize);
            left_lines.push(Line::from(Span::styled(underline, theme.diff_header())));
            right_lines.push(Line::from(Span::styled(String::new(), theme.dim_text())));
            right_lines.push(Line::from(Span::styled(String::new(), theme.dim_text())));

            let (paired_left, paired_right) = self.pair_lines(hunk, extension, theme, left_area.width, right_area.width);

            let max_lines = cmp::max(paired_left.len(), paired_right.len());
            for i in 0..max_lines {
                if i < paired_left.len() {
                    left_lines.push(paired_left[i].clone());
                } else {
                    left_lines.push(Line::from(Span::styled(
                        String::new(),
                        theme.dim_text(),
                    )));
                }

                if i < paired_right.len() {
                    right_lines.push(paired_right[i].clone());
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

    fn pair_lines(&self, hunk: &Hunk, extension: &str, theme: &Theme, left_width: u16, right_width: u16) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
        let mut left: Vec<Line<'static>> = Vec::new();
        let mut right: Vec<Line<'static>> = Vec::new();
        let mut delete_lines: Vec<&DiffLine> = Vec::new();
        let mut add_lines: Vec<&DiffLine> = Vec::new();

        for line in &hunk.lines {
            match line.line_type {
                DiffLineType::Context | DiffLineType::Header => {
                    flush_buffer_sbs(&mut left, &mut right, &mut delete_lines, &mut add_lines, extension, theme, left_width, right_width);
                    let spans = DiffEngine::highlight_line(
                        &line.content,
                        line.line_type.clone(),
                        extension,
                        theme,
                    );
                    let left_lineno = line.old_lineno;
                    let right_lineno = line.new_lineno;
                    let left_prefix = format!("│{:^4}│ ", left_lineno.unwrap_or(0));
                    let right_prefix = format!("│{:^4}│ ", right_lineno.unwrap_or(0));
                    wrap_and_push_pair(&mut left, &mut right, &left_prefix, &right_prefix, &spans, &spans, left_width as usize, right_width as usize);
                }
                DiffLineType::Delete => delete_lines.push(line),
                DiffLineType::Add => add_lines.push(line),
            }
        }

        flush_buffer_sbs(&mut left, &mut right, &mut delete_lines, &mut add_lines, extension, theme, left_width, right_width);

        (left, right)
    }
}

fn flush_buffer_inline(
    lines: &mut Vec<Line<'static>>,
    line_idx: &mut usize,
    minus_buffer: &mut Vec<&DiffLine>,
    plus_buffer: &mut Vec<&DiffLine>,
    extension: &str,
    theme: &Theme,
    selected_line: usize,
    area_width: u16,
) {
    let pair_count = cmp::max(minus_buffer.len(), plus_buffer.len());
    for i in 0..pair_count {
        let is_minus_selected = *line_idx == selected_line;
        let is_plus_selected = *line_idx == selected_line;

        if i < minus_buffer.len() && i < plus_buffer.len() {
            let minus_line = &minus_buffer[i];
            let plus_line = &plus_buffer[i];
            let (minus_spans, plus_spans) = DiffEngine::highlight_line_pair(
                &minus_line.content,
                &plus_line.content,
                extension,
                theme,
            );

            let prefix = format!("-│{:^4}│ ", minus_line.old_lineno.unwrap_or(0));
            wrap_and_push(lines, &prefix, &minus_spans, area_width as usize, theme.diff_delete(is_minus_selected));
            *line_idx += 1;

            let prefix = format!("+│{:^4}│ ", plus_line.new_lineno.unwrap_or(0));
            wrap_and_push(lines, &prefix, &plus_spans, area_width as usize, theme.diff_add(is_plus_selected));
            *line_idx += 1;
        } else if i < minus_buffer.len() {
            let line = minus_buffer[i];
            let is_selected = *line_idx == selected_line;
            let content_spans = DiffEngine::highlight_line(
                &line.content,
                DiffLineType::Delete,
                extension,
                theme,
            );
            let prefix = format!("-│{:^4}│ ", line.old_lineno.unwrap_or(0));
            wrap_and_push(lines, &prefix, &content_spans, area_width as usize, theme.diff_delete(is_selected));
            *line_idx += 1;
        } else {
            let line = plus_buffer[i];
            let is_selected = *line_idx == selected_line;
            let content_spans = DiffEngine::highlight_line(
                &line.content,
                DiffLineType::Add,
                extension,
                theme,
            );
            let prefix = format!("+│{:^4}│ ", line.new_lineno.unwrap_or(0));
            wrap_and_push(lines, &prefix, &content_spans, area_width as usize, theme.diff_add(is_selected));
            *line_idx += 1;
        }
    }
    minus_buffer.clear();
    plus_buffer.clear();
}

fn flush_buffer_sbs(
    left: &mut Vec<Line<'static>>,
    right: &mut Vec<Line<'static>>,
    dels: &mut Vec<&DiffLine>,
    adds: &mut Vec<&DiffLine>,
    extension: &str,
    theme: &Theme,
    left_width: u16,
    right_width: u16,
) {
    let empty_prefix = "│    │ ".to_string();
    let pair_count = cmp::max(dels.len(), adds.len());
    for i in 0..pair_count {
        if i < dels.len() && i < adds.len() {
            let (minus_spans, plus_spans) = DiffEngine::highlight_line_pair(
                &dels[i].content,
                &adds[i].content,
                extension,
                theme,
            );
            let left_prefix = format!("│{:^4}│ ", dels[i].old_lineno.unwrap_or(0));
            let right_prefix = format!("│{:^4}│ ", adds[i].new_lineno.unwrap_or(0));
            wrap_and_push_pair(left, right, &left_prefix, &right_prefix, &minus_spans, &plus_spans, left_width as usize, right_width as usize);
        } else if i < dels.len() {
            let line = dels[i];
            let spans = DiffEngine::highlight_line(
                &line.content,
                DiffLineType::Delete,
                extension,
                theme,
            );
            let left_prefix = format!("│{:^4}│ ", line.old_lineno.unwrap_or(0));
            wrap_and_push_pair(left, right, &left_prefix, &empty_prefix, &spans, &[], left_width as usize, right_width as usize);
        } else {
            let line = adds[i];
            let spans = DiffEngine::highlight_line(
                &line.content,
                DiffLineType::Add,
                extension,
                theme,
            );
            let right_prefix = format!("│{:^4}│ ", line.new_lineno.unwrap_or(0));
            wrap_and_push_pair(left, right, &empty_prefix, &right_prefix, &[], &spans, left_width as usize, right_width as usize);
        }
    }
    dels.clear();
    adds.clear();
}

fn wrap_and_push(
    lines: &mut Vec<Line<'static>>,
    prefix: &str,
    content_spans: &[Span<'static>],
    max_width: usize,
    prefix_style: Style,
) {
    let prefix_width = UnicodeWidthStr::width(prefix);
    let content_width = max_width.saturating_sub(prefix_width);
    if content_width == 0 {
        lines.push(Line::from(Span::styled(prefix.to_string(), prefix_style)));
        return;
    }

    let mut line_buf = Vec::new();
    line_buf.push(Span::styled(prefix.to_string(), prefix_style));

    let mut current_width = prefix_width;

    for span in content_spans {
        let s = span.content.as_ref();
        let span_width = UnicodeWidthStr::width(s);

        if current_width + span_width <= max_width {
            line_buf.push(Span::styled(s.to_string(), span.style));
            current_width += span_width;
        } else {
            let available = max_width.saturating_sub(current_width);
            if available > 0 {
                let mut truncated = String::new();
                let mut w = 0;
                for c in s.chars() {
                    let cw = UnicodeWidthStr::width(c.to_string().as_str());
                    if w + cw > available {
                        truncated.push('↴');
                        break;
                    }
                    truncated.push(c);
                    w += cw;
                }
                line_buf.push(Span::styled(truncated, span.style));
            }
            lines.push(Line::from(line_buf));

            line_buf = Vec::new();
            line_buf.push(Span::styled("│    │ ".to_string(), prefix_style));
            let mut cont_width = UnicodeWidthStr::width("│    │ ");

            let rest_start = available;
            if rest_start < span_width {
                let rest_text = s.chars().skip(rest_start).collect::<String>();
                let rest_width = UnicodeWidthStr::width(rest_text.as_str());
                line_buf.push(Span::styled(rest_text, span.style));
                cont_width += rest_width;
            }
            current_width = cont_width;
        }
    }

    if !line_buf.is_empty() {
        lines.push(Line::from(line_buf));
    }
}

fn wrap_and_push_pair(
    left: &mut Vec<Line<'static>>,
    right: &mut Vec<Line<'static>>,
    left_prefix: &str,
    right_prefix: &str,
    left_spans: &[Span<'static>],
    right_spans: &[Span<'static>],
    left_max_width: usize,
    right_max_width: usize,
) {
    let left_prefix_width = UnicodeWidthStr::width(left_prefix);
    let right_prefix_width = UnicodeWidthStr::width(right_prefix);
    let left_content_width = left_max_width.saturating_sub(left_prefix_width);
    let right_content_width = right_max_width.saturating_sub(right_prefix_width);

    if left_content_width == 0 && right_content_width == 0 {
        left.push(Line::from(Span::styled(left_prefix.to_string(), Style::default())));
        right.push(Line::from(Span::styled(right_prefix.to_string(), Style::default())));
        return;
    }

    let left_chunks = chunk_spans(left_spans, left_content_width);
    let right_chunks = chunk_spans(right_spans, right_content_width);

    let max_chunks = cmp::max(left_chunks.len(), right_chunks.len());

    let empty_cont_prefix = "│    │ ";

    for i in 0..max_chunks {
        let lprefix = if i == 0 { left_prefix } else { empty_cont_prefix };
        let rprefix = if i == 0 { right_prefix } else { empty_cont_prefix };

        let mut lspans = vec![Span::styled(lprefix.to_string(), Style::default())];
        let mut rspans = vec![Span::styled(rprefix.to_string(), Style::default())];

        if i < left_chunks.len() {
            lspans.extend(left_chunks[i].clone());
        }
        if i < right_chunks.len() {
            rspans.extend(right_chunks[i].clone());
        }

        left.push(Line::from(lspans));
        right.push(Line::from(rspans));
    }
}

fn chunk_spans(spans: &[Span<'static>], content_width: usize) -> Vec<Vec<Span<'static>>> {
    if content_width == 0 || spans.is_empty() {
        return vec![Vec::new()];
    }

    let mut chunks = Vec::new();
    let mut current_chunk = Vec::new();
    let mut current_width = 0;

    for span in spans {
        let s = span.content.as_ref();
        let span_width = UnicodeWidthStr::width(s);

        if span_width == 0 {
            current_chunk.push(Span::styled(s.to_string(), span.style));
            continue;
        }

        let mut remaining = s;
        let mut remaining_width = span_width;

        while remaining_width > 0 {
            let available = content_width.saturating_sub(current_width);
            if available == 0 {
                chunks.push(current_chunk);
                current_chunk = Vec::new();
                current_width = 0;
                continue;
            }

            if remaining_width <= available {
                current_chunk.push(Span::styled(remaining.to_string(), span.style));
                current_width += remaining_width;
                break;
            } else {
                let mut truncated = String::new();
                let mut w = 0;
                let mut chars_consumed: usize = 0;
                for c in remaining.chars() {
                    let cw = UnicodeWidthStr::width(c.to_string().as_str());
                    if w + cw > available {
                        truncated.push('↴');
                        break;
                    }
                    truncated.push(c);
                    w += cw;
                    chars_consumed += 1;
                }
                current_chunk.push(Span::styled(truncated, span.style));
                chunks.push(current_chunk);
                current_chunk = Vec::new();
                current_width = 0;

                if chars_consumed > 0 {
                    let mut iter = remaining.chars();
                    for _ in 0..chars_consumed.saturating_sub(1) {
                        iter.next();
                    }
                    remaining = iter.as_str();
                }
                remaining_width = UnicodeWidthStr::width(remaining);
            }
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    if chunks.is_empty() {
        chunks.push(Vec::new());
    }

    chunks
}