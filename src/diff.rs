use crate::diff_engine::DiffEngine;
use crate::git::{DiffLine, DiffLineType, FileDiff, Hunk};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cell::Cell;
use std::cell::RefCell;
use std::cmp;
use std::path::Path;
use std::time::Instant;
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
    pub focused: Cell<bool>,
    pub show_full_file: bool,
    pub full_file_content: Option<String>,
    visible_height: Cell<usize>,
    total_rendered_lines: Cell<usize>,
    last_scroll_time: RefCell<Instant>,
    scroll_speed: Cell<f32>,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            file_diff: None,
            scroll: 0,
            selected_line: 0,
            mode: DiffViewMode::SideBySide,
            focused: Cell::new(false),
            show_full_file: false,
            full_file_content: None,
            visible_height: Cell::new(0),
            total_rendered_lines: Cell::new(0),
            last_scroll_time: RefCell::new(Instant::now()),
            scroll_speed: Cell::new(0.0),
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
        self.show_full_file = false;
        self.full_file_content = None;
    }

    pub fn toggle_full_file(&mut self) {
        self.show_full_file = !self.show_full_file;
    }

    pub fn set_full_file_content(&mut self, content: String) {
        self.full_file_content = Some(content);
    }

    pub fn clear_full_file_content(&mut self) {
        self.full_file_content = None;
        self.show_full_file = false;
    }

    fn update_scroll_speed(&self) {
        const REPEATED_SCROLL_THRESHOLD_MILLIS: u128 = 300;
        const SCROLL_SPEED_START: f32 = 0.1;
        const SCROLL_SPEED_MAX: f32 = 10.0;
        const SCROLL_SPEED_MULTIPLIER: f32 = 1.05;

        let now = Instant::now();
        let since_last = now.duration_since(*self.last_scroll_time.borrow());
        *self.last_scroll_time.borrow_mut() = now;

        let speed = if since_last.as_millis() < REPEATED_SCROLL_THRESHOLD_MILLIS {
            self.scroll_speed.get() * SCROLL_SPEED_MULTIPLIER
        } else {
            SCROLL_SPEED_START
        };

        self.scroll_speed.set(speed.min(SCROLL_SPEED_MAX));
    }

    pub fn scroll_down(&mut self, _amount: usize) {
        self.update_scroll_speed();
        let step = (self.scroll_speed.get() as usize).max(1);
        let max = self
            .total_rendered_lines
            .get()
            .saturating_sub(self.visible_height.get().max(1));
        self.scroll = cmp::min(self.scroll + step, max);
    }

    pub fn scroll_up(&mut self, _amount: usize) {
        self.update_scroll_speed();
        let step = (self.scroll_speed.get() as usize).max(1);
        self.scroll = self.scroll.saturating_sub(step);
    }

    pub fn page_down(&mut self) {
        let page_size = self.visible_height.get().max(1);
        let max = self
            .total_rendered_lines
            .get()
            .saturating_sub(self.visible_height.get().max(1));
        self.scroll = cmp::min(self.scroll + page_size, max);
    }

    pub fn page_up(&mut self) {
        let page_size = self.visible_height.get().max(1);
        self.scroll = self.scroll.saturating_sub(page_size);
    }

    pub fn go_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn go_to_end(&mut self) {
        self.scroll = self
            .total_rendered_lines
            .get()
            .saturating_sub(self.visible_height.get().max(1));
    }

    pub fn render(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let Some(file_diff) = &self.file_diff else {
            self.total_rendered_lines.set(0);
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
            .border_style(if self.focused.get() {
                theme.border_focused_style()
            } else {
                theme.border_style()
            });

        let inner_area = block.inner(area);
        self.visible_height.set(inner_area.height as usize);
        f.render_widget(block, area);

        if file_diff.hunks.is_empty() && (file_diff.sizes.0 > 0 || file_diff.sizes.1 > 0) {
            self.render_binary(f, inner_area, file_diff, theme);
            return;
        }

        if self.show_full_file && self.full_file_content.is_some() {
            match self.mode {
                DiffViewMode::Inline => self.render_inline_full_file(f, inner_area, theme),
                DiffViewMode::SideBySide => self.render_side_by_side_full_file(f, inner_area, theme),
            }
        } else {
            match self.mode {
                DiffViewMode::Inline => self.render_inline(f, inner_area, theme),
                DiffViewMode::SideBySide => self.render_side_by_side(f, inner_area, theme),
            }
        }
    }

    fn render_inline(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let Some(file_diff) = &self.file_diff else { return };

        let extension = Path::new(&file_diff.new_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let file_name = Path::new(&file_diff.new_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut line_idx = 0;

        for hunk in &file_diff.hunks {
            lines.push(Line::from(Span::styled(String::new(), Style::default())));
            let header_text = hunk.header.trim();
            lines.push(Line::from(Span::styled(header_text.to_string(), theme.diff_header())));

            let mut minus_buffer: Vec<&DiffLine> = Vec::new();
            let mut plus_buffer: Vec<&DiffLine> = Vec::new();

            for diff_line in &hunk.lines {
                match diff_line.line_type {
                    DiffLineType::Context | DiffLineType::Header => {
                        flush_buffer_inline(&mut lines, &mut line_idx, &mut minus_buffer, &mut plus_buffer, file_name, extension, theme, area.width);

                        let content_spans = DiffEngine::highlight_line(
                            &diff_line.content,
                            diff_line.line_type.clone(),
                            file_name,
                            extension,
                            theme,
                        );
                        let prefix_spans = build_dual_prefix_spans(
                            diff_line.old_lineno,
                            diff_line.new_lineno,
                            theme.line_number_style(),
                            theme.line_number_style(),
                            theme.line_number_column_style(),
                        );
                        wrap_and_push(&mut lines, &prefix_spans, &content_spans, area.width as usize, theme.line_number_column_style());
                        line_idx += 1;
                    }
                    DiffLineType::Delete => minus_buffer.push(diff_line),
                    DiffLineType::Add => plus_buffer.push(diff_line),
                }
            }

            flush_buffer_inline(&mut lines, &mut line_idx, &mut minus_buffer, &mut plus_buffer, file_name, extension, theme, area.width);
        }

        let total = lines.len();
        self.total_rendered_lines.set(total);

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
        let file_name = Path::new(&file_diff.new_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let mut left_lines: Vec<Line<'static>> = Vec::new();
        let mut right_lines: Vec<Line<'static>> = Vec::new();

        for hunk in &file_diff.hunks {
            left_lines.push(Line::from(Span::styled(String::new(), theme.dim_text())));
            right_lines.push(Line::from(Span::styled(String::new(), theme.dim_text())));
            let header_text = hunk.header.trim().to_string();
            left_lines.push(Line::from(Span::styled(header_text.clone(), theme.diff_header())));
            let underline = "─".repeat(left_area.width.saturating_sub(1) as usize);
            left_lines.push(Line::from(Span::styled(underline, theme.diff_header())));
            right_lines.push(Line::from(Span::styled(String::new(), theme.dim_text())));
            right_lines.push(Line::from(Span::styled(String::new(), theme.dim_text())));

            let (paired_left, paired_right) = self.pair_lines(hunk, file_name, extension, theme, left_area.width, right_area.width);

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

        let total = cmp::max(left_lines.len(), right_lines.len());
        self.total_rendered_lines.set(total);

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

    fn render_inline_full_file(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let Some(file_diff) = &self.file_diff else { return };
        let Some(content) = &self.full_file_content else { return };

        let extension = Path::new(&file_diff.new_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let file_name = Path::new(&file_diff.new_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let full_lines: Vec<&str> = content.lines().collect();
        let mut rendered_lines: Vec<Line<'static>> = Vec::new();

        let hunk_ranges = build_hunk_ranges(file_diff);
        let mut hunk_idx = 0;
        let mut new_line_no: u32 = 1;
        let mut line_pos: usize = 0;

        while line_pos < full_lines.len() {
            if hunk_idx < hunk_ranges.len() && new_line_no == hunk_ranges[hunk_idx].new_start {
                let range = &hunk_ranges[hunk_idx];
                let hunk = &file_diff.hunks[range.hunk_idx];

                rendered_lines.push(Line::from(Span::styled(String::new(), Style::default())));
                let header_text = hunk.header.trim();
                rendered_lines.push(Line::from(Span::styled(header_text.to_string(), theme.diff_header())));

                let mut minus_buffer: Vec<&DiffLine> = Vec::new();
                let mut plus_buffer: Vec<&DiffLine> = Vec::new();
                let mut line_idx = 0;

                for diff_line in &hunk.lines {
                    match diff_line.line_type {
                        DiffLineType::Context | DiffLineType::Header => {
                            flush_buffer_inline(&mut rendered_lines, &mut line_idx, &mut minus_buffer, &mut plus_buffer, file_name, extension, theme, area.width);

                            let content_spans = DiffEngine::highlight_line(
                                &diff_line.content,
                                diff_line.line_type.clone(),
                                file_name,
                                extension,
                                theme,
                            );
                            let prefix_spans = build_dual_prefix_spans(
                                diff_line.old_lineno,
                                diff_line.new_lineno,
                                theme.line_number_style(),
                                theme.line_number_style(),
                                theme.line_number_column_style(),
                            );
                            wrap_and_push(&mut rendered_lines, &prefix_spans, &content_spans, area.width as usize, theme.line_number_column_style());
                            line_idx += 1;
                        }
                        DiffLineType::Delete => minus_buffer.push(diff_line),
                        DiffLineType::Add => plus_buffer.push(diff_line),
                    }
                }

                flush_buffer_inline(&mut rendered_lines, &mut line_idx, &mut minus_buffer, &mut plus_buffer, file_name, extension, theme, area.width);

                new_line_no += range.new_count;
                line_pos += range.new_count as usize;
                hunk_idx += 1;
            } else {
                let line_content = full_lines[line_pos];
                let prefix_spans = build_dual_prefix_spans(
                    None,
                    Some(new_line_no),
                    theme.line_number_style(),
                    theme.line_number_style(),
                    theme.line_number_column_style(),
                );
                let content_spans = DiffEngine::highlight_line(
                    line_content,
                    DiffLineType::Context,
                    file_name,
                    extension,
                    theme,
                );
                wrap_and_push(&mut rendered_lines, &prefix_spans, &content_spans, area.width as usize, theme.line_number_column_style());
                new_line_no += 1;
                line_pos += 1;
            }
        }

        let total = rendered_lines.len();
        self.total_rendered_lines.set(total);

        let visible_lines: Vec<Line> = rendered_lines
            .into_iter()
            .skip(self.scroll)
            .take(area.height as usize)
            .collect();

        let para = Paragraph::new(visible_lines);
        f.render_widget(para, area);
    }

    fn render_side_by_side_full_file(&self, f: &mut Frame, area: Rect, theme: &Theme) {
        let Some(file_diff) = &self.file_diff else { return };
        let Some(content) = &self.full_file_content else { return };

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
        let file_name = Path::new(&file_diff.new_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let new_lines: Vec<&str> = content.lines().collect();
        let hunk_ranges = build_hunk_ranges(file_diff);

        let mut left_rendered: Vec<Line<'static>> = Vec::new();
        let mut right_rendered: Vec<Line<'static>> = Vec::new();
        let column_style = theme.line_number_column_style();

        let mut hunk_idx = 0;

        for (i, new_line_content) in new_lines.iter().enumerate() {
            let new_lineno = (i + 1) as u32;

            if hunk_idx < hunk_ranges.len() && new_lineno == hunk_ranges[hunk_idx].new_start {
                let range = &hunk_ranges[hunk_idx];
                let hunk = &file_diff.hunks[range.hunk_idx];

                left_rendered.push(Line::from(Span::styled(String::new(), Style::default())));
                right_rendered.push(Line::from(Span::styled(String::new(), Style::default())));
                let header_text = hunk.header.trim();
                left_rendered.push(Line::from(Span::styled(header_text.to_string(), theme.diff_header())));
                let underline = "─".repeat(left_area.width.saturating_sub(1) as usize);
                left_rendered.push(Line::from(Span::styled(underline, theme.diff_header())));
                right_rendered.push(Line::from(Span::styled(String::new(), theme.dim_text())));
                right_rendered.push(Line::from(Span::styled(String::new(), theme.dim_text())));

                let (paired_left, paired_right) = self.pair_lines(hunk, file_name, extension, theme, left_area.width, right_area.width);
                left_rendered.extend(paired_left);
                right_rendered.extend(paired_right);

                hunk_idx += 1;
                continue;
            }

            if hunk_idx < hunk_ranges.len() && new_lineno >= hunk_ranges[hunk_idx].new_start {
                if new_lineno < hunk_ranges[hunk_idx].new_start + hunk_ranges[hunk_idx].new_count {
                    continue;
                }
            }

            let left_spans = DiffEngine::highlight_line(new_line_content, DiffLineType::Context, file_name, extension, theme);
            let left_prefix = build_prefix_spans('│', new_lineno, theme.line_number_style(), column_style);
            let right_spans = DiffEngine::highlight_line(new_line_content, DiffLineType::Context, file_name, extension, theme);
            let right_prefix = build_prefix_spans('│', new_lineno, theme.line_number_style(), column_style);
            wrap_and_push_pair(&mut left_rendered, &mut right_rendered, &left_prefix, &right_prefix, &left_spans, &right_spans, left_area.width as usize, right_area.width as usize, column_style);
        }

        let total = cmp::max(left_rendered.len(), right_rendered.len());
        self.total_rendered_lines.set(total);

        let visible_left: Vec<Line> = left_rendered.into_iter()
            .skip(self.scroll)
            .take(area.height as usize)
            .collect();
        let visible_right: Vec<Line> = right_rendered.into_iter()
            .skip(self.scroll)
            .take(area.height as usize)
            .collect();

        f.render_widget(Paragraph::new(visible_left), left_area);
        f.render_widget(Paragraph::new(visible_right), right_area);
    }

    fn pair_lines(&self, hunk: &Hunk, file_name: &str, extension: &str, theme: &Theme, left_width: u16, right_width: u16) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
        let mut left: Vec<Line<'static>> = Vec::new();
        let mut right: Vec<Line<'static>> = Vec::new();
        let mut delete_lines: Vec<&DiffLine> = Vec::new();
        let mut add_lines: Vec<&DiffLine> = Vec::new();

        let column_style = theme.line_number_column_style();

        for line in &hunk.lines {
            match line.line_type {
                DiffLineType::Context | DiffLineType::Header => {
                    flush_buffer_sbs(&mut left, &mut right, &mut delete_lines, &mut add_lines, file_name, extension, theme, left_width, right_width);
                    let spans = DiffEngine::highlight_line(
                        &line.content,
                        line.line_type.clone(),
                        file_name,
                        extension,
                        theme,
                    );
                    let left_lineno = line.old_lineno;
                    let right_lineno = line.new_lineno;
                    let left_prefix_spans = build_prefix_spans('│', left_lineno.unwrap_or(0), theme.line_number_style(), column_style);
                    let right_prefix_spans = build_prefix_spans('│', right_lineno.unwrap_or(0), theme.line_number_style(), column_style);
                    wrap_and_push_pair(&mut left, &mut right, &left_prefix_spans, &right_prefix_spans, &spans, &spans, left_width as usize, right_width as usize, column_style);
                }
                DiffLineType::Delete => delete_lines.push(line),
                DiffLineType::Add => add_lines.push(line),
            }
        }

        flush_buffer_sbs(&mut left, &mut right, &mut delete_lines, &mut add_lines, file_name, extension, theme, left_width, right_width);

        (left, right)
    }

    fn render_binary(&self, f: &mut Frame, area: Rect, file_diff: &FileDiff, theme: &Theme) {
        let size = format_bytes(file_diff.sizes.0, file_diff.sizes.1);
        let is_positive = file_diff.size_delta >= 0;
        let delta = if is_positive {
            format!("+{}", format_bytes_single(file_diff.size_delta as u64))
        } else {
            format!("-{}", format_bytes_single(file_diff.size_delta.unsigned_abs()))
        };
        let delta_style = if is_positive {
            theme.diff_add(false)
        } else {
            theme.diff_delete(false)
        };

        let lines = vec![Line::from(vec![
            Span::styled(format!(" size: {} -> {} ", size.0, size.1), theme.dim_text()),
            Span::styled(format!("({})", delta), delta_style),
        ])];

        f.render_widget(Paragraph::new(lines), area);
    }
}

fn flush_buffer_inline(
    lines: &mut Vec<Line<'static>>,
    line_idx: &mut usize,
    minus_buffer: &mut Vec<&DiffLine>,
    plus_buffer: &mut Vec<&DiffLine>,
    file_name: &str,
    extension: &str,
    theme: &Theme,
    area_width: u16,
) {
    let column_style = theme.line_number_column_style();
    let pair_count = cmp::max(minus_buffer.len(), plus_buffer.len());
    for i in 0..pair_count {
        if i < minus_buffer.len() && i < plus_buffer.len() {
            let minus_line = &minus_buffer[i];
            let plus_line = &plus_buffer[i];
            let (minus_spans, plus_spans) = DiffEngine::highlight_line_pair(
                &minus_line.content,
                &plus_line.content,
                file_name,
                extension,
                theme,
            );

            let prefix_spans = build_dual_prefix_spans(
                minus_line.old_lineno,
                None,
                theme.line_number_minus_style(),
                theme.line_number_style(),
                column_style,
            );
            wrap_and_push(lines, &prefix_spans, &minus_spans, area_width as usize, column_style);
            *line_idx += 1;

            let prefix_spans = build_dual_prefix_spans(
                None,
                plus_line.new_lineno,
                theme.line_number_style(),
                theme.line_number_plus_style(),
                column_style,
            );
            wrap_and_push(lines, &prefix_spans, &plus_spans, area_width as usize, column_style);
            *line_idx += 1;
        } else if i < minus_buffer.len() {
            let line = minus_buffer[i];
            let content_spans = DiffEngine::highlight_line(
                &line.content,
                DiffLineType::Delete,
                file_name,
                extension,
                theme,
            );
            let prefix_spans = build_dual_prefix_spans(
                line.old_lineno,
                None,
                theme.line_number_minus_style(),
                theme.line_number_style(),
                column_style,
            );
            wrap_and_push(lines, &prefix_spans, &content_spans, area_width as usize, column_style);
            *line_idx += 1;
        } else {
            let line = plus_buffer[i];
            let content_spans = DiffEngine::highlight_line(
                &line.content,
                DiffLineType::Add,
                file_name,
                extension,
                theme,
            );
            let prefix_spans = build_dual_prefix_spans(
                None,
                line.new_lineno,
                theme.line_number_style(),
                theme.line_number_plus_style(),
                column_style,
            );
            wrap_and_push(lines, &prefix_spans, &content_spans, area_width as usize, column_style);
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
    file_name: &str,
    extension: &str,
    theme: &Theme,
    left_width: u16,
    right_width: u16,
) {
    let column_style = theme.line_number_column_style();
    let empty_prefix = "│    │ ".to_string();
    let pair_count = cmp::max(dels.len(), adds.len());
    for i in 0..pair_count {
        if i < dels.len() && i < adds.len() {
            let (minus_spans, plus_spans) = DiffEngine::highlight_line_pair(
                &dels[i].content,
                &adds[i].content,
                file_name,
                extension,
                theme,
            );
            let left_prefix_spans = build_prefix_spans('│', dels[i].old_lineno.unwrap_or(0), theme.line_number_minus_style(), column_style);
            let right_prefix_spans = build_prefix_spans('│', adds[i].new_lineno.unwrap_or(0), theme.line_number_plus_style(), column_style);
            wrap_and_push_pair(left, right, &left_prefix_spans, &right_prefix_spans, &minus_spans, &plus_spans, left_width as usize, right_width as usize, column_style);
        } else if i < dels.len() {
            let line = dels[i];
            let spans = DiffEngine::highlight_line(
                &line.content,
                DiffLineType::Delete,
                file_name,
                extension,
                theme,
            );
            let left_prefix_spans = build_prefix_spans('│', line.old_lineno.unwrap_or(0), theme.line_number_minus_style(), column_style);
            let right_prefix_spans = vec![Span::styled(empty_prefix.to_string(), column_style)];
            let left_content_width = (left_width as usize).saturating_sub(left_prefix_spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum::<usize>());
            let right_content_width = (right_width as usize).saturating_sub(right_prefix_spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum::<usize>());
            let row_count = chunk_spans(&spans, left_content_width).len();
            let indicator = build_empty_indicator(right_content_width, row_count, theme);
            wrap_and_push_pair(left, right, &left_prefix_spans, &right_prefix_spans, &spans, &indicator, left_width as usize, right_width as usize, column_style);
        } else {
            let line = adds[i];
            let spans = DiffEngine::highlight_line(
                &line.content,
                DiffLineType::Add,
                file_name,
                extension,
                theme,
            );
            let left_prefix_spans = vec![Span::styled(empty_prefix.to_string(), column_style)];
            let right_prefix_spans = build_prefix_spans('│', line.new_lineno.unwrap_or(0), theme.line_number_plus_style(), column_style);
            let left_content_width = (left_width as usize).saturating_sub(left_prefix_spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum::<usize>());
            let right_content_width = (right_width as usize).saturating_sub(right_prefix_spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum::<usize>());
            let row_count = chunk_spans(&spans, right_content_width).len();
            let indicator = build_empty_indicator(left_content_width, row_count, theme);
            wrap_and_push_pair(left, right, &left_prefix_spans, &right_prefix_spans, &indicator, &spans, left_width as usize, right_width as usize, column_style);
        }
    }
    dels.clear();
    adds.clear();
}

fn build_empty_indicator(content_width: usize, rows: usize, theme: &Theme) -> Vec<Span<'static>> {
    if content_width == 0 || rows == 0 {
        return Vec::new();
    }
    let indicator_bg = if theme.diff_add_bg != Color::Reset {
        Color::Rgb(0x20, 0x20, 0x20)
    } else {
        Color::Rgb(0xe0, 0xe0, 0xe0)
    };
    let mut s = String::with_capacity(content_width);
    let pattern = "//";
    while s.len() < content_width {
        s.push_str(pattern);
    }
    s.truncate(content_width);
    (0..rows)
        .map(|_| Span::styled(s.clone(), Style::default().fg(theme.dim_text).bg(indicator_bg)))
        .collect()
}

fn build_prefix_spans(sign: char, lineno: u32, number_style: Style, column_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if sign == '│' {
        spans.push(Span::styled("│".to_string(), column_style));
    } else {
        spans.push(Span::styled(format!("{}│", sign), column_style));
    }
    spans.push(Span::styled(format!("{:^4}", lineno), number_style));
    spans.push(Span::styled("│ ".to_string(), column_style));
    spans
}

fn build_dual_prefix_spans(
    old_lineno: Option<u32>,
    new_lineno: Option<u32>,
    left_style: Style,
    right_style: Style,
    column_style: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let left_text = match old_lineno {
        Some(n) => format!("{:^4}", n),
        None => "    ".to_string(),
    };
    spans.push(Span::styled(left_text, left_style));
    spans.push(Span::styled("│".to_string(), column_style));
    let right_text = match new_lineno {
        Some(n) => format!("{:^4}", n),
        None => "    ".to_string(),
    };
    spans.push(Span::styled(right_text, right_style));
    spans.push(Span::styled("│ ".to_string(), column_style));
    spans
}

fn fill_rest_of_line(line_buf: &mut Vec<Span<'static>>, max_width: usize, fill_bg: Option<ratatui::style::Color>) {
    let line_width: usize = line_buf.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
    if line_width < max_width {
        if let Some(bg) = fill_bg {
            let fill_len = max_width - line_width;
            line_buf.push(Span::styled(
                " ".repeat(fill_len),
                Style::default().bg(bg),
            ));
        }
    }
}

fn wrap_and_push(
    lines: &mut Vec<Line<'static>>,
    prefix_spans: &[Span<'static>],
    content_spans: &[Span<'static>],
    max_width: usize,
    cont_column_style: Style,
) {
    let prefix_width: usize = prefix_spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
    let content_width = max_width.saturating_sub(prefix_width);
    let fill_bg = content_spans.iter().find_map(|s| s.style.bg);

    if content_width == 0 {
        let mut line_buf = Vec::new();
        line_buf.extend(prefix_spans.iter().cloned());
        fill_rest_of_line(&mut line_buf, max_width, fill_bg);
        lines.push(Line::from(line_buf));
        return;
    }

    let mut line_buf = Vec::new();
    line_buf.extend(prefix_spans.iter().cloned());

    let mut current_width = prefix_width;

    for span in content_spans {
        let s = span.content.as_ref();
        let span_width = UnicodeWidthStr::width(s);

        if current_width + span_width <= max_width {
            line_buf.push(Span::styled(s.to_string(), span.style));
            current_width += span_width;
        } else {
            let available = max_width.saturating_sub(current_width);
            let mut chars_consumed: usize = 0;
            if available > 0 {
                let mut truncated = String::new();
                let mut w = 0;
                for c in s.chars() {
                    let cw = UnicodeWidthStr::width(c.to_string().as_str());
                    if w + cw > available && chars_consumed > 0 {
                        break;
                    }
                    truncated.push(c);
                    w += cw;
                    chars_consumed += 1;
                }
                line_buf.push(Span::styled(truncated, span.style));
            }
            fill_rest_of_line(&mut line_buf, max_width, fill_bg);
            lines.push(Line::from(line_buf));

            line_buf = Vec::new();
            line_buf.push(Span::styled("    │    │ ".to_string(), cont_column_style));
            let mut cont_width = UnicodeWidthStr::width("    │    │ ");

            let rest_text = s.chars().skip(chars_consumed).collect::<String>();
            if !rest_text.is_empty() {
                cont_width += UnicodeWidthStr::width(rest_text.as_str());
                line_buf.push(Span::styled(rest_text, span.style));
            }
            current_width = cont_width;
        }
    }

    if !line_buf.is_empty() {
        fill_rest_of_line(&mut line_buf, max_width, fill_bg);
        lines.push(Line::from(line_buf));
    }
}

fn wrap_and_push_pair(
    left: &mut Vec<Line<'static>>,
    right: &mut Vec<Line<'static>>,
    left_prefix_spans: &[Span<'static>],
    right_prefix_spans: &[Span<'static>],
    left_spans: &[Span<'static>],
    right_spans: &[Span<'static>],
    left_max_width: usize,
    right_max_width: usize,
    cont_column_style: Style,
) {
    let left_prefix_width: usize = left_prefix_spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
    let right_prefix_width: usize = right_prefix_spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
    let left_content_width = left_max_width.saturating_sub(left_prefix_width);
    let right_content_width = right_max_width.saturating_sub(right_prefix_width);
    let left_fill_bg = left_spans.iter().find_map(|s| s.style.bg);
    let right_fill_bg = right_spans.iter().find_map(|s| s.style.bg);

    if left_content_width == 0 && right_content_width == 0 {
        let mut lspans = Vec::new();
        lspans.extend(left_prefix_spans.iter().cloned());
        fill_rest_of_line(&mut lspans, left_max_width, left_fill_bg);
        left.push(Line::from(lspans));
        let mut rspans = Vec::new();
        rspans.extend(right_prefix_spans.iter().cloned());
        fill_rest_of_line(&mut rspans, right_max_width, right_fill_bg);
        right.push(Line::from(rspans));
        return;
    }

    let left_chunks = chunk_spans(left_spans, left_content_width);
    let right_chunks = chunk_spans(right_spans, right_content_width);

    let max_chunks = cmp::max(left_chunks.len(), right_chunks.len());

    let empty_cont_prefix = "│    │ ";

    for i in 0..max_chunks {
        let lprefix = if i == 0 { None } else { Some(empty_cont_prefix) };
        let rprefix = if i == 0 { None } else { Some(empty_cont_prefix) };

        let mut lspans = Vec::new();
        if let Some(p) = lprefix {
            lspans.push(Span::styled(p.to_string(), cont_column_style));
        } else {
            lspans.extend(left_prefix_spans.iter().cloned());
        }
        if i < left_chunks.len() {
            lspans.extend(left_chunks[i].clone());
        }
        fill_rest_of_line(&mut lspans, left_max_width, left_fill_bg);
        left.push(Line::from(lspans));

        let mut rspans = Vec::new();
        if let Some(p) = rprefix {
            rspans.push(Span::styled(p.to_string(), cont_column_style));
        } else {
            rspans.extend(right_prefix_spans.iter().cloned());
        }
        if i < right_chunks.len() {
            rspans.extend(right_chunks[i].clone());
        }
        fill_rest_of_line(&mut rspans, right_max_width, right_fill_bg);
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
                    if w + cw > available && chars_consumed > 0 {
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
                    for _ in 0..chars_consumed {
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

struct HunkRange {
    new_start: u32,
    new_count: u32,
    hunk_idx: usize,
}

fn parse_hunk_header(header: &str) -> (u32, u32, u32, u32) {
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 3 {
        return (0, 0, 0, 0);
    }
    let old_part = parts[1].trim_start_matches('-');
    let new_part = parts[2].trim_start_matches('+');

    let old_parts: Vec<&str> = old_part.split(',').collect();
    let new_parts: Vec<&str> = new_part.split(',').collect();

    let old_start: u32 = old_parts[0].parse().unwrap_or(0);
    let old_count: u32 = old_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let new_start: u32 = new_parts[0].parse().unwrap_or(0);
    let new_count: u32 = new_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

    (old_start, old_count, new_start, new_count)
}

fn build_hunk_ranges(file_diff: &FileDiff) -> Vec<HunkRange> {
    file_diff
        .hunks
        .iter()
        .enumerate()
        .map(|(i, hunk)| {
            let (_old_start, _old_count, new_start, new_count) = parse_hunk_header(&hunk.header);
            HunkRange {
                new_start,
                new_count,
                hunk_idx: i,
            }
        })
        .collect()
}

fn format_bytes(old_bytes: u64, new_bytes: u64) -> (String, String) {
    (format_bytes_single(old_bytes), format_bytes_single(new_bytes))
}

fn format_bytes_single(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;

    if bytes >= MB {
        let mb = bytes as f64 / MB as f64;
        format!("{:.1} MB", mb)
    } else if bytes >= KB {
        let kb = bytes as f64 / KB as f64;
        format!("{:.1} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}