#!/bin/bash

# Shield Dev Environment
# Runs the service (cargo run) and web app (dx serve) side by side in a tmux
# session. The session only exists to host these two interactive panes, so it
# is destroyed when you detach or the script exits.

set -e

SESSION="shield-dev"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v tmux &>/dev/null; then
    echo "ERROR: tmux not found. Install it with: brew install tmux" >&2
    exit 1
fi

cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Replace any stale session left over from a previous run
tmux kill-session -t "$SESSION" 2>/dev/null || true

tmux new-session -d -s "$SESSION" -c "$REPO_ROOT" 'cargo run -p shield-service'

# Keep dead panes around so build/startup failures stay readable instead of
# the pane closing with the output
tmux set-option -w -t "$SESSION" remain-on-exit on

tmux split-window -h -t "$SESSION" -c "$REPO_ROOT" 'dx serve -p shield-app'
tmux select-layout -t "$SESSION" even-horizontal

if [[ -n "$TMUX" ]]; then
    # Already inside tmux: attach a nested client. tmux refuses to nest unless
    # $TMUX is cleared. Send the prefix twice to control the inner session,
    # e.g. C-b C-b d to detach it.
    TMUX='' tmux attach-session -t "$SESSION"
else
    tmux attach-session -t "$SESSION"
fi
