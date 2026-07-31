# Matrice de traçabilité

| Exigences | Source/section | Composant | Test d’acceptation |
|---|---|---|---|
| REQ-PROD-001..006 | A1 ch.6–9; contraintes produit | frontend, workspace, snapshots | E2E 1,2,12 |
| ISA-001..010 | R1 Vol I, chap. RV64I/M/F/D/C/Zicsr/Zifencei/exceptions; R2 Zfh/Zfhmin/Q snapshots | tables, executor, decoder | encode/decode + decode-only Zfh/Q round-trip + E2E 9,10 |
| ABI-001..012 | R3 `riscv-cc.adoc`, RV64ILP32*, integer/FP CC | ABI validator, loader | E2E 8, appels, variadiques |
| ABI-013..016 | R3 `riscv-elf.adoc` | ELF loader | corpus ELF cohérent/incohérent |
| ENC-001..006 | R2 README Encoding Syntax, mask/match, pseudo/import | generator, assembler | commit-pinned table diff |
| ASM-001..015 | R4 directives/pseudos/relocations | lexer, parser, two-pass assembler | GNU/LLVM differential + fuzz |
| FP-001..018 | R1 F/D/Q, R3 FP CC | FP engine, register view, generated decode gate | bits IEEE, flags, NaN-box, unsupported-extension trap |
| MEM-001..012 | R1 Loads/Stores/Exceptions; A1 moniteur | memory service/UI | E2E 2,3, boundary tests |
| DBG-001..014 | A1 debugger; R1 traps | debugger/backend | E2E 4,10,11 |
| ISO-001..010 | contraintes produit; R5 config/oracle | backend, file service | include/path, quota, crash recovery |
| IO-001..009 | A1 sauvegarde; format local | file service | round-trip project/snapshot |
| NFR-001..010 | contraintes produit | build/CI/UI | benchmark p95, cross-host hash |
| OBS-001..006 | contraintes produit | journal/trace/export | replay hash-identique |

## Tests différentiels obligatoires

T-ENC compare chaque instruction sélectionnée à R2 SHA `c6edca7`, puis round-trip encode/decode. `tools/check-oracles.sh` compare actuellement GNU Binutils 2.44 et LLVM MC 22.1.8 à sept encodages du corpus R1 v20260120 couvrant I/U, `ld/sd`, F et D. T-SEM compare le moteur à Sail R5 et Spike pour les instructions couvertes. T-ASM compare `as/objdump` et `llvm-mc` seulement quand le profil et l’ABI sont effectivement acceptés. T-FP utilise motifs `±0`, `±inf`, min/max normal, subnormaux, sNaN/qNaN et flags. T-FUZZ cible parser, decoder, expressions, commandes et chemins.

## Gaps connus

Les quotas, raccourcis, format `.luna`, présentation UI et compatibilité cross-platform sont des exigences produit sans section normative externe ; ils sont couverts par benchmarks et tests de contrat. La psABI draft et l’ELF RV64ILP32 restent des sources à revalider au gel d’implémentation.
