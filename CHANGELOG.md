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
