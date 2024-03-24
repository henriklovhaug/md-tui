# MD-TUI

<!--toc:start-->

- [MD-TUI](#md-tui)
  - [Installation](#installation)
  - [Key binds](#key-binds)
  - [Configuration](#configuration)
  - [Contributions](#contributions)
  - [Versioning](#versioning)

<!--toc:end-->

## Installation

Using cargo: `cargo install md-tui --locked`

Using AUR: `yay -S md-tui-bin`

## Key binds

| Key        | Action                                                                 |
| ---------- | ---------------------------------------------------------------------- |
| `j`        | Scroll down                                                            |
| `k`        | Scroll up                                                              |
| `l`        | Scroll one page down                                                   |
| `h`        | Scroll one page up                                                     |
| `r`        | Reload file                                                            |
| `f` or `/` | Search                                                                 |
| `s`        | Enter select link mode                                                 |
| `Enter`    | Select. Depending on which mode it can: open file, select link, search |
| `Esc`      | Go back to _normal_ mode                                               |
| `t`        | Go back to files                                                       |
| `b`        | Go back to previous file (file tree if no previous file)               |
| `g`        | Go to top of file                                                      |
| `G`        | Go to bottom of the file                                               |
| `d`        | Go down half a page                                                    |
| `u`        | Go up half a page                                                      |
| `q`        | Quit the application                                                   |

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
quote_bg_color = "#2A2A2A"
code_block_fg_color = "red" #Will change when tree-sitter gets implemented
code_block_bg_color = "#2A2A2A"
table_header_fg_color = "yellow"
table_header_bg_color = "reset"

# File tree
file_tree_selected_fg_color = "lightgreen"
file_tree_page_count_color = "lightgreen"
file_tree_name_color = "blue"
file_tree_path_color = "gray"
gitignore = false
```

## Contributions

Both PRs and issues are appreciated!

## Versioning

Until 1.0.0 release, every minor increase adds new features. Patches fixes
something that the program should support, but doesn't yet for some reason.
