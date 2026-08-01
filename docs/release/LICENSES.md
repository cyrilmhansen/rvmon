# Licences et provenance de release

Ce dossier ne redistribue pas les documents externes R1–R5/A1–A2/C1. Il
référence leurs licences et leur statut dans `docs/SOURCES.toml` et
`norms/manifest.toml`. Toute redistribution d’un document externe reste
conditionnée à la vérification de sa licence.

## Projet

- RVMonitor et ses crates : Apache-2.0 (`Cargo.toml`).
- Fuzz harness : Apache-2.0 (`fuzz/Cargo.toml`).

## Données et normes référencées

- R2 `riscv-opcodes` : BSD-3-Clause, extraits sous `norms/r2/`.
- R3 psABI : CC-BY-4.0, snapshot externe non vendored, statut expérimental.
- R4 Assembly Programmer’s Manual : CC-BY-4.0, référence externe.
- R5 Sail RISC-V : BSD-2-Clause, oracle externe.
- GNU Binutils : GPL-3.0-or-later, oracle indépendant optionnel.
- LLVM : Apache-2.0 WITH LLVM-exception, oracle indépendant optionnel.
- QEMU : GPL-2.0-or-later, oracle sémantique externe.

La liste exacte des crates et checksums est dans `SBOM.tsv`; elle est issue de
`cargo metadata` et de `Cargo.lock`, sans dépendance à un chemin hôte.
