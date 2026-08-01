# Deterministic fuzz smoke corpus

The first QUAL-001 tranche uses the Rust test harness and fixed LCG seeds so
that every run is reproducible without `cargo-fuzz` or an external service.
It also includes a versioned seed corpus under `tests/fuzz/seeds/` covering
normal, boundary and invalid monitor commands and expressions.

Run:

```text
bash tools/fuzz-smoke.sh
```

The command runs 20,000 generated command/expression inputs and 100,000
arbitrary 32-bit instruction words, then exercises the real monitor against
the checked-in seeds. These tests establish panic-freedom and bounded parsing
only; they are not independent semantic oracles. Nightly libFuzzer integration
remains a later QUAL-001 task.

Text counterexamples can be reduced with:

```text
tools/reduce-fuzz-case.sh INPUT OUTPUT CHECKER CHECKER_ARG...
```

The checker receives the candidate path as its last argument and exits zero
while the failure still reproduces. The reducer is greedy and line-oriented;
it is intended for command/diagnostic cases, not binary instruction streams.
