use crate::edits::{self, EditOp};
use crate::theme::Theme;
use lazy_static::lazy_static;
use ratatui::style::Style;
use ratatui::text::Span;
use regex::Regex;
use std::cell::RefCell;
use syntect::parsing::SyntaxReference;
use syntect::util::LinesWithEndings;

thread_local! {
    static BAT_ASSETS: RefCell<Option<bat::assets::HighlightingAssets>> = const { RefCell::new(None) };
}

lazy_static! {
    static ref DEFAULT_TOKENIZATION_REGEX: Regex = Regex::new(r#"\w+"#).unwrap();
}

pub fn init_bat_assets() {
    BAT_ASSETS.with(|a| {
        a.borrow_mut()
            .get_or_insert_with(bat::assets::HighlightingAssets::from_binary);
    });
}

fn with_assets<F, R>(f: F) -> R
where
    F: FnOnce(&bat::assets::HighlightingAssets) -> R,
{
    BAT_ASSETS.with(|a| {
        let guard = a.borrow();
        let assets = guard
            .as_ref()
            .expect("bat assets not initialized, call init_bat_assets() first");
        f(assets)
    })
}

fn get_theme(syntax_theme_name: &str) -> syntect::highlighting::Theme {
    with_assets(|assets| assets.get_theme(syntax_theme_name).clone())
}

pub struct DiffEngine;

impl DiffEngine {
    pub fn highlight_line(
        line: &str,
        line_type: crate::git::DiffLineType,
        file_name: &str,
        extension: &str,
        theme: &Theme,
    ) -> Vec<Span<'static>> {
        let base_style = match line_type {
            crate::git::DiffLineType::Add => theme.diff_add(false),
            crate::git::DiffLineType::Delete => theme.diff_delete(false),
            crate::git::DiffLineType::Context => theme.diff_context(false),
            crate::git::DiffLineType::Header => theme.diff_header(),
        };

        Self::syntax_highlight(line, file_name, extension, &theme.syntax_theme_name, base_style)
    }

    pub fn highlight_line_pair(
        minus_line: &str,
        plus_line: &str,
        file_name: &str,
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
            file_name,
            extension,
            &theme.syntax_theme_name,
            theme.diff_delete(false),
            theme.diff_delete_highlight,
            theme,
        );
        let plus_spans = Self::annotated_line_to_spans(
            &annotated_plus[0],
            file_name,
            extension,
            &theme.syntax_theme_name,
            theme.diff_add(false),
            theme.diff_add_highlight,
            theme,
        );

        (minus_spans, plus_spans)
    }

    fn find_syntax(file_name: &str, extension: &str) -> Option<&'static SyntaxReference> {
        with_assets(|assets| {
            let syntax_set = assets.get_syntax_set().ok()?;
            if !extension.is_empty() || file_name.len() > 4 {
                if let Some(syntax) = syntax_set
                    .find_syntax_by_extension(file_name)
                    .or_else(|| syntax_set.find_syntax_by_extension(extension))
                {
                    return Some(unsafe { std::mem::transmute::<&SyntaxReference, &'static SyntaxReference>(syntax) });
                }
            }
            None
        })
    }

    fn annotated_line_to_spans(
        segments: &[(EditOp, &str)],
        file_name: &str,
        extension: &str,
        syntax_theme_name: &str,
        base_style: Style,
        emph_style: Style,
        theme: &Theme,
    ) -> Vec<Span<'static>> {
        let full_line: String = segments.iter().map(|(_, s)| *s).collect();
        let syntax_segments = Self::get_syntax_segments(&full_line, file_name, extension, syntax_theme_name);

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

    fn get_syntax_segments(full_line: &str, file_name: &str, extension: &str, syntax_theme_name: &str) -> Vec<(Style, String)> {
        let syntax = match Self::find_syntax(file_name, extension) {
            Some(s) => s,
            None => return vec![(Style::default(), full_line.to_string())],
        };

        let syntect_theme = get_theme(syntax_theme_name);

        let mut highlighter = syntect::easy::HighlightLines::new(syntax, &syntect_theme);
        let mut segments = Vec::new();

        with_assets(|assets| {
            if let Some(syntax_set) = assets.get_syntax_set().ok() {
                for line in LinesWithEndings::from(full_line) {
                    if let Ok(ranges) = highlighter.highlight_line(line, syntax_set) {
                        for (syntect_style, text) in &ranges {
                            let fg = syntect_style.foreground;
                            let style = Style::default()
                                .fg(ratatui::style::Color::Rgb(fg.r, fg.g, fg.b));
                            segments.push((style, text.to_string()));
                        }
                    }
                }
            }
        });

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
                theme.diff_context(false).bg.unwrap_or(ratatui::style::Color::Reset)
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
                theme.diff_context(false).bg.unwrap_or(ratatui::style::Color::Reset)
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
        file_name: &str,
        extension: &str,
        syntax_theme_name: &str,
        base_style: Style,
    ) -> Vec<Span<'static>> {
        let syntax = match Self::find_syntax(file_name, extension) {
            Some(s) => s,
            None => return vec![Span::styled(line.to_string(), base_style)],
        };

        let syntect_theme = get_theme(syntax_theme_name);

        let mut highlighter = syntect::easy::HighlightLines::new(syntax, &syntect_theme);
        let mut spans = Vec::new();

        with_assets(|assets| {
            if let Some(syntax_set) = assets.get_syntax_set().ok() {
                for line in LinesWithEndings::from(line) {
                    if let Ok(ranges) = highlighter.highlight_line(line, syntax_set) {
                        for (syntect_style, text) in &ranges {
                            let syntect_fg = syntect_style.foreground;
                            let fg = ratatui::style::Color::Rgb(syntect_fg.r, syntect_fg.g, syntect_fg.b);
                            let bg = base_style.bg.unwrap_or(ratatui::style::Color::Reset);
                            let style = Style::default().fg(fg).bg(bg);
                            spans.push(Span::styled(text.to_string(), style));
                        }
                    }
                }
            }
        });

        if spans.is_empty() {
            spans.push(Span::styled(line.to_string(), base_style));
        }

        spans
    }
}

fn full_display_text(segments: &[(Style, String)]) -> String {
    segments.iter().map(|(_, s)| s.as_str()).collect()
}