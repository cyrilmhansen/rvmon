# libFuzzer targets

This directory is an autonomous fuzzing package and is intentionally not a
member of the main Cargo workspace. The normal CI path remains
`tools/fuzz-smoke.sh`, which has no external fuzzing dependency.

After installing `cargo-fuzz` and allowing Cargo to fetch the pinned
`libfuzzer-sys` dependency:

```text
cargo fuzz run commands -- -timeout=5 -max_total_time=60
cargo fuzz run disassembler -- -timeout=5 -max_total_time=60
```

The command target exercises the public monitor command surface. The
disassembler target consumes complete little-endian 32-bit words and ignores
an incomplete trailing fragment. These targets are panic-freedom harnesses,
not independent semantic oracles. Crash artifacts should be reduced with
`tools/reduce-fuzz-case.sh` when they can be represented as text.
