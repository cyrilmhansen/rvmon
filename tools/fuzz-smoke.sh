#!/usr/bin/env bash
set -euo pipefail

# Deterministic, dependency-free smoke fuzzing for CI and local replay.
# These tests check panic-freedom and bounded parsing; they are not semantic
# oracles and do not replace nightly libFuzzer runs.
cargo test -p luna-monitor fuzz_smoke_commands_and_expressions_are_bounded_and_panic_free --quiet
cargo test -p luna-disassembler fuzz_smoke_arbitrary_words_are_panic_free --quiet
