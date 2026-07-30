# Backlog atomique

Convention : 1 point ≈ 0,5 journée-agent ; les estimations incluent tests locaux mais pas revue finale. Une tâche est terminée seulement si sa condition de sortie est satisfaite.

## Étape 0 / M0

### BOOT-001 — Figer les snapshots normatifs

- **Jalon / exigences :** M0; REQ-PROD-002, REQ-PROD-006, ENC-001, ABI-001.
- **But :** enregistrer SHA complet R1–R5, en particulier R2/R3, artefacts, licences et provenance.
- **Non-but :** ne pas modifier les normes ni choisir une compatibilité supplémentaire.
- **Entrées/sources :** R1 v20260120; R2 commit court `c6edca7`; R3 snapshot `master`; R4/R5 référencés.
- **Fichiers/modules :** `norms/`, `tools/lock-norms`, manifest projet.
- **Étapes :** récupérer snapshots; calculer SHA; archiver notices; produire manifest machine-readable; documenter `CONFLICT-ABI-001`.
- **Dépendances/bloqués :** aucune; bloque BOOT-002, GEN-001 et ABI-001.
- **Tests :** vérification SHA sur deux lectures; test d’absence de fichier non manifesté.
- **Acceptation :** CI échoue si un SHA ou une licence manque; `norms/manifest.json` est reproductible.
- **Limites/échecs :** dépôt inaccessible, SHA ambigu, licence absente → diagnostic bloquant `NORM-*`.
- **Taille :** 3 points / 1,5 j, incertitude faible.
- **Compétences/outils :** Git, hashing, licences, scripts shell/Rust.
- **Parallélisable :** oui avec BOOT-003, non avec GEN-001.
- **Contexte minimal :** SPEC §4, DECISIONS D-006/D-014, OPEN_QUESTIONS 1–2.

### BOOT-002 — Geler le contrat de profil et de carte mémoire

- **Jalon / exigences :** M0; ABI-001..016, MEM-001..012, ISO-001.
- **But :** formaliser profils, capabilities, deux fenêtres pointeur et refus ELF.
- **Non-but :** implémenter RAM ou ELF.
- **Entrées/sources :** SPEC §§5–7, DECISIONS D-001..D-004/D-010, R3 `riscv-cc`/`riscv-elf`.
- **Fichiers/modules :** `crates/profile`, `crates/abi`, schémas de manifest, ADR-001.
- **Étapes :** types ProfileId/Capability; table d’adresses; règles `sign_extend_32`; codes ABI; exemples frontière; matrice ELF acceptée/refusée.
- **Dépendances/bloqués :** BOOT-001; bloque assembler ABI, loader et formats.
- **Tests :** propriétés sur 0, `0x7fffffff`, `0x80000000`, `0xffffffff`; fixtures ELF contradictoires.
- **Acceptation :** tous les exemples SPEC passent et aucun cast silencieux n’est accepté.
- **Limites/échecs :** adresse haute non déclarée, flag ambigu, overflow → diagnostic stable.
- **Taille :** 4 points / 2 j, incertitude moyenne.
- **Compétences/outils :** ABI/ELF, property testing.
- **Parallélisable :** oui avec BOOT-003; non avec formats ELF.
- **Contexte minimal :** SPEC §§5–7, DECISIONS D-003/D-004/D-010.

### BOOT-003 — Initialiser workspace, CI et politique de dépendances

- **Jalon / exigences :** M0; REQ-PROD-002, NFR-001..010.
- **But :** fournir build/test/lint/fuzz minimal reproductible.
- **Non-but :** livrer l’application.
- **Entrées/sources :** SPEC §§20–23; licences des outils R1–R5.
- **Fichiers/modules :** `Cargo.toml`, `rust-toolchain.toml`, CI, `DEPENDENCIES.md` provisoire, `norms/dependencies.lock`.
- **Étapes :** créer workspace; pin toolchain; activer fmt/clippy/test; établir matrice OS/arch; enregistrer bibliothèques et licences.
- **Dépendances/bloqués :** aucune; BOOT-001 peut l’alimenter; bloque toutes les crates.
- **Tests :** build offline après cache; test de versions; lint sans warnings nouveaux.
- **Acceptation :** clone propre + commande documentée produit le même binaire de test et rapport.
- **Limites/échecs :** dépendance non licenciée/non résolue → refus de fusion.
- **Taille :** 3 points / 1,5 j, incertitude faible.
- **Compétences/outils :** Rust/Cargo/CI.
- **Parallélisable :** oui avec BOOT-001/002.
- **Contexte minimal :** PLAN §5, RELEASE_CHECKLIST sections sources/CI.

### BOOT-004 — Construire le contrôle R2 contre R1

- **Jalon / exigences :** M0/M1; ENC-001..006, ISA-001..010.
- **But :** détecter divergences mask/match/champs sur le profil.
- **Non-but :** écrire une table d’opcodes à la main.
- **Entrées/sources :** R1 chapitres ISA; R2 README et commit figé.
- **Fichiers/modules :** `crates/opcode-gen`, `tools/check-r1-r2`, rapport golden.
- **Étapes :** parser les sources R2; générer représentation intermédiaire; vérifier largeur/overlap/champs; comparer corpus R1; produire rapport.
- **Dépendances/bloqués :** BOOT-001; bloque GEN-001.
- **Tests :** fixtures d’encodage connu, test de détection d’un masque altéré.
- **Acceptation :** CI échoue sur altération volontaire et passe sans divergence inexpliquée.
- **Limites/échecs :** instruction R1 sans entrée R2 → liste bloquante, jamais ajout manuel.
- **Taille :** 5 points / 2,5 j, incertitude moyenne.
- **Compétences/outils :** parsing de données, encodage RISC-V.
- **Parallélisable :** oui avec BOOT-003; GEN-001 dépend de lui.
- **Contexte minimal :** SPEC §11, DECISIONS D-006.

## M1 — ISA générée

### GEN-001 — Générer tables profile-aware et artefacts hashés

- **Jalon / exigences :** M1; ENC-001..006, ISA-001..010.
- **But :** produire instructions, champs, pseudos, imports et capabilities depuis R2.
- **Non-but :** ajouter des opcodes locaux ou exécuter les instructions.
- **Entrées/sources :** sortie BOOT-004; R2 README “mask/match”, pseudo/import.
- **Fichiers/modules :** `crates/opcode-gen`, `generated/opcodes/<sha>/`, manifest.
- **Étapes :** sélectionner I/M/F/D/Zicsr/Zifencei/C; générer Rust/JSON; conserver source location; générer hash et extension status.
- **Dépendances/bloqués :** BOOT-004; bloque ISA-001 et ADR-019.
- **Tests :** snapshot golden, encode mask/match connu, régénération idempotente.
- **Acceptation :** artefacts identiques deux générations et aucune entrée manuelle.
- **Limites/échecs :** overlap/pseudo non résolu → génération échoue avec source R2.
- **Taille :** 5 points / 2,5 j, incertitude moyenne.
- **Compétences/outils :** générateurs, R2.
- **Parallélisable :** non avec un autre générateur; oui avec BOOT-003.
- **Contexte minimal :** SPEC §11, DECISIONS D-006.

### ISA-001 — Implémenter encode/decode I minimal

- **Jalon / exigences :** M1; ISA-001, REQ-PROD-002.
- **But :** encoder/décoder `addi`, `lui`, `add`, `sub` et produire forme canonique.
- **Non-but :** pseudo-instructions, C, exécution complète.
- **Entrées/sources :** GEN-001; R1 RV64I; R2 champs.
- **Fichiers/modules :** `crates/isa`, `crates/bits`.
- **Étapes :** types instruction/operand; pack/unpack champs; validation extension; format listing bytes.
- **Dépendances/bloqués :** GEN-001; bloque M2.
- **Tests :** golden R2; round-trip; immédiats signés et registres invalides.
- **Acceptation :** `addi x1,x0,1` donne l’encodage attendu et se décode sans perte.
- **Limites/échecs :** immediate hors largeur, opcode illégal → diagnostic `ISA-*`.
- **Taille :** 4 points / 2 j, incertitude faible.
- **Compétences/outils :** Rust bit-level, RISC-V I.
- **Parallélisable :** oui avec MEM-001; non avec M2 integration.
- **Contexte minimal :** SPEC §§11,13, scénarios 1/10.

### ISA-002 — Ajouter décodeur illégal et C longueur variable

- **Jalon / exigences :** M1/M3; ISA-002, DIS-001..006.
- **But :** distinguer 16/32 bits, longueur illégale et données mêlées.
- **Non-but :** relaxation C automatique.
- **Entrées/sources :** R1 C/Instruction-Length; R2 C tables; SPEC §13.
- **Fichiers/modules :** `crates/isa`, `crates/disassembler`.
- **Étapes :** fetch bounded; dispatch C on/off; item illegal; cursor byte-addressed; source/data marks.
- **Dépendances/bloqués :** GEN-001; bloque M3/M5.
- **Tests :** C 16 + 32 mixte, truncation, opcode illégal, C off.
- **Acceptation :** E2E 9/10 passe avec adresse suivante exacte.
- **Limites/échecs :** fin de buffer à 2 octets, fetch non mappé → diagnostic/trap explicite.
- **Taille :** 4 points / 2 j, incertitude moyenne.
- **Compétences/outils :** ISA C, tests golden.
- **Parallélisable :** oui avec ASM-001; dépend de GEN-001.
- **Contexte minimal :** SPEC §13, matrice C.

## M2 — tranche entière

### ASM-BOOT-001 — Assembler la ligne source minimale

- **Jalon / exigences :** M2; REQ-PROD-001/004, ASM-001, ISA-001.
- **But :** assembler réellement `addi x1,x0,1` depuis un document source vers un `ObjectImage` minimal.
- **Non-but :** directives, macros, symboles complexes, compatibilité GNU complète.
- **Entrées/sources :** ISA-001; SPEC §§10–11; R1 RV64I; R4 syntaxe d’instruction.
- **Fichiers/modules :** `crates/assembler` minimal, `crates/asm-lexer` minimal ou parseur temporaire explicitement remplaçable, `tests/e2e`.
- **Étapes :** reconnaître mnemonic et trois registres/immediate; réutiliser encodeur généré; créer section `.text` à base connue; retourner diagnostics avec span; exposer bytes et point d’entrée.
- **Dépendances/bloqués :** ISA-001; bloque DEMO-001; ne doit pas diverger du parser M3.
- **Tests :** source valide, casse, alias `ra`/`x1`, immediate invalide, mnemonic inconnue, round-trip bytes.
- **Acceptation :** aucune fixture binaire n’est utilisée pour le chemin nominal : la source est lexée/parsé/assemblée et produit l’image exécutée par M2.
- **Limites/échecs :** plusieurs instructions ou directives non supportées doivent produire `ASM-BOOT-UNSUPPORTED`, sans parser silencieux.
- **Taille :** 3 points / 1,5 j, incertitude moyenne.
- **Compétences/outils :** parsing assembleur, RISC-V I.
- **Parallélisable :** oui avec MEM-001; non avec ASM-002 sur les mêmes fichiers sans contrat.
- **Contexte minimal :** règle utilisateur 7, SPEC scénario 1, ISA-001.

### MEM-001 — Implémenter RAM sparse et transactions

- **Jalon / exigences :** M2; MEM-001..012, REQ-PROD-003, ISO-001.
- **But :** fournir RAM little-endian isolée, bornes, read/write transactionnels.
- **Non-but :** MMIO complet et UI.
- **Entrées/sources :** SPEC §§6–8,18; R1 Loads/Stores/Exceptions.
- **Fichiers/modules :** `crates/memory`.
- **Étapes :** pages 4 KiB; map; read widths; write set; commit/rollback; fault types; quota 256 MiB.
- **Dépendances/bloqués :** BOOT-002/003; bloque MACHINE-001.
- **Tests :** endian, alignement, unmapped, atomic rollback, quota.
- **Acceptation :** aucune lecture RAM ne lit l’hôte; mutation refusée laisse zéro changement.
- **Limites/échecs :** cross-page, MMIO range, overflow adresse → trap/diagnostic.
- **Taille :** 5 points / 2,5 j, incertitude moyenne.
- **Compétences/outils :** mémoire virtuelle, property testing.
- **Parallélisable :** oui avec ISA-001.
- **Contexte minimal :** SPEC §§6–8/14/18.

### MACHINE-001 — Exécuter un hart RV64I et un pas

- **Jalon / exigences :** M2; REQ-PROD-001/002, ISA-001, DBG-001.
- **But :** charger bytes, fetch/decode/execute `addi`, exposer état avant/après.
- **Non-but :** run loop, FP, UI.
- **Entrées/sources :** ISA-001, MEM-001; R1 RV64I.
- **Fichiers/modules :** `crates/machine`.
- **Étapes :** MachineState; x0 invariant; PC increment; instruction counter; trap illegal; `step()` contract.
- **Dépendances/bloqués :** ISA-001/MEM-001; bloque DEMO-001.
- **Tests :** `addi x1,x0,1`, x0 write ignored, PC, illegal trap.
- **Acceptation :** harness charge et après un seul step observe `x1=1`, PC+4, compteur=1.
- **Limites/échecs :** PC non aligné/unmapped, run absent → résultat structuré.
- **Taille :** 4 points / 2 j, incertitude faible.
- **Compétences/outils :** simulateur RISC-V.
- **Parallélisable :** non avec DEMO-001; oui avec BOOT-003.
- **Contexte minimal :** SPEC §§6/8/16, règle utilisateur 7.

### DEMO-001 — Démonstration M2 scriptée

- **Jalon / exigences :** M2; REQ-PROD-001..004.
- **But :** livrer un test d’intégration source→assemble→load→step→print.
- **Non-but :** frontend interactif.
- **Entrées/sources :** ISA-001, MACHINE-001, MEM-001.
- **Fichiers/modules :** `tests/e2e/addi_step`, script CI.
- **Étapes :** fournir la source à ASM-BOOT-001; journaliser bytes/état; assertion x1.
- **Dépendances/bloqués :** ASM-BOOT-001, MACHINE-001 et MEM-001; bloque M2 sign-off.
- **Tests :** exécution répétée et hash de sortie.
- **Acceptation :** test lisible échoue si bytes, PC ou x1 divergent.
- **Limites/échecs :** erreur de source et opcode illégal testés séparément.
- **Taille :** 2 points / 1 j, incertitude faible.
- **Compétences/outils :** tests E2E.
- **Parallélisable :** non, intégration.
- **Contexte minimal :** règle utilisateur 7, SPEC scénario 1.

## M3 — assembleur et désassembleur

### ASM-001 — Lexer, registres et diagnostics de position

- **Jalon / exigences :** M3; ASM-001..015, REQ-PROD-004.
- **But :** lexer Unicode/commentaires/casse/registres avec spans stables.
- **Non-but :** macros et émission.
- **Entrées/sources :** SPEC §11; R4 dialecte.
- **Fichiers/modules :** `crates/asm-lexer`, `crates/diag`.
- **Étapes :** NFC policy; token numbers/strings/registers/identifiers; comments; source map byte/column.
- **Dépendances/bloqués :** BOOT-003; bloque parser.
- **Tests :** golden tokens, UTF-8, malformed escape, aliases ABI, comments.
- **Acceptation :** chaque erreur contient fichier/ligne/colonne/span/code.
- **Limites/échecs :** Unicode non NFC, identifiant ambigu, dépassement littéral.
- **Taille :** 4 points / 2 j, incertitude faible.
- **Compétences/outils :** lexer Unicode/Rust.
- **Parallélisable :** oui avec FP-001.
- **Contexte minimal :** SPEC §§11/19.

### ASM-002 — Expressions, labels et deux passes

- **Jalon / exigences :** M3; ASM-001..015, ABI-001.
- **But :** AST expressions, symboles globaux/locaux, sections et relocations internes.
- **Non-but :** linker ELF général.
- **Entrées/sources :** R4 expressions/relocations; SPEC §11.
- **Fichiers/modules :** `asm-parser`, `assembler`, `formats`.
- **Étapes :** Pratt/precedence; signed 128 evaluation; symbol table; pass 1/2; unresolved relocation; overflow.
- **Dépendances/bloqués :** ASM-001, profile; bloque directives/listing.
- **Tests :** precedence, local scope, overflow, undefined symbol, `hi20/lo12`, relocation snapshot.
- **Acceptation :** mêmes bytes avec ou sans labels équivalents; overflow n’est jamais wrap silencieux.
- **Limites/échecs :** duplicate local/global, range inverse, relocation non exportable.
- **Taille :** 5 points / 2,5 j, incertitude moyenne.
- **Compétences/outils :** parseurs, relocations RISC-V.
- **Parallélisable :** oui avec DIS-001; dépend ASM-001.
- **Contexte minimal :** SPEC §11, R4 relocations.

### ASM-003 — Directives, chaînes, macros et listing

- **Jalon / exigences :** M3; ASM-001..015, IO-001..009.
- **But :** directives V1, macros bornées, conditionnel, listing/map.
- **Non-but :** compatibilité de toutes les directives GNU/ASM-One.
- **Entrées/sources :** SPEC §11–12; R4 directives; A1 interaction.
- **Fichiers/modules :** `assembler`, `formats`, `diag`.
- **Étapes :** sections/data/align/include; `.macro`; recursion/depth quota; listing source→address→bytes; map.
- **Dépendances/bloqués :** ASM-002, FP-001 pour literals; bloque M3 sign-off.
- **Tests :** data endian, align, include sandbox, macro recursion, listing.
- **Acceptation :** source M3 produit image/listing/map déterministes et erreurs reliées.
- **Limites/échecs :** include extérieur, align overflow, macro non terminée.
- **Taille :** 5 points / 2,5 j, incertitude élevée.
- **Compétences/outils :** assembler/linker minimal, sécurité chemins.
- **Parallélisable :** oui avec DIS-001 et FP-001.
- **Contexte minimal :** SPEC §§11/12/18.

### DIS-001 — Désassembleur canonique et symbolisation

- **Jalon / exigences :** M3; DIS-001..006, MEM-001..012.
- **But :** afficher instruction réelle, pseudo optionnel, symbols, data/code mixed.
- **Non-but :** inférer des données sans marks.
- **Entrées/sources :** ISA-002; R1/R2; SPEC §13.
- **Fichiers/modules :** `crates/disassembler`, `monitor-model`.
- **Étapes :** decode items; address cursor; pseudo proof; symbol lookup; illegal item; bytes consumed.
- **Dépendances/bloqués :** ISA-001/002, ASM-002 pour symbols.
- **Tests :** assemble→disassemble→reassemble, C mixed, illegal, truncated.
- **Acceptation :** round-trip canonical stable et address never lost.
- **Limites/échecs :** data bytes not executable, overlapping symbols, unknown extension.
- **Taille :** 4 points / 2 j, incertitude moyenne.
- **Compétences/outils :** disassembly/round-trip.
- **Parallélisable :** oui avec ASM-003.
- **Contexte minimal :** SPEC §13, scénarios 3/9/10.

## M4–M5 — flottants et extensions

### FP-001 — Représenter formats et littéraux exacts

- **Jalon / exigences :** M4/M5; FP-001..018, REQ-PROD-002/004.
- **But :** bits binary16/32/64/128, décimal/hex/bitsN, display round-trip.
- **Non-but :** exécuter Q/Zfh.
- **Entrées/sources :** R1 F/D/Q; IEEE rules in SPEC §11.1; DECISIONS D-005/D-013.
- **Fichiers/modules :** `crates/float`, `assembler`, `monitor-model`.
- **Étapes :** format descriptors; exact bit parser; shortest display; special values; no host-dependent conversion.
- **Dépendances/bloqués :** BOOT-001/003; ASM-001; bloque FP-002.
- **Tests :** all four formats, ±0/inf/subnormal/sNaN/qNaN/payload, malformed width.
- **Acceptation :** `.binary128 bits128(...)` round-trips exact; bfloat16 never aliases binary16.
- **Limites/échecs :** decimal nonrepresentable warns/errors per mode; NaN payload incomplete.
- **Taille :** 5 points / 2,5 j, incertitude élevée.
- **Compétences/outils :** IEEE 754, arbitrary precision.
- **Parallélisable :** oui avec FP-ORACLE-001.
- **Contexte minimal :** SPEC §11.1, scénarios 6/7/13.

### FP-ORACLE-001 — Prototyper et figer l’oracle F

- **Jalon / exigences :** M4; FP-001..018, ISA F.
- **But :** comparer une implémentation logicielle candidate à SoftFloat/Sail/Spike/GNU disponibles.
- **Non-but :** intégrer un oracle dans le runtime sans audit.
- **Entrées/sources :** R1 F; R5 Sail; SoftFloat version/licence candidate.
- **Fichiers/modules :** `tools/oracles`, `norms/dependencies.lock`, rapport ADR.
- **Étapes :** construire adapters; 1000 motifs; comparer result bits/flags; mesurer unsupported cases; choisir backend.
- **Dépendances/bloqués :** BOOT-003; BOOT-001 pour versions; bloque FP-002.
- **Tests :** corpus fixe et mutation de l’implémentation pour prouver détection.
- **Acceptation :** oracle indépendant détecte une erreur injectée et choix/licence sont enregistrés.
- **Limites/échecs :** outil absent → capability `unavailable`, jamais comparaison self-to-self.
- **Taille :** 5 points / 2,5 j, incertitude élevée.
- **Compétences/outils :** IEEE, Sail/Spike, FFI éventuel.
- **Parallélisable :** oui avec FP-001, non FP-002.
- **Contexte minimal :** SPEC §§4/11.1/23, règle utilisateur 5/8.

### FP-002 — Exécuter fadd.s avec fcsr et NaN-boxing

- **Jalon / exigences :** M4; FP-001..018, DBG-001, REQ-PROD-002.
- **But :** ajouter f-register bits, `fadd.s`, rm dynamique/statique, flags et box validation.
- **Non-but :** toutes les instructions F/D.
- **Entrées/sources :** FP-001, FP-ORACLE-001; R1 F “fcsr”.
- **Fichiers/modules :** `float`, `machine`, `isa`.
- **Étapes :** f[32]:u64; fcsr fields; operation; sticky flags; NaN-box; trace changes; profile gate.
- **Dépendances/bloqués :** FP-ORACLE-001; MACHINE-001.
- **Tests :** `fadd.s` normal, ±0, subnormal, NaN, invalid box, all rm.
- **Acceptation :** E2E 5/6/13 match motif oracle and exact flags; hôte x86/arm same output.
- **Limites/échecs :** rm reserved, nonbox, unsupported profile → stable diagnostic/trap.
- **Taille :** 5 points / 2,5 j, incertitude élevée.
- **Compétences/outils :** IEEE/RISC-V F.
- **Parallélisable :** non avec FP-ORACLE integration.
- **Contexte minimal :** SPEC §§5/8/11.1/24.

### FP-003 — Étendre D et refuser proprement Q/Zfh exécutable

- **Jalon / exigences :** M5; F/D/Zfh/Q matrix.
- **But :** implémenter D nécessaire, données Zfh/Q et capability refusée.
- **Non-but :** Q runtime.
- **Entrées/sources :** R1 D/Q; R2; DECISIONS D-005.
- **Fichiers/modules :** `float`, `machine`, `isa`, profile.
- **Étapes :** `fadd.d` puis conversions nécessaires; binary16/128 data; execute gate; diagnostics.
- **Dépendances/bloqués :** FP-002, GEN-001; bloque M5.
- **Tests :** D motifs/flags; Q decode/data; executing Q trap; matrix generated.
- **Acceptation :** E2E 5/7 et aucune extension non exécutée n’est présentée comme support.
- **Limites/échecs :** wrong FLEN, invalid Q opcode, Zfh instruction → explicit unsupported.
- **Taille :** 5 points / 2,5 j, incertitude moyenne.
- **Compétences/outils :** IEEE/RISC-V D/Q.
- **Parallélisable :** oui avec CMD-001 après contract.
- **Contexte minimal :** SPEC §5.3/11.1.

## M6–M8 — interface, debug, persistance

### CMD-001 — Parser commandes et expressions contrôlées

- **Jalon / exigences :** M6; CMD-001..005, REQ-PROD-005.
- **But :** implémenter EBNF, alias, erreurs, help sans effets de bord.
- **Non-but :** vues UI complètes.
- **Entrées/sources :** SPEC §10/19.
- **Fichiers/modules :** `crates/command`, `diag`.
- **Étapes :** lexer line; expression evaluator signed 128; command AST; validation context; help catalog.
- **Dépendances/bloqués :** profile/diag; peut utiliser machine contract.
- **Tests :** grammar golden, precedence, invalid range, mutation while run, help snapshot.
- **Acceptation :** exemples SPEC parsés et erreurs codes stables.
- **Limites/échecs :** unknown command, overflow context, side-effect condition.
- **Taille :** 4 points / 2 j, incertitude moyenne.
- **Compétences/outils :** parser/CLI.
- **Parallélisable :** oui avec DBG-001.
- **Contexte minimal :** SPEC §10/19.

### MON-001 — Vues mémoire, marks et QuickJump

- **Jalon / exigences :** M6; MEM-001..012, REQ-PROD-003/005.
- **But :** modèle partagé code/hex/ASCII, curseur, sélection, marks, edit undo.
- **Non-but :** widgets spécifiques.
- **Entrées/sources :** SPEC §14/15; A1 moniteur.
- **Fichiers/modules :** `monitor-model`, `memory`, `formats`.
- **Étapes :** view adapters; byte cursor; marks; QuickJump expression/symbol; transactions; search/fill/copy.
- **Dépendances/bloqués :** MEM-001, DIS-001, CMD-001.
- **Tests :** address retention, cross-view, rollback, ASCII byte behavior.
- **Acceptation :** E2E 2/3 sans perte d’adresse et undo exact.
- **Limites/échecs :** invalid range, MMIO, unmapped, concurrent run.
- **Taille :** 4 points / 2 j, incertitude moyenne.
- **Compétences/outils :** modèles UI, mémoire.
- **Parallélisable :** oui avec REG-001.
- **Contexte minimal :** SPEC §§14–15, scénarios 2/3.

### REG-001 — Contrat et vues exactes des registres

- **Jalon / exigences :** M6/M7; REQ-PROD-003/005, DBG-001..014, FP-001..018.
- **But :** exposer groupes x/f/csr, aliases ABI, changements surlignés et éditions validées.
- **Non-but :** construire les widgets terminaux.
- **Entrées/sources :** SPEC §8/15; R3 register convention; R1 F/Zicsr.
- **Fichiers/modules :** `crates/monitor-model`, `machine`, `float`, `diag`.
- **Étapes :** types de vue; formatter hex/signed/float; diff depuis stop; x0 read-only; fcsr fields; validation NaN-box/CSR.
- **Dépendances/bloqués :** MACHINE-001, FP-002, profile; bloque MON-001/UI-001 pour le panneau registres.
- **Tests :** x0 write, ABI aliases, changed highlight, exact float bits, invalid CSR, invalid box.
- **Acceptation :** le panneau et l’API renvoient la même valeur bit-exacte et chaque édition valide une transaction ou renvoie un diagnostic.
- **Limites/échecs :** CSR absent, format décimal ambigu, run actif → refus sans mutation.
- **Taille :** 3 points / 1,5 j, incertitude moyenne.
- **Compétences/outils :** états machine, IEEE display, UI model.
- **Parallélisable :** oui avec MON-001; intégration avec DBG-001.
- **Contexte minimal :** SPEC §§8/15/16, règle utilisateur 8.

### DBG-001 — Breakpoints, watches et stepping contractuel

- **Jalon / exigences :** M7; DBG-001..014, REQ-PROD-002/003.
- **But :** run/pause/step-over/out, break/watch conditions, traps.
- **Non-but :** UI terminal finale.
- **Entrées/sources :** SPEC §16; A1 debugger; R1 traps.
- **Fichiers/modules :** `debugger`, `machine`.
- **Étapes :** stop reasons; breakpoint resolution labels/lines; watch access; no-side-effect condition; step stack heuristic.
- **Dépendances/bloqués :** MACHINE-001, ASM-002, DIS-001.
- **Tests :** label break, register highlight event, watch write/read, infinite quota, trap.
- **Acceptation :** E2E 4/11 et état arrêté avant instruction suivante.
- **Limites/échecs :** self-modifying code, unresolved label, recursive call, watch MMIO.
- **Taille :** 5 points / 2,5 j, incertitude élevée.
- **Compétences/outils :** debugging/runtime.
- **Parallélisable :** oui avec MON-001; intégration machine nécessaire.
- **Contexte minimal :** SPEC §16/18/24.

### FORMAT-001 — Projets, snapshots et replay

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** formats versionnés, état/symboles/breakpoints, hash reproductible.
- **Non-but :** ELF général.
- **Entrées/sources :** SPEC §§8/12/21/22; DECISIONS D-008/D-010.
- **Fichiers/modules :** `formats`, `app`, `memory`, `machine`.
- **Étapes :** canonical serialization; snapshot manifest; schema migration; journal; crash recovery; replay runner.
- **Dépendances/bloqués :** profile, assembler, debugger; bloque M8.
- **Tests :** round-trip, hash cross-platform, corrupt file, migration, crash journal.
- **Acceptation :** E2E 12 restaure bytes/symboles/état/breakpoints identiques.
- **Limites/échecs :** version majeure, missing source, tampered hash → refusal with remedy.
- **Taille :** 5 points / 2,5 j, incertitude moyenne.
- **Compétences/outils :** formats canoniques, persistence.
- **Parallélisable :** oui avec UI-001 après contracts.
- **Contexte minimal :** SPEC §§12/21/22.

### UI-001 — Frontend terminal et cycle ASM-One modernisé

- **Jalon / exigences :** M6–M9; REQ-PROD-001/005, E2E 2/3/4.
- **But :** connecter commandes, éditeur minimal, vues, registre, diagnostics et keymap.
- **Non-but :** frontend graphique ou compatibilité Amiga pixel-perfect.
- **Entrées/sources :** SPEC §§10/14–17; A1 ch.6–9.
- **Fichiers/modules :** `frontend-terminal`, `app`.
- **Étapes :** pane model; keymap; error navigation; Ctrl+Enter/F5/F10/F11; state highlighting; accessibility baseline.
- **Dépendances/bloqués :** CMD-001, MON-001, DBG-001, FORMAT-001 partiel.
- **Tests :** scripted terminal interaction, keymap coverage, no address loss, crash restore.
- **Acceptation :** démonstration M6/M7 sans mutation hors commande et clavier complet.
- **Limites/échecs :** terminal étroit, unicode, resize, unavailable color.
- **Taille :** 6 points / 3 j, incertitude élevée.
- **Compétences/outils :** terminal UI/accessibilité.
- **Parallélisable :** oui mais interface publique gelée; fusion avec M6/M7.
- **Contexte minimal :** SPEC §§14–17, A1.

## M9 — qualité et release

### QUAL-001 — Fuzzing, corpus et réduction automatique

- **Jalon / exigences :** M9; REQ-PROD-002/004, ISA/ASM/DIS/CMD.
- **But :** fuzz parser, decoder, commandes, expressions, snapshots et réduire les contre-exemples.
- **Non-but :** prouver la sémantique seul.
- **Entrées/sources :** SPEC §23; corpus R2/GNU/LLVM/Sail.
- **Fichiers/modules :** `tests/fuzz`, `tools/reduce`, CI.
- **Étapes :** targets; seed corpus; budgets PR 60 s/nightly 30 min; deterministic replay; reducer; issue artifact.
- **Dépendances/bloqués :** M3–M8 contracts.
- **Tests :** injected crash, seed replay, no nondeterministic failure.
- **Acceptation :** chaque crash réduit et rejouable; zéro crash connu non classé.
- **Limites/échecs :** oracle indisponible, timeout → classer unsupported, pas passer vert.
- **Taille :** 4 points / 2 j, incertitude moyenne.
- **Compétences/outils :** fuzzing/property testing.
- **Parallélisable :** oui avec QUAL-002.
- **Contexte minimal :** TEST_PLAN fuzzing.

### QUAL-002 — Benchmarks, multi-plateforme et accessibilité

- **Jalon / exigences :** M9; NFR-001..010.
- **But :** mesurer p95, mémoire, déterminisme x86_64/arm64 et clavier/contraste.
- **Non-but :** optimisation spéculative avant mesure.
- **Entrées/sources :** SPEC §20; RELEASE_CHECKLIST.
- **Fichiers/modules :** `tools/bench`, CI matrix, docs.
- **Étapes :** corpus 64 KiB/256 MiB sparse; latency commands; snapshot; replay; terminal keyboard/contrast tests.
- **Dépendances/bloqués :** M8, UI-001.
- **Tests :** thresholds, repeated runs, host comparison.
- **Acceptation :** seuils SPEC passés ou waiver signé avec cause et impact.
- **Limites/échecs :** hardware slow/no TTY → rapport séparé, pas masquer.
- **Taille :** 4 points / 2 j, incertitude moyenne.
- **Compétences/outils :** benchmarking, accessibility.
- **Parallélisable :** oui avec QUAL-001.
- **Contexte minimal :** SPEC §20.

### REL-001 — Release candidate et dossier de preuve

- **Jalon / exigences :** M9; toutes exigences couvertes.
- **But :** assembler rapport traceability, changelog, hashes, SBOM/licences, quickstart et démo.
- **Non-but :** nouvelles fonctionnalités.
- **Entrées/sources :** tous jalons; `TRACEABILITY.md`, `RELEASE_CHECKLIST.md`.
- **Fichiers/modules :** release scripts/docs/artifacts.
- **Étapes :** clean checkout; regenerate tables; run full matrix; package norms manifest; sign hashes; run E2E 1–14.
- **Dépendances/bloqués :** QUAL-001/002, FORMAT-001, BOOT-001.
- **Tests :** release install/replay on supported hosts; negative package test.
- **Acceptation :** checklist entièrement verte ou waivers approuvés; aucune table modifiée à la main.
- **Limites/échecs :** source drift, missing oracle evidence, nondeterministic hash → no release.
- **Taille :** 4 points / 2 j, incertitude faible.
- **Compétences/outils :** release engineering, documentation.
- **Parallélisable :** non, intégration finale.
- **Contexte minimal :** tous documents du projet.
