#!/usr/bin/env bash
set -uo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

strict_oracles=0
report=""
while (($#)); do
    case "$1" in
        --strict-oracles) strict_oracles=1 ;;
        --report)
            shift
            (($#)) || { printf '%s\n' '--report requires a path' >&2; exit 2; }
            report="$1"
            ;;
        --help|-h)
            printf '%s\n' 'usage: tools/release-audit.sh [--report PATH] [--strict-oracles]'
            printf '%s\n' 'default: deterministic local release checks; strict mode adds GNU/LLVM oracle checks'
            exit 0
            ;;
        *) printf 'unknown option: %s\n' "$1" >&2; exit 2 ;;
    esac
    shift
done

if [[ -n "$report" ]]; then
    mkdir -p "$(dirname -- "$report")"
    exec > >(tee "$report")
    exec 2>&1
fi

printf '%s\n' 'RVMonitor release audit' 'profile=local-core' 'report-format=stable-text-v1'
printf 'strict-oracles=%s\n\n' "$strict_oracles"

failures=0
run_check() {
    local name="$1"
    shift
    printf '[CHECK] %s\n' "$name"
    printf '[COMMAND]'
    printf ' %q' "$@"
    printf '\n'
    "$@"
    local status=$?
    if ((status == 0)); then
        printf '[PASS] %s\n\n' "$name"
    else
        printf '[FAIL] %s (exit=%s)\n\n' "$name" "$status"
        failures=$((failures + 1))
    fi
}

run_check 'working-tree-whitespace' git diff --check
run_check 'rustfmt' cargo fmt --all -- --check
run_check 'workspace-tests' cargo test --workspace --quiet
run_check 'fuzz-manifest' cargo metadata --manifest-path fuzz/Cargo.toml --no-deps --format-version 1 >/dev/null
run_check 'release-dossier' bash tools/generate-release-dossier.sh --check
run_check 'release-policy' bash tools/check-release-policy.sh
run_check 'r2-generated-tables' bash tools/check-r2.sh
run_check 'fuzz-smoke' bash tools/fuzz-smoke.sh
run_check 'terminal-accessibility-smoke' bash tools/accessibility-smoke.sh
run_check 'release-benchmark-smoke' bash tools/bench-smoke.sh
if [[ "${RELEASE_E2E:-0}" == 1 ]]; then
    run_check 'guest-qemu-e2e' bash tools/e2e-release-smoke.sh --strict
else
    printf '[SEPARATE] guest-qemu-e2e (set RELEASE_E2E=1)\n\n'
fi

if ((strict_oracles)); then
    run_check 'gnu-llvm-independent-oracles' bash tools/check-oracles.sh
else
    printf '[SEPARATE] gnu-llvm-independent-oracles (use --strict-oracles)\n'
    printf '[SEPARATE] Sail/Spike oracle evidence is not part of the local audit\n\n'
fi

if ((failures == 0)); then
    printf 'RESULT=PASS\n'
else
    printf 'RESULT=FAIL failures=%s\n' "$failures"
fi
exit "$failures"
