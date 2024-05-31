# MD-TUI

<!--toc:start-->

- [MD-TUI](#md-tui)
  - [Installation](#installation)
    - [Requirements](#requirements)
  - [Usage](#usage)
  - [Key binds](#key-binds)
  - [Syntax highlighting](#syntax-highlighting)
  - [Configuration](#configuration)
    - [Keyboard actions](#keyboard-actions)
    - [Colors and misc](#colors-and-misc)
  - [Links](#links)
  - [Neovim plugin](#neovim-plugin)
  - [Contributions](#contributions)

<!--toc:end-->

`MD-TUI` is a TUI application for viewing markdown files directly in your
terminal. I created it because I wasn't happy with how alternatives handled
links in their applications. While the full markdown specification is not yet
supported, it will slowly get there. It's a good solution for quickly viewing
your markdown notes, or opening external links from someones README. If your
terminal support images, they will render.

## Installation

Using cargo: `cargo install md-tui --locked`

On Arch Linux: `pacman -S md-tui`

For Nix users, there's also Nix flake.

Prebuilt binaries with install script can be found on the release page.

### Requirements

1. A terminal
2. Nerd font

## Usage

Start the program running `mdt <file.md>` or just `mdt`. The latter will search
recursively from where it was invoked for any markdown file and show it in a
_file tree_.

## Key binds

These are the default settings. See [keyboard configuration](#keyboard-actions)
for configuration options.

| Key              | Action                                                                 |
| ---------------- | ---------------------------------------------------------------------- |
| `j` or `<Down>`  | Scroll down                                                            |
| `k` or `<Up>`    | Scroll up                                                              |
| `h`              | Go down half a page                                                    |
| `l`              | Go up half a page                                                      |
| `d` or `<Left>`  | Scroll one page down                                                   |
| `u` or `<Right>` | Scroll one page up                                                     |
| `f` or `/`       | Search                                                                 |
| `n` or `N`       | Jump to next or previous search result                                 |
| `s` or `S`       | Enter select link mode. Different selection strategy.                  |
| `K`              | Hover. Preview where a link is going without going there               |
| `<Enter>`        | Select. Depending on which mode it can: open file, select link, search |
| `Esc`            | Go back to _normal_ mode                                               |
| `t`              | Go back to files                                                       |
| `b`              | Go back to previous file (file tree if no previous file)               |
| `g`              | Go to top of file                                                      |
| `G`              | Go to bottom of the file                                               |
| `e`              | Edit file in `$EDITOR`                                                 |
| `q`              | Quit the application                                                   |

## Syntax highlighting

`MD-TUI` supports syntax highlighting in code blocks for the following
languages:

- Bash/sh
- C/C++
- Css
- Elixir
- Go
- Html
- Java
- JavaScript
- Json
- Lua
- Ocaml
- PHP
- Python
- Rust
- Scala
- Typescript

## Configuration

The program checks for the file `~/.config/mdt/config.toml` at startup. The
following parameters and their defaults are written below.

### Keyboard actions

Some key actions are not configurable. Like the following:

- Enter
- Arrow keys
- Escape
- Question mark for help menu
- 'q' to quit the application
- '/' for search

> If you override another default key, it's undefined behavior if that key does
> not get reassigned.

> Actions can only be assigned to single characters. Space, fn keys, ctrl+key,
> backspace etc., will not take affect and the default will be in use.

```toml
# Keyboard actions
up = 'k'
down = 'j'
page_up = 'u'
page_down = 'd'
half_page_down = 'l'
half_page_up = 'h'
top = 'g'
bottom = 'G'
search = 'f'
search_next = 'n'
search_previous = 'N'
# This will search downwards until it finds one or select the last link in document.
select_link = 's'
# Finds the link 2/3 up the page. It will search then for closest in both direction.
select_link_alt = 'S'
edit = 'e'
hover = 'K'
back = 'b'
file_tree = 't'
```

### Colors and misc

Setting color to `""` will not remove it, but leave it as its default. To remove
colors, set it to `reset`.

```toml
# General settings
width = 100

# Inline styling
bold_color = "reset"
bold_italic_color = "reset"
code_bg_color = "#2A2A2A"
code_fg_color = "red"
italic_color = "reset"
link_color = "blue"
link_selected_bg_color = "darkgrey"
link_selected_fg_color = "green"
strikethrough_color = "reset"

# Block styling
code_block_bg_color = "#2A2A2A"
quote_bg_color = "reset"
table_header_bg_color = "reset"
table_header_fg_color = "yellow"

# File tree
file_tree_name_color = "blue"
file_tree_page_count_color = "lightgreen"
file_tree_path_color = "gray"
file_tree_selected_fg_color = "lightgreen"
gitignore = false

# Quote bar
quote_caution = "lightmagenta"
quote_default = "white"
quote_important = "lightred"
quote_note = "lightblue"
quote_tip = "lightgreen"
quote_warning = "lightYellow"

# Heading
h_bg_color = "blue"
h_fg_color = "black"
h2_fg_color = "green"
h3_fg_color = "magenta"
h4_fg_color = "cyan"
h5_fg_color = "yellow"
h6_fg_color = "lightred"
```

## Links

MD-TUI currently supports `[text](url)`, `[[link]]`, and `[[link|Some title]]`
type of links.

## Neovim plugin

This application also exists as a plugin for Neovim called
[Preview](https://github.com/henriklovhaug/Preview.nvim).

> [!NOTE]
>
> This version does not support images regardless of your terminal capabilities.

## Contributions

Both PRs and issues are appreciated!
