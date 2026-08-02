#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf >/dev/null
scripts/build-minibasic-asm-payload.sh >/dev/null
image=target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor
binary=target/payloads/minibasic-payload-asm.bin
data_binary=target/payloads/minibasic-payload-asm-data.bin

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

monitor_prompt_count() { grep -ao 'rvmonitor> ' "$output_file" | wc -l; }
wait_for_text() {
    local text="$1"
    for _ in {1..500}; do
        if grep -aFq -- "$text" "$output_file"; then return 0; fi
        sleep 0.01
    done
    cat "$output_file" >&2
    printf 'timeout waiting for guest text: %s\n' "$text" >&2
    return 1
}

sleep 0.2
address=$((16#81000100))
load_region() {
    local command="$1"
    local address="$2"
    local file="$3"
    while IFS= read -r bytes; do
        before=$(monitor_prompt_count)
        printf '%s 0x%08x %s\n' "$command" "$address" "$bytes" >&3
        for _ in {1..500}; do
            if (( $(monitor_prompt_count) > before )); then break; fi
            sleep 0.01
        done
        address=$((address + ${#bytes} / 2))
    done < <(xxd -p -c32 "$file")
}
load_region payload-load $((16#81000100)) "$binary"
load_region payload-load-data $((16#82000000)) "$data_binary"

printf 'run-at 0x81000100\n' >&3
wait_for_text 'READY>'
printf 'PRINT 2+3*4\n' >&3
wait_for_text '14.000000'
wait_for_text 'trap: breakpoint'
printf 'run-at 0x81000100\n' >&3
sleep 0.2
printf 'PRINT 22 / 7\n' >&3
wait_for_text '3.142857'
wait_for_text 'trap: breakpoint'
exec 3>&-
sleep 0.2
kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=""

for expected in '14.000000' '3.142857'; do
    if ! grep -aFq -- "$expected" "$output_file"; then
        cat "$output_file"
        printf 'missing binary MiniBASIC output: %s\n' "$expected" >&2
        exit 1
    fi
done
if grep -aFq -- 'mcause=0x0000000000000005' "$output_file" || \
   grep -aFq -- 'error: unknown command' "$output_file"; then
    cat "$output_file"
    printf 'binary MiniBASIC payload faulted or delegated a command\n' >&2
    exit 1
fi
printf 'guest binary MiniBASIC payload QEMU test passed\n'
