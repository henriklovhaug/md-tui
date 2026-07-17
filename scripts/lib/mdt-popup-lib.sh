# Shared helper for opening md-tui (mdt) inside a tmux popup. Sourced, not
# executed — defines a function and has no side effects on source.

# run_mdt_popup <abs_file> <dump_path> [username]
# Opens mdt on <abs_file> in a tmux popup, with MDT_DUMP_PATH=<dump_path> so the
# Sidemark dump lands in a file (the popup's stdout is the TUI and is not wired
# back to the caller). The env var is baked into the %q-quoted command string
# because the tmux server scrubs the environment. The caller is responsible for
# creating/reading/removing <dump_path> and for checking $TMUX and `mdt`.
run_mdt_popup() {
    local file=$1 dump=$2 username=${3-} cmd
    cmd=$(printf 'MDT_DUMP_PATH=%q mdt %q' "$dump" "$file")
    [[ -n $username ]] && cmd+=$(printf ' -u %q' "$username")
    tmux popup -E -w 90% -h 90% "$cmd"
}
