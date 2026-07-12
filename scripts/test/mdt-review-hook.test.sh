#!/usr/bin/env bash
# Dependency-free test runner for mdt-review-hook.sh. Sources the hook (which
# does NOT run main when sourced) and exercises its pure functions.
set -uo pipefail

HERE=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
ROOT=$(cd -- "$HERE/../.." && pwd)
# shellcheck source=../mdt-review-hook.sh
. "$ROOT/scripts/mdt-review-hook.sh"

pass=0 fail=0
ok() { printf 'ok   - %s\n' "$1"; pass=$((pass + 1)); }
no() { printf 'FAIL - %s\n' "$1"; fail=$((fail + 1)); }
# check <desc> <expected> <actual>
check() {
    if [[ "$2" == "$3" ]]; then
        ok "$1"
    else
        no "$1"
        printf '       expected: %q\n       actual:   %q\n' "$2" "$3"
    fi
}

# --- glob_match ---------------------------------------------------------
glob_fixture=$(mktemp -d -t mdt-glob.XXXXXX)
trap 'rm -rf -- "$glob_fixture"' EXIT
mkdir -p "$glob_fixture/docs/sub" "$glob_fixture/src"
touch "$glob_fixture/docs/a.md" "$glob_fixture/docs/sub/b.md" \
    "$glob_fixture/src/c.md" "$glob_fixture/docs/d.txt"

glob_match "$glob_fixture/docs/a.md" "$glob_fixture" 'docs/**/*.md'
check "glob: top-level docs .md matches" 0 $?

glob_match "$glob_fixture/docs/sub/b.md" "$glob_fixture" 'docs/**/*.md'
check "glob: nested docs .md matches (globstar)" 0 $?

glob_match "$glob_fixture/src/c.md" "$glob_fixture" 'docs/**/*.md'
check "glob: .md outside allow-list does not match" 1 $?

glob_match "$glob_fixture/docs/d.txt" "$glob_fixture" 'docs/**/*.md'
check "glob: wrong extension does not match" 1 $?

glob_match "$glob_fixture/docs/a.md" "$glob_fixture"
check "glob: no globs given does not match" 1 $?

# --- payload parsing ----------------------------------------------------
payload='{"tool_name":"Write","cwd":"/proj","tool_input":{"file_path":"/proj/docs/a.md","content":"hi"}}'

check "payload: extract_file_path" "/proj/docs/a.md" \
    "$(printf '%s' "$payload" | extract_file_path)"
check "payload: extract_cwd" "/proj" \
    "$(printf '%s' "$payload" | extract_cwd)"
check "payload: missing file_path -> empty" "" \
    "$(printf '%s' '{"tool_input":{}}' | extract_file_path)"

# --- format_reason ------------------------------------------------------
empty_dump=$(mktemp -t mdt-empty.XXXXXX)
check "format_reason: empty/absent dump -> empty" "" "$(format_reason "$empty_dump")"
check "format_reason: nonexistent dump -> empty" "" "$(format_reason /no/such/file)"
rm -f -- "$empty_dump"

one_dump=$(mktemp -t mdt-one.XXXXXX)
cat >"$one_dump" <<'YAML'
mrsf_version: "1.0"
document: "docs/a.md"
comments:
  - id: 11111111-1111-1111-1111-111111111111
    author: "me"
    timestamp: '2026-06-17T00:00:00+00:00'
    text: "tighten this sentence"
    resolved: false
    line: 3
    end_line: 3
    start_column: 0
    end_column: 5
    selected_text: "Hello world"
YAML
out=$(format_reason "$one_dump")
rm -f -- "$one_dump"
case $out in
    *"address"*"before continuing"*) ok "format_reason: header present" ;;
    *) no "format_reason: header present" ;;
esac
case $out in
    *'- L3: tighten this sentence  (on: Hello world)'*) ok "format_reason: comment line rendered + decoded" ;;
    *) no "format_reason: comment line rendered + decoded"; printf '       got: %q\n' "$out" ;;
esac

two_dump=$(mktemp -t mdt-two.XXXXXX)
cat >"$two_dump" <<'YAML'
mrsf_version: "1.0"
document: "docs/a.md"
comments:
  - id: 11111111-1111-1111-1111-111111111111
    author: "me"
    timestamp: '2026-06-17T00:00:00+00:00'
    text: "first note with \"quotes\""
    resolved: false
    line: 1
    end_line: 1
    start_column: 0
    end_column: 1
  - id: 22222222-2222-2222-2222-222222222222
    author: "me"
    timestamp: '2026-06-17T00:00:00+00:00'
    text: "second note"
    resolved: false
    line: 9
    end_line: 9
    start_column: 0
    end_column: 2
YAML
out2=$(format_reason "$two_dump")
rm -f -- "$two_dump"
case $out2 in
    *'these 2 md-tui review comment(s)'*) ok "format_reason: counts two comments" ;;
    *) no "format_reason: counts two comments"; printf '       got: %q\n' "$out2" ;;
esac
case $out2 in
    *'- L1: first note with "quotes"'*) ok "format_reason: decodes escaped quotes, no selected_text" ;;
    *) no "format_reason: decodes escaped quotes, no selected_text"; printf '       got: %q\n' "$out2" ;;
esac

multiline_dump=$(mktemp -t mdt-ml.XXXXXX)
cat >"$multiline_dump" <<'YAML'
mrsf_version: "1.0"
document: "docs/a.md"
comments:
  - id: 33333333-3333-3333-3333-333333333333
    author: "me"
    timestamp: '2026-06-17T00:00:00+00:00'
    text: "line one\nline two"
    resolved: false
    line: 4
    end_line: 4
    start_column: 0
    end_column: 1
YAML
out3=$(format_reason "$multiline_dump")
rm -f -- "$multiline_dump"
case $out3 in
    *'- L4: line one line two'*) ok "format_reason: collapses newline in body to one bullet line" ;;
    *) no "format_reason: collapses newline in body to one bullet line"; printf '       got: %q\n' "$out3" ;;
esac

# A literal TAB inside a scalar must not scramble the awk column split: the
# `line`/`text`/`sel` columns stay intact and the tab is squashed to a space.
tab_dump=$(mktemp -t mdt-tab.XXXXXX)
printf 'comments:\n  - id: x\n    text: "a\tb"\n    line: 7\n    selected_text: "c\td"\n' >"$tab_dump"
out_tab=$(format_reason "$tab_dump")
rm -f -- "$tab_dump"
case $out_tab in
    *'- L7: a b  (on: c d)'*) ok "format_reason: literal tab in field does not scramble columns" ;;
    *) no "format_reason: literal tab in field does not scramble columns"; printf '       got: %q\n' "$out_tab" ;;
esac

# --- emit_context -------------------------------------------------------
emitted=$(emit_context $'do the thing\nand the other')
check "emit_context: hookEventName is PostToolUse" "PostToolUse" \
    "$(printf '%s' "$emitted" | jq -r .hookSpecificOutput.hookEventName)"
check "emit_context: additionalContext preserves newlines" $'do the thing\nand the other' \
    "$(printf '%s' "$emitted" | jq -r .hookSpecificOutput.additionalContext)"

# Adversarial comment text (quotes, backslashes, command substitution, braces)
# must produce *valid* JSON and round-trip unchanged — never break out of the
# string or inject structure.
adv=$'he said "hi"\\ then `whoami` $(id) {"k":"v"} \n\t end'
adv_emitted=$(emit_context "$adv")
if printf '%s' "$adv_emitted" | jq -e . >/dev/null 2>&1; then
    ok "emit_context: adversarial text -> valid JSON"
else
    no "emit_context: adversarial text -> valid JSON"; printf '       got: %q\n' "$adv_emitted"
fi
check "emit_context: adversarial text round-trips unchanged" "$adv" \
    "$(printf '%s' "$adv_emitted" | jq -r .hookSpecificOutput.additionalContext)"

# --- shared popup lib ---------------------------------------------------
. "$ROOT/scripts/lib/mdt-popup-lib.sh"
if declare -F run_mdt_popup >/dev/null; then
    ok "lib: run_mdt_popup is defined after sourcing"
else
    no "lib: run_mdt_popup is defined after sourcing"
fi

# --- main fail-open gates (must exit 0 and emit nothing before popup) ---
HOOK="$ROOT/scripts/mdt-review-hook.sh"

# Non-matching path: exits 0, no stdout, regardless of $TMUX.
out=$(printf '%s' '{"cwd":"/proj","tool_input":{"file_path":"/proj/src/c.md"}}' \
    | TMUX="" bash "$HOOK" 'docs/**/*.md'; printf 'rc=%s' "$?")
check "main: non-matching path emits nothing" "rc=0" "$out"

# Matching path (real on-disk file, so it clears the glob gate) but no tmux:
# the TMUX gate must fire -> exit 0, nothing on stdout (note goes to stderr).
gate_fixture=$(mktemp -d -t mdt-gate.XXXXXX)
mkdir -p "$gate_fixture/docs"
touch "$gate_fixture/docs/a.md"
out=$(printf '%s' '{"cwd":"'"$gate_fixture"'","tool_input":{"file_path":"'"$gate_fixture"'/docs/a.md"}}' \
    | TMUX="" bash "$HOOK" 'docs/**/*.md' 2>/dev/null; printf 'rc=%s' "$?")
check "main: matching path without tmux skips (exit 0, no stdout)" "rc=0" "$out"
rm -rf -- "$gate_fixture"

# Empty payload: exits 0.
out=$(printf '%s' '{}' | TMUX="" bash "$HOOK" 'docs/**/*.md' 2>/dev/null; printf 'rc=%s' "$?")
check "main: empty payload exits 0" "rc=0" "$out"

# --- main happy path (stubbed tmux + mdt) -------------------------------
# The only path that reaches the popup, the EXIT-trap cleanup, and the JSON
# emission. Stub `mdt` (writes a Sidemark dump) and `tmux` (runs the popup
# command directly) on PATH so the full chain runs without a human/tmux.
e2e=$(mktemp -d -t mdt-e2e.XXXXXX)
mkdir -p "$e2e/proj/docs" "$e2e/bin"
echo "# hi" >"$e2e/proj/docs/a.md"
cat >"$e2e/bin/mdt" <<'STUB'
#!/usr/bin/env bash
cat >"$MDT_DUMP_PATH" <<'YAML'
mrsf_version: "1.0"
document: "docs/a.md"
comments:
  - id: 11111111-1111-1111-1111-111111111111
    author: "me"
    timestamp: '2026-06-17T00:00:00+00:00'
    text: "expand this"
    resolved: false
    line: 1
    end_line: 1
    start_column: 0
    end_column: 4
    selected_text: "hi"
YAML
STUB
printf '#!/usr/bin/env bash\neval "${@: -1}"\n' >"$e2e/bin/tmux"
chmod +x "$e2e/bin/mdt" "$e2e/bin/tmux"

e2e_payload="{\"cwd\":\"$e2e/proj\",\"tool_input\":{\"file_path\":\"$e2e/proj/docs/a.md\"}}"
e2e_out=$(printf '%s' "$e2e_payload" \
    | PATH="$e2e/bin:$PATH" TMUX="fake" bash "$HOOK" 'docs/**/*.md' 2>"$e2e/err")
e2e_rc=$?
check "main happy path: exit 0" 0 "$e2e_rc"
check "main happy path: no stderr noise" "" "$(cat "$e2e/err")"
check "main happy path: emits PostToolUse additionalContext" "PostToolUse" \
    "$(printf '%s' "$e2e_out" | jq -r .hookSpecificOutput.hookEventName)"
case $(printf '%s' "$e2e_out" | jq -r .hookSpecificOutput.additionalContext) in
    *'- L1: expand this  (on: hi)'*) ok "main happy path: comment rendered in context" ;;
    *) no "main happy path: comment rendered in context"; printf '       out: %q\n' "$e2e_out" ;;
esac
rm -rf -- "$e2e"

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[[ $fail -eq 0 ]]
