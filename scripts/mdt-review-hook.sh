#!/usr/bin/env bash
# Claude Code PostToolUse hook (matcher: Write). On an allow-listed markdown
# write, open mdt's comment mode in a tmux popup and return any comments as a
# blocking revision request. Fails OPEN: any problem -> exit 0, write proceeds.
#
# Usage (from settings.json hook command):
#   mdt-review-hook.sh '<glob>' ['<glob>' ...]
# Globs are evaluated relative to the project root (cwd from the hook payload).
set -uo pipefail

# glob_match <file> <project_root> <glob>...
# Returns 0 if <file> is in any glob's filesystem expansion under root, else 1.
glob_match() {
    local file=$1 root=${2%/}
    shift 2
    local g f rc=1
    # Save the prior globstar/nullglob state and restore it exactly, so sourcing
    # this function never silently flips a caller's shell options off.
    local prior_opts
    prior_opts=$(shopt -p globstar nullglob)
    shopt -s globstar nullglob
    for g in "$@"; do
        for f in "$root"/$g; do
            if [[ $f == "$file" ]]; then
                rc=0
                break 2
            fi
        done
    done
    eval "$prior_opts"
    return $rc
}

# Read the hook JSON payload from stdin and echo the written file path.
extract_file_path() {
    jq -r '.tool_input.file_path // empty'
}

# Read the hook JSON payload from stdin and echo the project root (cwd).
extract_cwd() {
    jq -r '.cwd // empty'
}

# Decode a Sidemark double-quoted scalar (e.g. "a \"b\"") to plain text.
# The scalars use JSON-compatible escapes for the common cases; jq decodes
# them. On the rare \xNN control escape jq errors -> fall back to the raw
# token so we never lose the comment.
decode_scalar() {
    local tok=$1 dec
    dec=$(printf '%s' "$tok" | jq -r . 2>/dev/null) && {
        printf '%s' "$dec"
        return 0
    }
    printf '%s' "$tok"
}

# format_reason <dump_file>
# Print a blocking revision request, or nothing if there are no comments.
format_reason() {
    local dump=$1
    [[ -s $dump ]] || return 0

    local rows
    # Fields are joined with a literal TAB, so any TAB inside a scalar would
    # scramble the columns — squash them to spaces on capture. (Sidemark emits
    # tabs as the `\t` escape inside double-quoted scalars, so this is belt-and-
    # braces against a stray literal tab.)
    rows=$(awk '
        /^  - id:/             { if (have) print line "\t" text "\t" sel; have=1; line=""; text=""; sel="" }
        /^    line: /          { line=$2 }
        /^    text: /          { text=substr($0, index($0, ": ") + 2); gsub(/\t/, " ", text) }
        /^    selected_text: / { sel=substr($0, index($0, ": ") + 2); gsub(/\t/, " ", sel) }
        END                    { if (have) print line "\t" text "\t" sel }
    ' "$dump")
    [[ -n $rows ]] || return 0

    local count
    count=$(printf '%s\n' "$rows" | grep -c .)
    printf 'You MUST address these %d md-tui review comment(s) before continuing:\n\n' "$count"

    local line text sel t s
    while IFS=$'\t' read -r line text sel; do
        # Collapse newlines in a decoded body so each comment stays on one
        # bullet line (a multi-line comment otherwise breaks the layout).
        t=$(decode_scalar "$text"); t=${t//$'\n'/ }
        if [[ -n $sel ]]; then
            s=$(decode_scalar "$sel"); s=${s//$'\n'/ }
            printf -- '- L%s: %s  (on: %s)\n' "$line" "$t" "$s"
        else
            printf -- '- L%s: %s\n' "$line" "$t"
        fi
    done <<<"$rows"
}

# emit_context <text>
# Print the PostToolUse hook JSON that injects <text> into Claude's context.
# Claude Code's docs recommend additionalContext over decision:block for
# PostToolUse (the tool already ran); the imperative wording in <text> supplies
# the "must address" force.
emit_context() {
    jq -n --arg c "$1" \
        '{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $c}}'
}

main() {
    # jq is required to parse the payload; without it, fail open.
    command -v jq >/dev/null 2>&1 || exit 0

    local payload file cwd
    # Read the whole hook payload; we only need file_path + cwd (the `content`
    # field is intentionally discarded — we review the file on disk, not stdin).
    payload=$(cat)
    file=$(printf '%s' "$payload" | extract_file_path)
    cwd=$(printf '%s' "$payload" | extract_cwd)
    [[ -n $file ]] || exit 0
    cwd=${cwd:-$PWD}
    [[ $file == /* ]] || file=$cwd/$file

    # Glob gate: only allow-listed paths get reviewed.
    glob_match "$file" "$cwd" "$@" || exit 0

    # Environment gate: review needs a live tmux and the mdt binary.
    if [[ -z ${TMUX-} ]]; then
        printf 'mdt-review: not inside tmux, skipping review of %s\n' "$file" >&2
        exit 0
    fi
    if ! command -v mdt >/dev/null 2>&1; then
        printf 'mdt-review: mdt not on PATH, skipping review of %s\n' "$file" >&2
        exit 0
    fi

    # shellcheck source=lib/mdt-popup-lib.sh
    . "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/lib/mdt-popup-lib.sh"

    # `dump` is intentionally NOT local: the EXIT trap below fires after main
    # returns, when a local would already be out of scope (and tripping `set
    # -u`). The `${dump:-}` guard keeps the trap safe even if it never got set.
    dump=$(mktemp -t mdt-review.XXXXXX) || exit 0
    trap 'rm -f -- "${dump:-}"' EXIT
    run_mdt_popup "$file" "$dump" "${MDT_REVIEW_AUTHOR:-${USER-}}" || exit 0

    local reason
    reason=$(format_reason "$dump")
    [[ -n $reason ]] || exit 0
    emit_context "$reason" || exit 0
}

# Only run main when executed directly, not when sourced by the test runner.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
