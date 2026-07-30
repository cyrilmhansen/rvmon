# Index des ADR

## ADR existants hérités

| ID | Décision | Source | Statut | Impact |
|---|---|---|---|---|
| ADR-001 | Séparer ISA, ABI et environnement | SPEC §4–5; D-001 | accepté | contrats de profil |
| ADR-002 | Profil RV64IMAFD_Zicsr_Zifencei | SPEC §5; D-002 | accepté | tables/executor |
| ADR-003 | Pointeurs ILP32 sign-extended | SPEC §5–7; D-003 | accepté | ABI/mémoire |
| ADR-004 | Flags RV64ILP32 locaux | D-004, CONFLICT-ABI-001 | accepté localement | ELF strict |
| ADR-005 | FLEN 64, Q data-only | D-005 | accepté | FP/format |
| ADR-006 | Tables générées R2 | D-006 | obligatoire | CI/générateur |
| ADR-007 | RAM isolée et quotas | D-007 | accepté | backend |
| ADR-008 | Transactions/historique | D-008 | accepté | memory/debug |
| ADR-009 | Grammaire de commandes native | D-009 | accepté | frontend |
| ADR-010 | ELF limité et canonique `.luna` | D-010 | accepté localement | formats |
| ADR-011 | U mono-hart | D-011 | accepté V1 | machine |
| ADR-012 | C explicite, relaxation off | D-012 | accepté | decoder/assembler |
| ADR-013 | FP bit-preserving/déterministe | D-013 | à confirmer par prototype | float |
| ADR-014 | Hiérarchie des sources/conflits | D-014 | accepté | gouvernance |

## ADR à prendre pendant le projet

| ID | Moment | Question | Défaut recommandé | Bloque |
|---|---|---|---|---|
| ADR-015 | M0 | SHA complet R2/R3 et archive | figer les snapshots observés | M1/profil |
| ADR-016 | M0 | aliasage des fenêtres pointeur | aucune aliasing, mapping explicite | ABI/memory |
| ADR-017 | M0 | politique ELF flags draft | refuser toute ambiguïté | import ELF |
| ADR-018 | M4 | backend FP | SoftFloat audité, fallback interdit | F/D |
| ADR-019 | M1 | forme de données générée | JSON + Rust généré, hash manifest | isa |
| ADR-020 | M3 | politique GNU/LLVM unsupported | capability par outil/version | diff tests |
| ADR-021 | M5 | activation C | capability explicite, no auto-relax | C runtime |
| ADR-022 | M7 | step-over/out sans DWARF | heuristique ra/sp documentée | debugger |
| ADR-023 | M8 | quota historique en octets | configurable, désactivable | reverse history |
| ADR-024 | M8 | ELF export contrôlé | différer après `.luna` | release interop |
| ADR-025 | M9 | UI terminal initiale | ratatui/crossterm pinés | frontend |

Chaque ADR doit contenir contexte, décision, alternatives, conséquences, test de contrat et plan de migration. Un ADR ne peut transformer une fonction hors périmètre en exigence sans mise à jour de SPEC et approbation.
