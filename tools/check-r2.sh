#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

manifest_commit="$(awk '
    /^\[r2\]$/ { in_r2 = 1; next }
    /^\[/ { in_r2 = 0 }
    in_r2 && /^commit = / { gsub(/"/, "", $3); print $3; exit }
' norms/manifest.toml)"
if [[ ! "$manifest_commit" =~ ^[0-9a-f]{40}$ ]]; then
    printf 'invalid full R2 commit in norms/manifest.toml: %s\n' "$manifest_commit" >&2
    exit 1
fi

sum_file=norms/r2/SHA256SUMS
sha256sum --check --quiet "$sum_file"

while read -r _ path; do
    if [[ ! -f "$path" ]]; then
        printf 'R2 manifest references missing file: %s\n' "$path" >&2
        exit 1
    fi
done < "$sum_file"

expected_paths="$(awk '{ print $2 }' "$sum_file" | sort)"
actual_paths="$(
    find norms/r2/extensions -maxdepth 1 -type f -print
    printf '%s\n' norms/r2/rv_i
)"
actual_paths="$(printf '%s\n' "$actual_paths" | sort)"
if [[ "$expected_paths" != "$actual_paths" ]]; then
    printf 'R2 source set differs from norms/r2/SHA256SUMS\n' >&2
    diff -u <(printf '%s\n' "$expected_paths") <(printf '%s\n' "$actual_paths") >&2 || true
    exit 1
fi

cargo check -p luna-isa-core >/dev/null

opcode_file="$(find target/debug/build -path '*/luna-isa-core-*/out/opcode.rs' -type f -print | sort | head -n 1)"
if [[ -z "$opcode_file" ]]; then
    printf 'generated opcode artifact was not found\n' >&2
    exit 1
fi
table_file="$(mktemp)"
trap 'rm -f "$table_file"' EXIT
awk '1 { print } /^\];$/ { exit }' "$opcode_file" > "$table_file"
expected_artifact_hash="$(awk -F'"' '/R2_OPCODE_TABLE_SHA256/ { print $2; exit }' "$opcode_file")"
actual_artifact_hash="$(sha256sum "$table_file" | awk '{ print $1 }')"
if [[ "$expected_artifact_hash" != "$actual_artifact_hash" ]]; then
    printf 'generated opcode artifact hash mismatch: expected %s, found %s\n' \
        "$expected_artifact_hash" "$actual_artifact_hash" >&2
    exit 1
fi
printf 'R2 sources, hashes, commit metadata, and generator validation: PASS\n'
