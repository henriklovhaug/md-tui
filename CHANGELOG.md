# Version 0.7.4

- (#104) Files can now be opened in `$EDITOR` from `MDT`.
  - Tested with Neovim and Vim.
- (#110) Add a Nix flake.
- (#105 partially) More flexible table dash parsing.
- Add highlighting for css, html, php, typescript.
- (#105) Length is defined by longest cell in column.
  - Cell wrapping is not implemented yet.
- Inline code can be written with triple ticks, but it checks if it's a
  codeblock first.

# Version 0.7.3

- Fix quote blocks word wrapping.
- Add the arrow keys as aliases for HJKL (#103, by @cmrschwarz)
- Add Scala syntax highlighting (#117, by @sierikov)
- (#106) `MDT` now allows text part of link to cross multiple lines.
- Allow arbitrary programming language in codeblock. Does not mean it will get
  highlighted.

# Version 0.7.2

- Images which was below viewport would crash the app when reaching it.
- Improve parsing for quotes. It allows them to be indented. As with indented
  paragraphs, this TUI does not respect indenting in for blocks other than
  lists.
- Tabs are now transformed to 4 spaces in codeblocks. May be reversed when
  proper rendering of tabs is configured.

# Version 0.7.1

- Changes default width to 100
- Add possibility to change config using environment variables
- Fix crash when terminal does not respond with font size

# Version 0.7.0

## Images!!

It now supports images!! Mostly thanks to the
[ratatui-image crate](https://crates.io/crates/ratatui-image). For terminals
with a nerd font and no image support it will fall back to Unicode half blocks.
This means `MD-TUI` is a poor fit for `TTY`s. (#15).

## Fixes

- Code block without specifying language now has leading empty line.

# Version 0.6.3

- Improve heading search.
- (#67) numbering of ordered list is now handled automatically.

# Version 0.6.2

## Fixes

- (#92) Fix page counter which had the total number of pages wrong sometimes.
- (#49) Headings are visually separate-able, and configurable.
- (#96) The scroll is not affected on file change. It also now detects file
  changes.

# Version 0.6.0

## NEW FEATURE

**SYNTAX HIGHLIGHTING!**

Currently adds support for (with more to come):

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

## Fixes

- Crash when number of markdown files where exactly at page limit at user tried
  to scroll to the last one using `j` or `k`.
- Improved resizing. Can still crash if the terminal changes too fast. (#69)
- Improve heading parsing.
- Paragraphs could clip the end and last to chars of words could be clipped.

# Version 0.5.2

## Fixes

- Fix parsing of word modifiers after last version broke them.

# Version 0.5.1

## Breaking change

- `s` key now tries to find the top link in view. It searches downward, but
  selects the last one link if it doesn't find any.

## Added key

- The `S` key does the same as `s` but searches from two thirds up and does the
  search both ways.

## Fixes

- Every block now has a newline between them regardless of users formatting.
- Fixed tasks which was parsed incorrectly.
- Changed how spaces are parsed in lists so search markings can become nicer.
- (#83) Search markings vastly improved. Both in performance and multiple word
  capturing. It is however slightly more strict. The search `something` will not
  match end of sentence like `something.` or `something,`.
- (#84) Changed parsing grammar to allow italic/bold/strike through words to
  wrap newline in quote blocks.
- Other minor changes to parsing.

# Version 0.5.0

## Fixes

- #64 Tables and code blocks destroyed search marking for every block type
  following.
- Gave code blocks a bit more space.
- #68 Improve link parsing. Root counts from where `mdt` was invoked.
- #72 If paragraphs started with italic modifier, it didn't work. Now fixed.
- Bold&italic had an issue of consuming its preceding whitespace.
- #78 Fixed wrong indentation of lists.

## New features

- #33 Able to jump the next or previous search result.
- #65 Quote markings supported.
- #75 Wiki links. `[[linkToSomething]]` or `[[URL|Some title]]` is supported.

# Version 0.4.2

## New features

- Added new option `gitignore` which, when set to true (default false), does not
  load files from `.gitignore`.
- When in link mode, you can press `h` to hover/see where the link goes (#35)

## New behavior

- It continues to find markdown files even if it meets a directory it's not
  allowed to read (#51)

## Fixes

- Support multi-line comments (#56)
- Paragraphs after table don't look weird anymore (#48)

# Version 0.4.1

## Fixes

- #52 Code no longer does an unnecessary division
- #53 Programming language in code block is now optional
- #55 No longer crashes when a block of something is higher than then terminal
  height and clips both the top and bottom

# Version 0.4.0

## New features

- Adds configuration for custom coloring and width (#4)
- Supports bold and italic at the same time (#47)

## Fixes

- Stricter italic checking (#45)
- Allowing escaping some characters (#28)
- Rewrote table parsing (slight regression. Noted in #48)
- Allows newlines in code blocks

# Version 0.3.2

## Fixes

- #39 does not panic at horizontal separators
- #36 More aggressive check for comments
- #40 Checks file endings on relative files
