# gitui-view

[![crates.io](https://img.shields.io/crates/v/gitui-view.svg)](https://crates.io/crates/gitui-view)
[![docs.rs](https://img.shields.io/docsrs/gitui-view)](https://docs.rs/gitui-view)
[![License](https://img.shields.io/crates/l/gitui-view)](LICENSE)

A terminal-based Git repository browser with inline and side-by-side diff views, built with [ratatui](https://github.com/ratatui/ratatui).

![demo](https://raw.githubusercontent.com/lyj-514328/gitui-view/master/demo.gif)

gitui-view is a lite version of [gitui](https://github.com/gitui-org/gitui), drawing inspiration from [delta](https://github.com/dandavison/delta) for diff rendering. It focuses on the core Git browsing experience — status, log, stashes, and diff review — with a clean and minimal interface.

## Features

- **Status Tab** — View staged and unstaged file changes
- **Log Tab** — Browse commit history with author, timestamp, and summary
- **Stashes Tab** — List and inspect stashed changes
- **Diff View** — Review diffs in inline or side-by-side mode, with syntax highlighting
- **Auto Theme Detection** — Automatically picks light or dark theme based on your terminal color scheme
- **Custom Themes** — Load custom themes via RON files
- **Keyboard-driven** — Vim-style navigation with `j`/`k`, tab switching, and quick-jump keys

## Keybindings

| Key | Action |
|---|---|
| `q` | Quit |
| `h` | Toggle help |
| `d` | Toggle diff view |
| `m` | Toggle inline / side-by-side diff mode |
| `Tab` / `←` `→` | Switch tabs |
| `1` `2` `3` | Go to Status / Log / Stashes tab |
| `↑` `↓` / `k` `j` | Navigate list / scroll diff |
| `Enter` | Show file list for commit/stash, or show diff for file |
| `Esc` | Go back |

## Usage

```bash
# Open the repository in the current directory
gitui-view

# Open a specific repository
gitui-view /path/to/repo

# Use a custom theme file
gitui-view --theme my_theme.ron
```

## Installation

### From crates.io

```bash
cargo install gitui-view
```

### From source

```bash
git clone https://github.com/lyj-514328/gitui-view.git
cd gitui-view
cargo install --path .
```

## Configuration

Themes can be customized via RON files. See the built-in light and dark themes in [src/theme.rs](src/theme.rs) for available fields.

```bash
gitui-view --theme ~/.config/gitui-view/theme.ron
```

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) — Terminal UI framework
- [git2](https://github.com/rust-lang/git2-rs) — libgit2 bindings
- [crossterm](https://github.com/crossterm-rs/crossterm) — Terminal manipulation
- [syntect](https://github.com/trishume/syntect) — Syntax highlighting
- [chrono](https://github.com/chronotope/chrono) — Date/time formatting

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.