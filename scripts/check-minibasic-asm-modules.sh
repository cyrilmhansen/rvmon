#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
temporary_dir="$(mktemp -d)"
trap 'rm -rf "$temporary_dir"' EXIT

bash "$repo_root/scripts/compose-minibasic-asm.sh" \
  "$temporary_dir/payload-repl.rv"
cmp -s "$temporary_dir/payload-repl.rv" \
  "$repo_root/examples/minibasic-asm/payload-repl.rv" || {
  printf 'MiniBASIC assembly modules do not match payload-repl.rv\n' >&2
  diff -u "$repo_root/examples/minibasic-asm/payload-repl.rv" \
    "$temporary_dir/payload-repl.rv" | head -80 >&2 || true
  exit 1
}
printf 'MiniBASIC assembly modules are synchronized\n'
