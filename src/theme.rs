use ratatui::style::{Color, Modifier, Style};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Theme {
    pub tab_active: Color,
    pub tab_inactive: Color,
    pub tab_bar_bg: Color,
    pub diff_add_fg: Color,
    pub diff_add_bg: Color,
    pub diff_delete_fg: Color,
    pub diff_delete_bg: Color,
    pub diff_context_bg: Color,
    pub diff_header_fg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub title_fg: Color,
    pub dim_text: Color,
    pub normal_bg: Color,
    pub light_bg: Color,
    pub file_entry: Color,
    pub file_entry_staged: Color,
    pub file_entry_modified: Color,
    pub file_entry_untracked: Color,
    pub commit_hash: Color,
    pub commit_msg: Color,
    pub commit_author: Color,
    pub commit_date: Color,
    pub stash_msg: Color,
    pub help_key: Color,
    pub help_desc: Color,
    pub mode_indicator_bg: Color,
    pub status_added: Color,
    pub status_deleted: Color,
    pub status_modified: Color,
    pub diff_delete_highlight: Style,
    pub diff_add_highlight: Style,
    pub line_number_fg: Color,
    pub syntax_theme_name: String,
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if s == "Reset" {
        return Some(Color::Reset);
    }
    if s == "Black" {
        return Some(Color::Black);
    }
    if s == "Red" {
        return Some(Color::Red);
    }
    if s == "Green" {
        return Some(Color::Green);
    }
    if s == "Yellow" {
        return Some(Color::Yellow);
    }
    if s == "Blue" {
        return Some(Color::Blue);
    }
    if s == "Magenta" {
        return Some(Color::Magenta);
    }
    if s == "Cyan" {
        return Some(Color::Cyan);
    }
    if s == "White" {
        return Some(Color::White);
    }
    if s == "DarkGray" {
        return Some(Color::DarkGray);
    }
    if s == "LightRed" {
        return Some(Color::LightRed);
    }
    if s == "LightGreen" {
        return Some(Color::LightGreen);
    }
    if s == "LightYellow" {
        return Some(Color::LightYellow);
    }
    if s == "LightBlue" {
        return Some(Color::LightBlue);
    }
    if s == "LightMagenta" {
        return Some(Color::LightMagenta);
    }
    if s == "LightCyan" {
        return Some(Color::LightCyan);
    }
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        return Some(Color::Rgb(r, g, b));
    }
    None
}

fn parse_ron_color_value(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') {
        return parse_color(&s[1..s.len() - 1]);
    }
    parse_color(s)
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            tab_active: Color::White,
            tab_inactive: Color::DarkGray,
            tab_bar_bg: Color::Reset,
            diff_add_fg: Color::Green,
            diff_add_bg: Color::Rgb(0x00, 0x28, 0x00),
            diff_delete_fg: Color::Red,
            diff_delete_bg: Color::Rgb(0x3f, 0x00, 0x01),
            diff_context_bg: Color::Reset,
            diff_header_fg: Color::Blue,
            selection_bg: Color::Blue,
            selection_fg: Color::White,
            border: Color::DarkGray,
            border_focused: Color::Reset,
            title_fg: Color::White,
            dim_text: Color::DarkGray,
            normal_bg: Color::Reset,
            light_bg: Color::Reset,
            file_entry: Color::White,
            file_entry_staged: Color::LightGreen,
            file_entry_modified: Color::Yellow,
            file_entry_untracked: Color::DarkGray,
            commit_hash: Color::Magenta,
            commit_msg: Color::White,
            commit_author: Color::Green,
            commit_date: Color::LightCyan,
            stash_msg: Color::LightMagenta,
            help_key: Color::White,
            help_desc: Color::DarkGray,
            mode_indicator_bg: Color::Reset,
            status_added: Color::LightGreen,
            status_deleted: Color::LightRed,
            status_modified: Color::Yellow,
            diff_delete_highlight: Style::default().fg(Color::LightRed).bg(Color::Rgb(0x90, 0x10, 0x11)),
            diff_add_highlight: Style::default().fg(Color::LightGreen).bg(Color::Rgb(0x00, 0x60, 0x00)),
            line_number_fg: Color::Rgb(0x58, 0x58, 0x58),
            syntax_theme_name: "Monokai Extended".to_string(),
        }
    }

    pub fn light() -> Self {
        Self {
            tab_active: Color::Black,
            tab_inactive: Color::DarkGray,
            tab_bar_bg: Color::Reset,
            diff_add_fg: Color::Green,
            diff_add_bg: Color::Rgb(0xd0, 0xff, 0xd0),
            diff_delete_fg: Color::Red,
            diff_delete_bg: Color::Rgb(0xff, 0xe0, 0xe0),
            diff_context_bg: Color::Reset,
            diff_header_fg: Color::Blue,
            selection_bg: Color::Blue,
            selection_fg: Color::White,
            border: Color::DarkGray,
            border_focused: Color::Reset,
            title_fg: Color::Black,
            dim_text: Color::DarkGray,
            normal_bg: Color::Reset,
            light_bg: Color::Reset,
            file_entry: Color::Black,
            file_entry_staged: Color::Green,
            file_entry_modified: Color::Yellow,
            file_entry_untracked: Color::DarkGray,
            commit_hash: Color::Magenta,
            commit_msg: Color::Black,
            commit_author: Color::Green,
            commit_date: Color::LightCyan,
            stash_msg: Color::LightMagenta,
            help_key: Color::Blue,
            help_desc: Color::DarkGray,
            mode_indicator_bg: Color::Reset,
            status_added: Color::Green,
            status_deleted: Color::Red,
            status_modified: Color::Yellow,
            diff_delete_highlight: Style::default().fg(Color::Red).bg(Color::Rgb(0xff, 0xc0, 0xc0)),
            diff_add_highlight: Style::default().fg(Color::Green).bg(Color::Rgb(0xa0, 0xef, 0xa0)),
            line_number_fg: Color::Rgb(0x99, 0x99, 0x99),
            syntax_theme_name: "GitHub".to_string(),
        }
    }

    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_ron(&content)
    }

    pub fn from_ron(input: &str) -> anyhow::Result<Self> {
        let mut theme = Theme::dark();
        let input = input.trim();

        if !input.starts_with('(') || !input.ends_with(')') {
            anyhow::bail!("Invalid RON format: expected (...)");
        }

        let inner = &input[1..input.len() - 1].trim();
        for field in split_ron_fields(inner) {
            let field = field.trim();
            if let Some(eq_pos) = field.find(':') {
                let key = field[..eq_pos].trim().trim_matches('"');
                let value = field[eq_pos + 1..].trim();

                if let Some(color) = parse_ron_color_value(value) {
                    apply_field(&mut theme, key, color);
                }
            }
        }

        Ok(theme)
    }

    // --- Tab ---
    pub fn tab_active_style(&self) -> Style {
        Style::default()
            .fg(self.tab_active)
            .add_modifier(Modifier::BOLD)
    }
    pub fn tab_inactive_style(&self) -> Style {
        Style::default().fg(self.tab_inactive)
    }
    pub fn tab_bar_style(&self) -> Style {
        Style::default()
    }

    // --- Diff ---
    pub fn diff_add(&self, selected: bool) -> Style {
        let base = Style::default().fg(self.diff_add_fg).bg(self.diff_add_bg);
        if selected {
            Style::default().fg(self.selection_fg).bg(self.selection_bg)
        } else {
            base
        }
    }
    pub fn diff_delete(&self, selected: bool) -> Style {
        let base = Style::default().fg(self.diff_delete_fg).bg(self.diff_delete_bg);
        if selected {
            Style::default().fg(self.selection_fg).bg(self.selection_bg)
        } else {
            base
        }
    }
    pub fn diff_context(&self, selected: bool) -> Style {
        let base = Style::default().bg(self.diff_context_bg);
        if selected {
            Style::default().fg(self.selection_fg).bg(self.selection_bg)
        } else {
            base
        }
    }
    pub fn diff_header(&self) -> Style {
        Style::default()
            .fg(self.diff_header_fg)
            .add_modifier(Modifier::BOLD)
    }

    // --- Selection ---
    pub fn selected(&self) -> Style {
        Style::default().fg(self.selection_fg).bg(self.selection_bg)
    }

    // --- UI chrome ---
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }
    pub fn border_focused_style(&self) -> Style {
        Style::default().fg(self.border_focused)
    }
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.title_fg)
            .add_modifier(Modifier::BOLD)
    }
    pub fn dim_text(&self) -> Style {
        Style::default().fg(self.dim_text)
    }
    pub fn line_number_style(&self) -> Style {
        Style::default().fg(self.line_number_fg)
    }
    pub fn normal(&self) -> Style {
        Style::default()
    }

    // --- Commit info ---
    pub fn commit_hash(&self, selected: bool) -> Style {
        if selected {
            self.selected()
        } else {
            Style::default().fg(self.commit_hash)
        }
    }
    pub fn commit_msg(&self, selected: bool) -> Style {
        if selected {
            self.selected()
        } else {
            Style::default().fg(self.commit_msg)
        }
    }

    // --- Stash ---
    pub fn stash_msg(&self, selected: bool) -> Style {
        if selected {
            self.selected()
        } else {
            Style::default().fg(self.stash_msg)
        }
    }

    // --- Help ---
    pub fn help_key(&self) -> Style {
        Style::default()
            .fg(self.help_key)
            .add_modifier(Modifier::BOLD)
    }
    pub fn help_desc(&self) -> Style {
        Style::default().fg(self.help_desc)
    }
}

fn split_ron_fields(input: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (i, c) in input.char_indices() {
        match c {
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => depth -= 1,
            ',' if depth == 0 => {
                fields.push(input[start..i].to_string());
                start = i + 1;
            }
            _ => {}
        }
    }

    let remaining = input[start..].trim();
    if !remaining.is_empty() {
        fields.push(remaining.to_string());
    }

    fields
}

fn apply_field(theme: &mut Theme, key: &str, color: Color) {
    match key {
        "tab_active" => theme.tab_active = color,
        "tab_inactive" => theme.tab_inactive = color,
        "tab_bar_bg" => theme.tab_bar_bg = color,
        "diff_add_fg" => theme.diff_add_fg = color,
        "diff_add_bg" => theme.diff_add_bg = color,
        "diff_delete_fg" => theme.diff_delete_fg = color,
        "diff_delete_bg" => theme.diff_delete_bg = color,
        "diff_context_bg" => theme.diff_context_bg = color,
        "diff_header_fg" => theme.diff_header_fg = color,
        "selection_bg" => theme.selection_bg = color,
        "selection_fg" => theme.selection_fg = color,
        "border" => theme.border = color,
        "border_focused" => theme.border_focused = color,
        "title_fg" => theme.title_fg = color,
        "dim_text" => theme.dim_text = color,
        "normal_bg" => theme.normal_bg = color,
        "light_bg" => theme.light_bg = color,
        "file_entry" => theme.file_entry = color,
        "file_entry_staged" => theme.file_entry_staged = color,
        "file_entry_modified" => theme.file_entry_modified = color,
        "file_entry_untracked" => theme.file_entry_untracked = color,
        "commit_hash" => theme.commit_hash = color,
        "commit_msg" => theme.commit_msg = color,
        "commit_author" => theme.commit_author = color,
        "commit_date" => theme.commit_date = color,
        "stash_msg" => theme.stash_msg = color,
        "help_key" => theme.help_key = color,
        "help_desc" => theme.help_desc = color,
        "mode_indicator_bg" => theme.mode_indicator_bg = color,
        "status_added" => theme.status_added = color,
        "status_deleted" => theme.status_deleted = color,
        "status_modified" => theme.status_modified = color,
        "line_number_fg" => theme.line_number_fg = color,
        _ => {}
    }
}