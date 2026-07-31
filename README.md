# RVMonitor — RV64ILP32 monitor

Bootstrap implementation of the first vertical slice described in `PLAN.md`.

```text
cargo test --workspace
cargo run -p luna-app
```

Le parcours prioritaire, avec le moniteur exécuté dans QEMU, est documenté
dans [docs/TUTORIAL-GUEST.md](docs/TUTORIAL-GUEST.md). Voir aussi
[docs/TUTORIAL.md](docs/TUTORIAL.md) pour le simulateur hôte et la console
hôte connectée à QEMU par GDB RSP.

The current slice includes the isolated RV64 simulator, the backend-neutral
console, generic breakpoints/watchpoints/history, and a live GDB RSP path to
QEMU. The integer, branch, jump, and `fadd.s`/`fadd.d` encodings are derived
at build time from the pinned R2 extracts; they are not maintained as
hand-edited opcode tables.

The implementation is intentionally not yet the complete monitor. Native
QEMU watchpoints, full target-state snapshots, richer terminal UI, and broader
ISA coverage remain backlog work. Normative source identities are recorded in
`norms/manifest.toml`; the current bootstrap uses only the pinned `rv_i`
extract for the bootstrap path while the profile generator validates all
selected R2 extracts. Run `bash tools/check-r2.sh` to verify provenance,
hashes, and generator inputs before changing ISA data.
