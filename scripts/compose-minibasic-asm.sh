#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
module_dir="$repo_root/examples/minibasic-asm/modules"
generated_source="${1:-}"

modules=(
  00_data_bootstrap.rv
  10_repl_and_dispatch.rv
  20_expression.rv
  30_arrays_and_functions.rv
  40_strings_and_tables.rv
  90_session.rv
)

for module in "${modules[@]}"; do
  [[ -f "$module_dir/$module" ]] || {
    printf 'missing MiniBASIC assembly module: %s\n' "$module" >&2
    exit 1
  }
done

if [[ -n "$generated_source" ]]; then
  : > "$generated_source"
  for module in "${modules[@]}"; do
    cat "$module_dir/$module" >> "$generated_source"
  done
else
  for module in "${modules[@]}"; do
    cat "$module_dir/$module"
  done
fi
