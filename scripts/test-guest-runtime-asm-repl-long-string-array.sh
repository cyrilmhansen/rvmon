#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor

temporary_dir="$(mktemp -d)"
input_fifo="$temporary_dir/input"
output_file="$temporary_dir/output"
mkfifo "$input_fifo"
qemu_pid=""
cleanup() {
    if [[ -n "$qemu_pid" ]] && kill -0 "$qemu_pid" 2>/dev/null; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
    fi
    rm -rf "$temporary_dir"
}
trap cleanup EXIT

qemu-system-riscv64 -M virt -m 64M -bios none -kernel "$image" \
    -nographic <"$input_fifo" >"$output_file" 2>&1 &
qemu_pid=$!
exec 3>"$input_fifo"
sleep 0.1
wait_for_text() {
    local text="$1"
    for _ in {1..500}; do
        if grep -aFq -- "$text" "$output_file"; then
            return 0
        fi
        sleep 0.01
    done
    cat "$output_file" >&2
    printf 'timeout waiting for guest text: %s\n' "$text" >&2
    return 1
}
source_prompt_count() { grep -ao 'source> ' "$output_file" | wc -l; }
monitor_prompt_count() { grep -ao 'rvmonitor> ' "$output_file" | wc -l; }
payload_stream() {
    awk '/^symbols$/{print; found=1; next} found && /^run-at /{print; exit} !found{print}' \
        examples/minibasic-asm/payload-repl.rv
}
in_source_mode=0
while IFS= read -r line; do
    if (( in_source_mode == 0 )); then
        before=$(monitor_prompt_count)
        printf '%s\n' "$line" >&3
        if [[ "$line" == "assemble-program "* ]]; then
            wait_for_text 'source mode:'
            in_source_mode=1
        else
            for _ in {1..500}; do
                (( $(monitor_prompt_count) > before )) && break
                sleep 0.01
            done
        fi
    elif [[ "$line" == end ]]; then
        printf '%s\n' "$line" >&3
        wait_for_text 'assembled program:'
        in_source_mode=2
    elif (( in_source_mode == 1 )); then
        before=$(source_prompt_count)
        printf '%s\n' "$line" >&3
        for _ in {1..500}; do
            (( $(source_prompt_count) > before )) && break
            sleep 0.01
        done
    else
        before=$(monitor_prompt_count)
        printf '%s\n' "$line" >&3
        for _ in {1..500}; do
            (( $(monitor_prompt_count) > before )) && break
            sleep 0.01
        done
    fi
done < <(payload_stream)
printf 'DIM LONGARRAY$(2)\nI=1\nLONGARRAY$(1)="ARRAY-DIRECT-TARGET"\n10 PRINT LONGARRAY$(I+0)\n20 LET LONGARRAY$(1)="ARRAY-PROGRAM-TARGET"\n30 PRINT "VALUE",LONGARRAY$(1)\n40 END\nRUN\n' >&3
sleep 1.0
printf 'q\n' >&3
exec 3>&-
sleep 0.3
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in 'ARRAY-DIRECT-TARGET' 'ARRAY-PROGRAM-TARGET' 'VALUE' 'trap: breakpoint'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing long string-array output: %s\n' "$expected" >&2
        exit 1
    fi
done
if grep -aFq -- 'error: unknown command' "$output_file"; then
    cat "$output_file"
    printf 'unexpected monitor command error during long string-array run\n' >&2
    exit 1
fi
printf 'guest assembly REPL long-string-array QEMU test passed\n'
