#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# The historical resident-Rust test expected private implementation details
# that no longer describe the target-side payload. Keep this release entry
# point, but delegate to the three observable target contracts used by the
# current tutorial and payload ABI.
bash scripts/test-guest-runtime-asm-repl-direct.sh
bash scripts/test-guest-runtime-asm-repl-hammurabi.sh
bash scripts/test-guest-minibasic-payload.sh

printf 'MiniBASIC-RV target payload suite passed\n'
