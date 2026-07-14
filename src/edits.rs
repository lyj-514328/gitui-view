use regex::Regex;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::align;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditOp {
    NoOp,
    Deletion,
    Insertion,
}

pub fn infer_edits<'a>(
    minus_lines: Vec<&'a str>,
    plus_lines: Vec<&'a str>,
    tokenization_regex: &Regex,
    max_line_distance: f64,
    max_line_distance_for_naively_paired_lines: f64,
) -> (
    Vec<Vec<(EditOp, &'a str)>>,
    Vec<Vec<(EditOp, &'a str)>>,
    Vec<(Option<usize>, Option<usize>)>,
) {
    let mut annotated_minus_lines = Vec::new();
    let mut annotated_plus_lines = Vec::new();
    let mut line_alignment = Vec::new();

    let mut plus_index = 0;

    'minus_lines_loop: for (minus_index, minus_line) in minus_lines.iter().enumerate() {
        let mut considered = 0;
        for plus_line in &plus_lines[plus_index..] {
            let alignment = align::Alignment::new(
                tokenize(minus_line, tokenization_regex),
                tokenize(plus_line, tokenization_regex),
            );
            let (annotated_minus_line, annotated_plus_line, distance) = annotate(
                alignment,
                minus_line,
                plus_line,
            );
            if minus_lines.len() == plus_lines.len()
                && distance <= max_line_distance_for_naively_paired_lines
                || distance <= max_line_distance
            {
                for _ in 0..considered {
                    annotated_plus_lines.push(vec![(EditOp::NoOp, plus_lines[plus_index])]);
                    line_alignment.push((None, Some(plus_index)));
                    plus_index += 1;
                }
                annotated_minus_lines.push(annotated_minus_line);
                annotated_plus_lines.push(annotated_plus_line);
                line_alignment.push((Some(minus_index), Some(plus_index)));
                plus_index += 1;
                continue 'minus_lines_loop;
            } else {
                considered += 1;
            }
        }
        annotated_minus_lines.push(vec![(EditOp::NoOp, minus_line)]);
        line_alignment.push((Some(minus_index), None));
    }
    for plus_line in &plus_lines[plus_index..] {
        annotated_plus_lines.push(vec![(EditOp::NoOp, plus_line)]);
        line_alignment.push((None, Some(plus_index)));
        plus_index += 1;
    }

    (annotated_minus_lines, annotated_plus_lines, line_alignment)
}

fn tokenize<'a>(line: &'a str, regex: &Regex) -> Vec<&'a str> {
    let mut tokens = vec![""];
    let mut offset = 0;
    for m in regex.find_iter(line) {
        if offset == 0 && m.start() > 0 {
            tokens.push("");
        }
        for t in line[offset..m.start()].graphemes(true) {
            tokens.push(t);
        }
        tokens.push(&line[m.start()..m.end()]);
        offset = m.end();
    }
    if offset < line.len() {
        if offset == 0 {
            tokens.push("");
        }
        for t in line[offset..line.len()].graphemes(true) {
            tokens.push(t);
        }
    }
    tokens
}

fn annotate<'a>(
    alignment: align::Alignment<'a>,
    minus_line: &'a str,
    plus_line: &'a str,
) -> (Vec<(EditOp, &'a str)>, Vec<(EditOp, &'a str)>, f64) {
    let mut annotated_minus_line = Vec::new();
    let mut annotated_plus_line = Vec::new();

    let (mut x_offset, mut y_offset) = (0, 0);
    let (mut minus_line_offset, mut plus_line_offset) = (0, 0);
    let (mut d_numer, mut d_denom) = (0, 0);

    let get_section = |n: usize,
                       line_offset: &mut usize,
                       substrings_offset: &mut usize,
                       substrings: &[&str],
                       line: &'a str| {
        let section_length = substrings[*substrings_offset..*substrings_offset + n]
            .iter()
            .fold(0, |n, s| n + s.len());
        let old_offset = *line_offset;
        *line_offset += section_length;
        *substrings_offset += n;
        &line[old_offset..*line_offset]
    };
    let mut minus_section = |n: usize, offset: &mut usize| {
        get_section(n, &mut minus_line_offset, offset, &alignment.x, minus_line)
    };
    let mut plus_section = |n: usize, offset: &mut usize| {
        get_section(n, &mut plus_line_offset, offset, &alignment.y, plus_line)
    };
    let distance_contribution = |section: &str| UnicodeWidthStr::width(section.trim());

    let (mut minus_op_prev, mut plus_op_prev) = (EditOp::NoOp, EditOp::NoOp);
    for (op, n) in alignment.coalesced_operations() {
        match op {
            align::Operation::Deletion => {
                let minus_section = minus_section(n, &mut x_offset);
                let n_d = distance_contribution(minus_section);
                d_denom += n_d;
                d_numer += n_d;
                annotated_minus_line.push((EditOp::Deletion, minus_section));
                minus_op_prev = EditOp::Deletion;
            }
            align::Operation::NoOp => {
                let minus_section = minus_section(n, &mut x_offset);
                let n_d = distance_contribution(minus_section);
                d_denom += 2 * n_d;
                let is_space = minus_section.trim().is_empty();
                let coalesce_space_with_previous = is_space
                    && ((minus_op_prev == EditOp::Deletion
                        && plus_op_prev == EditOp::Insertion
                        && (x_offset < alignment.x.len() - 1 || y_offset < alignment.y.len() - 1))
                        || (minus_op_prev == EditOp::NoOp && plus_op_prev == EditOp::NoOp));
                annotated_minus_line.push((
                    if coalesce_space_with_previous {
                        minus_op_prev
                    } else {
                        EditOp::NoOp
                    },
                    minus_section,
                ));
                let op = if coalesce_space_with_previous {
                    plus_op_prev
                } else {
                    EditOp::NoOp
                };
                let plus_section = plus_section(n, &mut y_offset);
                annotated_plus_line.push((op, plus_section));
                minus_op_prev = EditOp::NoOp;
                plus_op_prev = EditOp::NoOp;
            }
            align::Operation::Insertion => {
                let plus_section = plus_section(n, &mut y_offset);
                let n_d = distance_contribution(plus_section);
                d_denom += n_d;
                d_numer += n_d;
                annotated_plus_line.push((EditOp::Insertion, plus_section));
                plus_op_prev = EditOp::Insertion;
            }
        }
    }
    (
        annotated_minus_line,
        annotated_plus_line,
        compute_distance(d_numer as f64, d_denom as f64),
    )
}

fn compute_distance(d_numer: f64, d_denom: f64) -> f64 {
    if d_denom > 0.0 {
        d_numer / d_denom
    } else {
        0.0
    }
}