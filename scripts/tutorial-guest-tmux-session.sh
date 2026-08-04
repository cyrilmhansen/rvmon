#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

server="rvmonitor-tutorial-$$"
session="tutorial"
temporary_dir="$(mktemp -d)"
qemu_fifo="$temporary_dir/qemu-input"
guide_fifo="$temporary_dir/guide-input"
controller_log="$temporary_dir/controller.log"
mkfifo "$qemu_fifo" "$guide_fifo"
export XDG_CONFIG_HOME="$temporary_dir/empty-config"
mkdir -p "$XDG_CONFIG_HOME"
# The default /tmp/tmux-UID socket directory is not writable in some isolated
# runners; keep the private server socket beside the session FIFOs.
export TMUX_TMPDIR="$temporary_dir"
mkdir -p "$TMUX_TMPDIR/tmux-$(id -u)"
chmod 700 "$TMUX_TMPDIR/tmux-$(id -u)"
controller_pid=""
tmux_cmd=(tmux -f /dev/null -L "$server")

cleanup() {
    if [[ -n "$controller_pid" ]] && kill -0 "$controller_pid" 2>/dev/null; then
        kill "$controller_pid" 2>/dev/null || true
        wait "$controller_pid" 2>/dev/null || true
    fi
    "${tmux_cmd[@]}" kill-session -t "$session" 2>/dev/null || true
    "${tmux_cmd[@]}" kill-server 2>/dev/null || true
    rm -rf "$temporary_dir"
}
trap cleanup EXIT INT TERM

image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
pause="${TUTORIAL_GUEST_PAUSE:-1}"
timeout_seconds="${TUTORIAL_GUEST_TIMEOUT:-360}"

"${tmux_cmd[@]}" new-session -d -s "$session" -c "$repo_root" -x 160 -y 48 \
    "sleep 600"
"${tmux_cmd[@]}" set-option -t "$session" default-shell /bin/bash
"${tmux_cmd[@]}" respawn-pane -k -t "$session:0.0" -c "$repo_root" \
    "exec /usr/bin/qemu-system-riscv64 -M virt -m 64M -bios none -kernel $image -nographic < $qemu_fifo"
"${tmux_cmd[@]}" split-window -v -p 70 -t "$session:0.0" \
    "bash scripts/tutorial-guest-tmux-guide.sh < $guide_fifo"
"${tmux_cmd[@]}" swap-pane -s "$session:0.0" -t "$session:0.1"
"${tmux_cmd[@]}" select-pane -t "$session:0.0"
"${tmux_cmd[@]}" set-option -t "$session" window-size manual

TUTORIAL_GUEST_PAUSE="$pause" \
TUTORIAL_GUEST_TIMEOUT="$timeout_seconds" \
TUTORIAL_GUEST_GUIDE_FIFO="$guide_fifo" \
TUTORIAL_GUEST_QEMU_INPUT_FIFO="$qemu_fifo" \
    /usr/bin/bash /home/john/projects/luna/20260731/scripts/tutorial-guest-session.sh > /dev/null 2>"$controller_log" &
controller_pid=$!

# The outer recorder supplies a pseudo-terminal through `script`; tmux therefore
# presents both panes as one auditable terminal session to asciinema.
# Keep attach-session in the foreground so it retains the pseudo-terminal
# supplied by `script`. A watcher closes the tmux session when the controller
# has sent the final `q`; this avoids waiting for QEMU's deliberate EOF policy.
(
    while kill -0 "$controller_pid" 2>/dev/null; do
        sleep 0.1
    done
    "${tmux_cmd[@]}" kill-session -t "$session" 2>/dev/null || true
) &
watcher_pid=$!
"${tmux_cmd[@]}" attach-session -t "$session"
wait "$controller_pid" 2>/dev/null || true
wait "$watcher_pid" 2>/dev/null || true
controller_pid=""
