#!/usr/bin/env bash
# PostToolUse hook: after Claude edits or writes a Rust file, run `cargo fmt`
# so the code stays rustfmt-clean. Reads the tool call JSON on stdin.
# Plain grep instead of jq, so it works on machines without jq.

input=$(cat)
if printf '%s' "$input" | grep -Eq '"file_path" *: *"[^"]*\.rs"'; then
  cd "$CLAUDE_PROJECT_DIR" || exit 0
  if ! output=$(cargo fmt 2>&1); then
    # Exit code 2 shows the message to Claude without undoing the edit.
    echo "cargo fmt failed (likely a syntax error in the edit):" >&2
    echo "$output" >&2
    exit 2
  fi
fi
exit 0
