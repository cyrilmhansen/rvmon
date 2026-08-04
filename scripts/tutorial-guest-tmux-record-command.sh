#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "$(dirname -- "${BASH_SOURCE[0]}")")"
stty rows 48 cols 160 2>/dev/null || true
exec script -qefc "bash scripts/tutorial-guest-tmux-session.sh" /dev/null
