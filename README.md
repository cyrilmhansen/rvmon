# RVMonitor — RV64ILP32 monitor

Bootstrap implementation of the first vertical slice described in `PLAN.md`.

```text
cargo test --workspace
cargo run -p luna-app
```

Voir [docs/TUTORIAL.md](docs/TUTORIAL.md) pour les exemples interactifs du
moniteur interne et de la connexion QEMU.

The current slice includes the isolated RV64 simulator, the backend-neutral
console, generic breakpoints/watchpoints/history, and a live GDB RSP path to
QEMU. The `addi` encoding is derived at build time from the pinned R2 extract
in `norms/r2/rv_i`; it is not maintained as a hand-edited opcode table.

The implementation is intentionally not yet the complete monitor. Native
QEMU watchpoints, full target-state snapshots, richer terminal UI, and broader
ISA coverage remain backlog work. Normative source identities are recorded in
`norms/manifest.toml`; the current bootstrap uses only the pinned `rv_i`
extract while the full generator is being built.
