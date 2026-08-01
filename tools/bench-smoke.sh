#!/usr/bin/env bash
set -euo pipefail

# Timing is reported for comparison, not used as a release gate yet.
cargo run -p luna-monitor --example bench_smoke --release --quiet
