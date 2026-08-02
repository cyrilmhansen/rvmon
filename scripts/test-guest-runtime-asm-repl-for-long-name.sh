#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
FOR_PROGRAM='10 FOR LONGVARIABLE16=1 TO 5 STEP 2' \
FOR_EXPECTED='1.000000 3.000000 5.000000' \
bash scripts/test-guest-runtime-asm-repl-for-step.sh
