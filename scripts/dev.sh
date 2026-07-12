#!/bin/bash

# Shield Dev Environment
# Runs the service (cargo run) and web app (dx serve) side by side in a tmux
# session, with the stylance watcher (scoped component CSS bundler) in a small
# pane under the web app. The session only exists to host these interactive
# panes, so it is destroyed when you detach or the script exits.

set -e

SESSION="shield-dev"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v tmux &>/dev/null; then
    echo "ERROR: tmux not found. Install it with: brew install tmux" >&2
    exit 1
fi

if ! command -v stylance &>/dev/null; then
    echo "ERROR: stylance not found. Install it with: cargo install stylance-cli --locked" >&2
    exit 1
fi

# Bundle once before dx starts compiling in its pane: `asset!()` fails the
# build if app/assets/styles.css doesn't exist yet, and the watcher below
# would race that first compile.
(cd "$REPO_ROOT" && stylance app)

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

# Stylance watcher in a short pane under the web app (its output is low-volume:
# one line per rebundle). Split after the layout call so it doesn't get
# rearranged into a third column.
tmux split-window -v -l 20% -t "$SESSION" -c "$REPO_ROOT" 'stylance --watch app'

if [[ -n "$TMUX" ]]; then
    # Already inside tmux: attach a nested client. tmux refuses to nest unless
    # $TMUX is cleared. Send the prefix twice to control the inner session,
    # e.g. C-b C-b d to detach it.
    TMUX='' tmux attach-session -t "$SESSION"
else
    tmux attach-session -t "$SESSION"
fi
