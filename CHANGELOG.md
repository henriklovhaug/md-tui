# Version 0.5.1 (unreleased)

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
