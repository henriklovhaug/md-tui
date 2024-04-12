# MD-TUI

<!--toc:start-->

- [MD-TUI](#md-tui)
  - [Installation](#installation)
  - [Usage](#usage)
  - [Key binds](#key-binds)
  - [Syntax highlighting](#syntax-highlighting)
  - [Configuration](#configuration)
  - [Links](#links)
  - [Contributions](#contributions)

<!--toc:end-->

`MD-TUI` is a TUI application for viewing markdown files directly in your
terminal. I created it because I wasn't happy with how alternatives handled
links in their applications. While the full markdown specification is not yet
supported, it will slowly get there. It's a good solution for quickly viewing
your markdown notes, or opening external links from someones README.

## Installation

Using cargo: `cargo install md-tui --locked`

Using AUR: `yay -S md-tui-bin`

## Usage

Start the program running `mdt <file.md>` or just `mdt`. The latter will search
recursively from where it was invoked for any markdown file and show it in a
_file tree_.

## Key binds

| Key        | Action                                                                 |
| ---------- | ---------------------------------------------------------------------- |
| `j`        | Scroll down                                                            |
| `k`        | Scroll up                                                              |
| `l`        | Scroll one page down                                                   |
| `h`        | Scroll one page up                                                     |
| `r`        | Reload file                                                            |
| `f` or `/` | Search                                                                 |
| `n` or `N` | Jump to next or previous search result                                 |
| `s` or `S` | Enter select link mode. Different selection strategy.                  |
| `Enter`    | Select. Depending on which mode it can: open file, select link, search |
| `Esc`      | Go back to _normal_ mode                                               |
| `t`        | Go back to files                                                       |
| `b`        | Go back to previous file (file tree if no previous file)               |
| `g`        | Go to top of file                                                      |
| `G`        | Go to bottom of the file                                               |
| `d`        | Go down half a page                                                    |
| `u`        | Go up half a page                                                      |
| `q`        | Quit the application                                                   |

## Syntax highlighting

`MD-TUI` supports syntax highlighting in code blocks for the following
languages:

- Rust
- JavaScript
- Java
- Go
- Python
- Ocaml
- Json
- Bash/sh
- C/C++
- Lua

## Configuration

The program checks for the file `~/.config/mdt/config.toml` at startup. The
following parameters and their defaults are written below. Setting color to `""`
will not remove it, but leave it as its default. To remove colors, set it to
`reset`.

```toml
# General settings
width = 80

# Inline styling
italic_color = "reset"
bold_color = "reset"
bold_italic_color = "reset"
strikethrough_color = "reset"
code_fg_color = "red"
code_bg_color = "#2A2A2A"
link_color = "blue"
link_selected_fg_color = "green"
link_selected_bg_color = "darkgrey"

# Block styling
h_bg_color = "blue"
h_fg_color = "black"
quote_bg_color = "reset"
code_block_bg_color = "#2A2A2A"
table_header_fg_color = "yellow"
table_header_bg_color = "reset"

# File tree
file_tree_selected_fg_color = "lightgreen"
file_tree_page_count_color = "lightgreen"
file_tree_name_color = "blue"
file_tree_path_color = "gray"
gitignore = false

# Quote bar
quote_important = "lightred"
quote_warning = "lightYellow"
quote_tip = "lightgreen"
quote_note = "lightblue"
quote_caution = "lightmagenta"
quote_default = "white"

# Heading
h2_fg_color = "green"
h3_fg_color = "magenta"
h4_fg_color = "cyan"
h5_fg_color = "yellow"
h6_fg_color = "lightred"
```

## Links

MD-TUI currently supports `[text](url)`, `[[link]]`, and `[[link|Some title]]`
type of links.

## Contributions

Both PRs and issues are appreciated!
