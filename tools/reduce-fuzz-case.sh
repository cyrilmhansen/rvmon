#!/usr/bin/env bash
set -euo pipefail

if (( $# < 4 )); then
    echo "usage: $0 INPUT OUTPUT CHECKER CHECKER_ARG..." >&2
    echo "checker must exit 0 when the case still reproduces" >&2
    exit 2
fi

input=$1
output=$2
shift 2

if [[ ! -f $input ]]; then
    echo "input does not exist: $input" >&2
    exit 2
fi

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT
current=$workdir/current
candidate=$workdir/candidate
cp -- "$input" "$current"

if ! "$@" "$current"; then
    echo "checker does not reproduce the input" >&2
    exit 1
fi

mapfile -t lines < "$current"
changed=1
while (( changed )); do
    changed=0
    index=0
    while (( index < ${#lines[@]} )); do
        : > "$candidate"
        for (( line=0; line < ${#lines[@]}; line++ )); do
            if (( line != index )); then
                printf '%s\n' "${lines[line]}" >> "$candidate"
            fi
        done
        if "$@" "$candidate" >/dev/null 2>&1; then
            mapfile -t lines < "$candidate"
            changed=1
        else
            ((index += 1))
        fi
    done
done

: > "$output"
for line in "${lines[@]}"; do
    printf '%s\n' "$line" >> "$output"
done
echo "reduced $(wc -l < "$input") line(s) to $(wc -l < "$output") line(s): $output"
