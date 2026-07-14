use crate::edits::{self, EditOp};
use crate::theme::Theme;
use lazy_static::lazy_static;
use ratatui::style::Style;
use ratatui::text::Span;
use regex::Regex;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

lazy_static! {
    static ref DEFAULT_TOKENIZATION_REGEX: Regex = Regex::new(r#"\w+"#).unwrap();
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

pub struct DiffEngine;

impl DiffEngine {
    pub fn highlight_line(
        line: &str,
        line_type: crate::git::DiffLineType,
        extension: &str,
        theme: &Theme,
    ) -> Vec<Span<'static>> {
        let base_style = match line_type {
            crate::git::DiffLineType::Add => theme.diff_add,
            crate::git::DiffLineType::Delete => theme.diff_delete,
            crate::git::DiffLineType::Context => theme.diff_context,
            crate::git::DiffLineType::Header => theme.diff_header,
        };

        Self::syntax_highlight(line, extension, base_style)
    }

    pub fn highlight_line_pair(
        minus_line: &str,
        plus_line: &str,
        extension: &str,
        theme: &Theme,
    ) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
        let (annotated_minus, annotated_plus, _alignment) = edits::infer_edits(
            vec![minus_line],
            vec![plus_line],
            &DEFAULT_TOKENIZATION_REGEX,
            1.0,
            0.0,
        );

        let minus_spans = Self::annotated_line_to_spans(
            &annotated_minus[0],
            extension,
            theme.diff_delete,
            theme.diff_delete_highlight,
            theme,
        );
        let plus_spans = Self::annotated_line_to_spans(
            &annotated_plus[0],
            extension,
            theme.diff_add,
            theme.diff_add_highlight,
            theme,
        );

        (minus_spans, plus_spans)
    }

    fn annotated_line_to_spans(
        segments: &[(EditOp, &str)],
        extension: &str,
        base_style: Style,
        emph_style: Style,
        theme: &Theme,
    ) -> Vec<Span<'static>> {
        let full_line: String = segments.iter().map(|(_, s)| *s).collect();
        let syntax_segments = Self::get_syntax_segments(&full_line, extension);

        let mut spans = Vec::new();
        let mut pos = 0;

        for (op, segment) in segments {
            let seg_start = pos;
            let seg_end = pos + segment.len();
            pos = seg_end;

            let diff_style = match op {
                EditOp::Deletion | EditOp::Insertion => emph_style,
                EditOp::NoOp => base_style,
            };

            let seg_spans = Self::merge_syntax_into_diff(
                &syntax_segments,
                seg_start,
                seg_end,
                diff_style,
                theme,
            );
            spans.extend(seg_spans);
        }

        spans
    }

    fn get_syntax_segments(full_line: &str, extension: &str) -> Vec<(Style, String)> {
        if extension.is_empty() {
            return vec![(Style::default(), full_line.to_string())];
        }

        let syntax = match SYNTAX_SET.find_syntax_by_extension(extension) {
            Some(s) => s,
            None => return vec![(Style::default(), full_line.to_string())],
        };

        let theme = &THEME_SET.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut segments = Vec::new();

        for line in LinesWithEndings::from(full_line) {
            if let Ok(ranges) = highlighter.highlight_line(line, &SYNTAX_SET) {
                for (syntect_style, text) in &ranges {
                    let fg = syntect_style.foreground;
                    let style = Style::default()
                        .fg(ratatui::style::Color::Rgb(fg.r, fg.g, fg.b));
                    segments.push((style, text.to_string()));
                }
            }
        }

        if segments.is_empty() {
            segments.push((Style::default(), full_line.to_string()));
        }

        segments
    }

    fn merge_syntax_into_diff(
        syntax_segments: &[(Style, String)],
        range_start: usize,
        range_end: usize,
        diff_style: Style,
        theme: &Theme,
    ) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut char_pos = 0;

        for (syntax_style, text) in syntax_segments {
            let seg_len = text.len();
            let seg_start = char_pos;
            let seg_end = char_pos + seg_len;
            char_pos = seg_end;

            if seg_end <= range_start || seg_start >= range_end {
                continue;
            }

            let start = if seg_start < range_start {
                range_start - seg_start
            } else {
                0
            };
            let end = if seg_end > range_end {
                seg_len - (seg_end - range_end)
            } else {
                seg_len
            };

            if start >= end {
                continue;
            }

            let part = &text[start..end];

            let fg = if diff_style.fg.is_some() {
                diff_style.fg.unwrap()
            } else {
                syntax_style.fg.unwrap_or(ratatui::style::Color::Reset)
            };

            let bg = if diff_style.bg.is_some() {
                diff_style.bg.unwrap()
            } else {
                theme.diff_context.bg.unwrap_or(ratatui::style::Color::Reset)
            };

            let final_style = Style::default().fg(fg).bg(bg);
            spans.push(Span::styled(part.to_string(), final_style));
        }

        if spans.is_empty() {
            let text = &full_display_text(syntax_segments)[range_start..range_end];
            let fg = diff_style.fg.unwrap_or(ratatui::style::Color::Reset);
            let bg = if diff_style.bg.is_some() {
                diff_style.bg.unwrap()
            } else {
                theme.diff_context.bg.unwrap_or(ratatui::style::Color::Reset)
            };
            spans.push(Span::styled(
                text.to_string(),
                Style::default().fg(fg).bg(bg),
            ));
        }

        spans
    }

    fn syntax_highlight(
        line: &str,
        extension: &str,
        base_style: Style,
    ) -> Vec<Span<'static>> {
        if extension.is_empty() {
            return vec![Span::styled(line.to_string(), base_style)];
        }

        let syntax = match SYNTAX_SET.find_syntax_by_extension(extension) {
            Some(s) => s,
            None => return vec![Span::styled(line.to_string(), base_style)],
        };

        let theme = &THEME_SET.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut spans = Vec::new();

        for line in LinesWithEndings::from(line) {
            if let Ok(ranges) = highlighter.highlight_line(line, &SYNTAX_SET) {
                for (syntect_style, text) in &ranges {
                    let syntect_fg = syntect_style.foreground;
                    let fg = ratatui::style::Color::Rgb(syntect_fg.r, syntect_fg.g, syntect_fg.b);
                    let bg = base_style.bg.unwrap_or(ratatui::style::Color::Reset);
                    let style = Style::default().fg(fg).bg(bg);
                    spans.push(Span::styled(text.to_string(), style));
                }
            }
        }

        if spans.is_empty() {
            spans.push(Span::styled(line.to_string(), base_style));
        }

        spans
    }
}

fn full_display_text(segments: &[(Style, String)]) -> String {
    segments.iter().map(|(_, s)| s.as_str()).collect()
}