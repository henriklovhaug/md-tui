# MD-TUI

<!--toc:start-->

- [MD-TUI](#md-tui)
  - [Installation](#installation)
  - [Key binds](#key-binds)
  - [Contributions](#contributions)

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

To change the config create a file called `config.toml` at the path
`~/.config/mdt/`. The following values are possible to set and their defaults.

```toml
# Size
width = 80

# Inline styling
italic_color = ""
bold_color = ""
strikethrough_color = ""
code_fg_color = "red"
code_bg_color = "#2A2A2A"

# Block configs
h1_bg_color = "blue"
quote_bg_color = "#2A2A2A"
code_block_fg_color = "red" #Will change when tree-sitter gets implemted
code_block_bg_color = "#2A2A2A"
```

## Contributions

Both PRs and issues are appreciated!
