#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
bash scripts/build-minibasic-asm-payload.sh >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
code=target/payloads/minibasic-payload-asm.bin
data=target/payloads/minibasic-payload-asm-data.bin

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

wait_for_text() {
    local text="$1"
    for _ in {1..2000}; do
        if grep -aFq -- "$text" "$output_file"; then return 0; fi
        sleep 0.01
    done
    cat "$output_file" >&2
    printf 'timeout waiting for guest text: %s\n' "$text" >&2
    return 1
}

load_region() {
    local command="$1" address="$2" file="$3"
    while IFS= read -r bytes; do
        printf '%s 0x%08x %s\n' "$command" "$address" "$bytes" >&3
        sleep 0.002
        address=$((address + ${#bytes} / 2))
    done < <(xxd -p -c32 "$file")
}

sleep 0.2
load_region payload-load 0x81000100 "$code"
load_region payload-load-data 0x82000000 "$data"
printf 'run-at 0x81000100\n' >&3
wait_for_text 'READY> '
printf '%s\n' \
  '10 FOR I=1 TO 2' \
  '20 PRINT I' \
  '30 NEXT I' \
  '40 END' \
  'TRACE ON' \
  'RUN' >&3
wait_for_text 'trap: breakpoint'
printf 'q\n' >&3
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in '[10]' '[20]' '[30]' '[40]' '1.000000' '2.000000'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing dynamic TRACE output: %s\n' "$expected" >&2
        exit 1
    fi
done

trace30_count="$(grep -ao '\[30\]' "$output_file" | wc -l)"
if (( trace30_count < 2 )); then
    cat "$output_file"
    printf 'expected NEXT loop to trace line 30 at least twice, got %s\n' "$trace30_count" >&2
    exit 1
fi

printf 'guest assembly dynamic TRACE QEMU test passed\n'
