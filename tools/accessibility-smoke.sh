#!/usr/bin/env bash
set -euo pipefail

output=$(cargo run -p luna-app --quiet -- --script examples/internal-first-step.rv 2>&1)
if [[ $output == *$'\033['* ]]; then
    echo "accessibility smoke: unexpected ANSI escape sequence" >&2
    exit 1
fi
grep -q "x01=0x0000000000000001" <<<"$output"
printf '%s\n' "accessibility-smoke pipe-output=plain register-result=present"
