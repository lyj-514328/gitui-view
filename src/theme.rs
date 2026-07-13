use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub tab_active: Style,
    pub tab_inactive: Style,
    pub tab_bar: Style,
    pub diff_add: Style,
    pub diff_delete: Style,
    pub diff_context: Style,
    pub diff_header: Style,
    pub diff_add_highlight: Style,
    pub diff_delete_highlight: Style,
    pub selected_bg: Style,
    pub selected: Style,
    pub normal: Style,
    pub title: Style,
    pub border: Style,
    pub border_focused: Style,
    pub dim_text: Style,
    pub file_entry: Style,
    pub file_entry_staged: Style,
    pub file_entry_modified: Style,
    pub file_entry_untracked: Style,
    pub commit_hash: Style,
    pub commit_msg: Style,
    pub commit_author: Style,
    pub commit_date: Style,
    pub stash_msg: Style,
    pub scrollbar: Style,
    pub help_key: Style,
    pub help_desc: Style,
    pub mode_indicator: Style,
    pub status_added: Style,
    pub status_deleted: Style,
    pub status_modified: Style,
}

impl Theme {
    pub fn dark() -> Self {
        let dark_bg = Color::Rgb(30, 30, 30);
        let light_bg = Color::Rgb(40, 40, 40);
        let _highlight_bg = Color::Rgb(60, 60, 80);

        Self {
            tab_active: Style::default()
                .fg(Color::Rgb(200, 200, 200))
                .bg(Color::Rgb(60, 60, 80))
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default()
                .fg(Color::Rgb(140, 140, 140))
                .bg(dark_bg),
            tab_bar: Style::default().bg(dark_bg),

            diff_add: Style::default()
                .fg(Color::Rgb(160, 220, 160))
                .bg(Color::Rgb(30, 55, 30)),
            diff_delete: Style::default()
                .fg(Color::Rgb(220, 160, 160))
                .bg(Color::Rgb(55, 30, 30)),
            diff_context: Style::default()
                .fg(Color::Rgb(200, 200, 200))
                .bg(light_bg),
            diff_header: Style::default()
                .fg(Color::Rgb(100, 160, 220))
                .bg(dark_bg)
                .add_modifier(Modifier::BOLD),
            diff_add_highlight: Style::default()
                .fg(Color::Rgb(180, 255, 180))
                .bg(Color::Rgb(40, 80, 40)),
            diff_delete_highlight: Style::default()
                .fg(Color::Rgb(255, 180, 180))
                .bg(Color::Rgb(80, 40, 40)),

            selected_bg: Style::default().bg(Color::Rgb(70, 70, 100)),
            selected: Style::default()
                .fg(Color::Rgb(220, 220, 255))
                .bg(Color::Rgb(70, 70, 100)),

            normal: Style::default().fg(Color::Rgb(200, 200, 200)).bg(dark_bg),
            title: Style::default()
                .fg(Color::Rgb(180, 200, 255))
                .add_modifier(Modifier::BOLD),
            border: Style::default().fg(Color::Rgb(100, 100, 120)).bg(dark_bg),
            border_focused: Style::default()
                .fg(Color::Rgb(150, 180, 255))
                .bg(dark_bg),
            dim_text: Style::default()
                .fg(Color::Rgb(120, 120, 120))
                .bg(dark_bg),

            file_entry: Style::default().fg(Color::Rgb(200, 200, 200)).bg(light_bg),
            file_entry_staged: Style::default()
                .fg(Color::Rgb(160, 220, 160))
                .bg(light_bg),
            file_entry_modified: Style::default()
                .fg(Color::Rgb(220, 200, 160))
                .bg(light_bg),
            file_entry_untracked: Style::default()
                .fg(Color::Rgb(160, 160, 160))
                .bg(light_bg),

            commit_hash: Style::default()
                .fg(Color::Rgb(200, 180, 120))
                .bg(light_bg),
            commit_msg: Style::default().fg(Color::Rgb(200, 200, 200)).bg(light_bg),
            commit_author: Style::default()
                .fg(Color::Rgb(160, 180, 220))
                .bg(light_bg),
            commit_date: Style::default()
                .fg(Color::Rgb(140, 140, 140))
                .bg(light_bg),

            stash_msg: Style::default().fg(Color::Rgb(200, 180, 220)).bg(light_bg),

            scrollbar: Style::default()
                .fg(Color::Rgb(80, 80, 100))
                .bg(dark_bg),

            help_key: Style::default()
                .fg(Color::Rgb(180, 200, 255))
                .add_modifier(Modifier::BOLD),
            help_desc: Style::default().fg(Color::Rgb(160, 160, 160)),

            mode_indicator: Style::default()
                .fg(Color::Rgb(180, 220, 180))
                .bg(Color::Rgb(40, 60, 40))
                .add_modifier(Modifier::BOLD),

            status_added: Style::default()
                .fg(Color::Rgb(100, 200, 100))
                .add_modifier(Modifier::BOLD),
            status_deleted: Style::default()
                .fg(Color::Rgb(200, 100, 100))
                .add_modifier(Modifier::BOLD),
            status_modified: Style::default()
                .fg(Color::Rgb(200, 180, 100))
                .add_modifier(Modifier::BOLD),
        }
    }
}
