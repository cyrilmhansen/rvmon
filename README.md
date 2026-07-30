# RVMonitor — RV64ILP32 monitor

Bootstrap implementation of the first vertical slice described in `PLAN.md`.

```text
cargo test --workspace
cargo run -p luna-app
```

The current slice assembles the source line `addi x1,x0,1`, loads it into isolated target memory, executes one RV64I step, and reports the resulting register state. The `addi` encoding is derived at build time from the pinned R2 extract in `norms/r2/rv_i`; it is not maintained as a hand-edited opcode table.

The implementation is intentionally not yet the complete monitor. ISA coverage, the full R2 generator, floating point, commands, debugger, persistence, and terminal UI remain backlog work.
