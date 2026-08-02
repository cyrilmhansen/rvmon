# Backlog atomique

Convention : 1 point ≈ 0,5 journée-agent ; les estimations incluent tests locaux mais pas revue finale. Pour un agent GPT-5.6 Luna High, on budgète
indicativement 20k–40k tokens de travail total par point, ou 40k–80k par
journée-agent. Ce sont des tokens de contexte + sortie + raisonnement ; ils ne
mesurent pas la taille du diff et ne remplacent pas la condition de sortie.
Une tâche est terminée seulement si sa condition de sortie est satisfaite.

## État de suivi au 2026-08-01

Le backlog conserve des tâches agrégées historiques dont les sous-tâches sont
déjà livrées. Cette table est la source de lecture de l’avancement jusqu’à la
réconciliation détaillée des titres : une tâche agrégée ne doit pas être
réimplémentée si les preuves indiquées existent.

| Groupe | État constaté | Preuve principale | Action de planification |
|---|---|---|---|
| BOOT-005/006, GEN-002 | livré | `tools/check-r2.sh`, `tools/check-oracles.sh`, artefacts `norms/r2` | conserver comme garde-fous ; BOOT-004 et GEN-001 restent à clôturer formellement |
| GUEST-001/002, ISA-003/004 | livré | `scripts/test-guest-monitor.sh`, `scripts/test-guest-ld-sd.sh` | ne pas rouvrir ; préparer les extensions guest dédiées |
| ASM-001A..G, DIS-001A..C | livré par sous-tranches | tests assembleur/désassembleur et `docs/TESTS.md` | clôturer les agrégats ASM-001/002/003/DIS-001 après vérification de couverture |
| FP-001A, FP-002A, FP-003A, FP-004A/B, FP-005A, GEN-003 | livré | tests `luna-machine`, probes QEMU et tests de formats | traiter FP-002 comme partiellement livré et FP-003 comme reste D/Q/Zfh |
| CMD-001A/B/C, MON-001A/B, REG-001A..D | livré par sous-tranches | tests `luna-monitor` et commandes host/backend | clôturer les agrégats après matrice de couverture |
| UI-000A..H | livré | tests app/monitor, TTY et documentation | conserver UI-001 pour l’éditeur/panneaux réellement interactifs |
| FORMAT-001 | livré | conteneurs v4 stricts, checksum, manifeste et replay byte-déterministe | conserver les tests comme garde-fous |
| DBG-001, QUAL-001/002, REL-001 | partiel | débogueur livré par sous-tranches ; fuzzing, performance et release restent à faire | restent sur le chemin critique |

Les tâches BOOT-001 à BOOT-004, GEN-001, ISA-001/002, ASM-BOOT-001,
MEM-001, MACHINE-001 et DEMO-001 sont des entrées de plan initial. Le dépôt
contient déjà une implémentation plus avancée que leur description historique,
mais leur statut ne sera déclaré « livré » qu’après rattachement explicite aux
tests et aux artefacts de la matrice de release. Cette distinction évite de
confondre code présent et exigence formellement auditée.

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

### BOOT-005 — Vérifier la provenance et les champs des extraits R2 — TERMINÉ

- **Jalon / exigences :** M0/M1; ENC-001..006, REQ-PROD-006.
- **But :** rendre reproductible et fail-closed la consommation des extraits R2 avant génération des tables.
- **Non-but :** prétendre avoir comparé la sémantique R1 complète ou ajouter manuellement un opcode.
- **Entrées/sources :** `norms/manifest.toml`; commit R2 complet; extraits `norms/r2/extensions/*`; R2 README sur les champs mask/match.
- **Fichiers/modules :** `crates/isa-core/build.rs`, `norms/r2/SHA256SUMS`, `tools/check-r2.sh`, `TESTS.md`, `README.md`.
- **Étapes réalisées :** valider les plages 0..31, valeurs représentables et chevauchements de champs; propager le SHA R2 dans l’artefact généré; comparer les empreintes et l’ensemble exact des fichiers; déclencher le build du générateur depuis le contrôle.
- **Dépendances et tâches bloquées :** BOOT-003; GEN-001 peut maintenant évoluer sans accepter une entrée corrompue; le contrôle R2↔R1 complet reste à faire dans BOOT-004.
- **Tests :** `bash tools/check-r2.sh`; `cargo test -p luna-isa-core`; mutation manuelle d’un extrait attendue en échec via SHA-256.
- **Critères de sortie :** contrôle vert sur l’arbre propre; commit R2 de 40 caractères exposé par `R2_COMMIT`; toute absence, modification ou incohérence de source provoque une erreur.
- **Cas limites et échecs :** champ inversé, bit >31, valeur hors largeur, champ fixe chevauché, commit court ou fichier ajouté → échec explicite.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible; équivalent indicatif 60k–120k tokens.
- **Compétences/outils :** Rust build scripts, R2, SHA-256, shell reproductible.
- **Parallélisable :** oui avec ASM-001 et FP-001; non avec une modification simultanée de `build.rs`.
- **Paquet de contexte minimal :** SPEC §4/§11, `crates/isa-core/build.rs`, `norms/manifest.toml`, `tools/check-r2.sh`.

### BOOT-006 — Brancher les premiers oracles d’encodage GNU/LLVM — TERMINÉ

- **Jalon / exigences :** M0/M1; ENC-001..006, ISA-001..010, REQ-PROD-006.
- **But :** obtenir une preuve externe reproductible pour un corpus réduit d’encodages R1, en complément des tables R2 et des tests internes.
- **Non-but :** utiliser GNU ou LLVM dans le runtime, prouver toute la sémantique R1, ou valider l’ABI RV64ILP32.
- **Entrées/sources :** R1 Unprivileged ISA v20260120; `norms/oracles/manifest.toml`; corpus `tests/golden/r1-encoding-corpus.tsv`.
- **Fichiers/modules :** `tools/check-oracles.sh`, `norms/oracles/manifest.toml`, `tests/golden/r1-encoding-corpus.tsv`, `TESTS.md`, `TRACEABILITY.md`.
- **Étapes réalisées :** assembler le même corpus avec GNU `as` et LLVM `llvm-mc`; extraire `.text`; comparer les octets à la fixture R1 puis entre oracles; refuser les versions d’outils non déclarées.
- **Dépendances et tâches bloquées :** BOOT-005; le différentiel complet par extension et la sémantique Sail/Spike restent à réaliser.
- **Tests :** `bash tools/check-oracles.sh`; échec attendu si une version, une fixture ou un octet diverge.
- **Critères de sortie :** sept encodages I/U, `ld/sd`, F et D concordent avec GNU 2.44, LLVM 22.1.8 et le corpus R1.
- **Cas limites et échecs :** oracle absent ou version différente → échec explicite; pseudo-instruction ou alias non inclus dans le corpus; divergence classée oracle/norme, jamais masquée.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible; équivalent indicatif 60k–120k tokens.
- **Compétences/outils :** GNU as/objdump, LLVM MC, formats ELF/text, RISC-V encodings.
- **Parallélisable :** oui avec ASM-001 et FP-ORACLE-001; non avec une modification simultanée du corpus ou du script oracle.
- **Paquet de contexte minimal :** SPEC §23, TEST_PLAN §§3–5, `norms/oracles/manifest.toml`, `tools/check-oracles.sh`.

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

### GEN-002 — Publier les métadonnées profile-aware de la table générée — TERMINÉ

- **Jalon / exigences :** M1; ENC-001..006, ISA-001..010, REQ-PROD-006.
- **But :** rendre explicites dans l’artefact généré la largeur 16/32 bits, l’extension source, le nombre d’entrées et l’empreinte de la table.
- **Non-but :** déclarer qu’une instruction est exécutable simplement parce qu’elle est présente dans R2 ; la matrice parse/assemble/execute reste séparée.
- **Entrées/sources :** extraits R2 épinglés, profil `rv64imafd_zicsr_zifencei` avec C décodable mais émission désactivée, SPEC §§5/11/23.
- **Fichiers/modules :** `crates/isa-core/build.rs`, `crates/isa-core/src/lib.rs`, `crates/isa/src/lib.rs`, `tools/check-r2.sh`, `TESTS.md`.
- **Étapes réalisées :** ajouter `instruction_bits` à chaque opcode; générer `GeneratedExtension`; compter les entrées; calculer `R2_OPCODE_TABLE_SHA256`; vérifier l’artefact depuis le script de contrôle.
- **Dépendances et tâches bloquées :** BOOT-005; le générateur complet des pseudos/imports et la comparaison exhaustive R1 restent dans GEN-001/BOOT-004.
- **Tests :** `cargo test -p luna-isa-core -p luna-isa`; `bash tools/check-r2.sh`; vérification de longueur SHA et d’idempotence de génération.
- **Critères de sortie :** tous les opcodes générés portent une largeur valide; chaque extension sélectionnée a un compte non nul; le hash déclaré égale le hash recalculé de la table.
- **Cas limites et échecs :** artefact absent, hash divergent, extension sans entrée ou largeur autre que 16/32 → échec.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne; équivalent indicatif 80k–160k tokens.
- **Compétences/outils :** générateur Rust, SHA-256, ISA C/32 bits, Cargo build scripts.
- **Parallélisable :** oui avec ASM-001 et FP-001; non avec une modification simultanée de l’artefact généré.
- **Paquet de contexte minimal :** `crates/isa-core/build.rs`, `crates/isa-core/src/lib.rs`, `tools/check-r2.sh`, TEST_PLAN §3.

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

### GUEST-001 — Couvrir le moniteur interne par un E2E UART QEMU — TERMINÉ

- **Jalon / exigences :** M2/M5/M7; REQ-PROD-001..005, ISA-001..003, FP-001..006, DBG-001..006, ISO-001..004.
- **But :** vérifier dans une image bare-metal réelle la chaîne source → assemblage invité → mémoire cible → pas-à-pas → registres, avec les opérations flottantes et les transferts de bits déjà exposés par le backend.
- **Non-but :** livrer l’interface terminal complète, les snapshots invités, le backend GDB externe ou le support de toute la grammaire assembleur hôte.
- **Entrées/sources :** SPEC §§5, 8, 10, 14, 16, 18, 23 et 24; R1 Unprivileged ISA; R2 commit épinglé et artefacts générés; `docs/TUTORIAL-GUEST.md`.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-monitor.sh`, `docs/TESTS.md`, `docs/TUTORIAL-GUEST.md`.
- **Étapes réalisées :** parser et désassembleur invités pour `fmv.w.x`, `fmv.x.w`, `fmv.d.x` et `fmv.x.d` via `GENERATED_OPCODES`; scénario UART avec adresses découvertes par `nm`; contrôles des branches, breakpoints, édition/annulation mémoire, directives exactes, `fadd.s`, `fadd.d`, NaN-boxing et transferts de bits.
- **Dépendances et tâches bloquées :** image guest et artefacts R2 existants; la parité complète des commandes, les watches et les snapshots restent différés.
- **Tests :** `bash scripts/test-guest-monitor.sh`; le script compile l’image, lance QEMU avec timeout borné et vérifie les sorties UART et motifs de registres exacts.
- **Critères de sortie :** le script passe depuis un arbre propre sans adresse codée en dur; huit instructions source sont assemblées; `f3`, `f6`, `f7`, `x8` et `x1` correspondent aux motifs attendus; l’annulation restaure chaque écriture testée.
- **Cas limites et échecs :** fenêtre de travail hors RAM refusée; instruction sautée non exécutée; source non supportée diagnostiquée; timeout ou absence de symbole provoque un échec non ambigu.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible; équivalent indicatif 60k–120k tokens.
- **Compétences/outils :** Rust `no_std`, RISC-V F/D, QEMU virt, UART, `nm`, shell POSIX.
- **Parallélisable :** oui avec la documentation et les oracles; non avec une modification simultanée du protocole UART guest.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-monitor.sh`, `docs/TESTS.md`, `docs/TUTORIAL-GUEST.md`, SPEC §§8/10/16/18/24.

### GUEST-002 — Modifier de manière contrôlée les registres entiers du guest — TERMINÉ

- **Jalon / exigences :** M7; DBG-001..004, ABI-001..008, REQ-PROD-003.
- **But :** exposer `set xN <hex64>` dans le moniteur M-mode arrêté afin de préparer des cas ILP32 et de déboguer sans perdre les bits hauts du registre RV64.
- **Non-but :** convertir automatiquement une valeur en pointeur, modifier `x0`, écrire les registres pendant l’exécution ou fournir encore les alias ABI textuels.
- **Entrées/sources :** SPEC §§5, 7, 8, 15 et 16; règles RISC-V sur `x0` et extension de signe ILP32; `TargetContext` du contrat guest.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-monitor.sh`, `docs/TESTS.md`, `docs/TUTORIAL-GUEST.md`.
- **Étapes réalisées :** parser `set`; validation de l’arrêt sur breakpoint; parsing hexadécimal 64 bits; refus explicite de `x0`; écriture du contexte sauvegardé; affichage exact et E2E UART avec motif haut/bas.
- **Dépendances et tâches bloquées :** GUEST-001; la modification des registres flottants existante est réutilisée; l’exécution bornée et les watches restent différés.
- **Tests :** `bash scripts/test-guest-monitor.sh`; vérification de `x9=0x8000000080000000` et de l’erreur `x0 is read-only`.
- **Critères de sortie :** depuis l’arrêt initial, `set x9` restitue exactement 64 bits; aucune mutation n’est observée après le refus de `set x0`; la compilation bare-metal et l’E2E passent.
- **Cas limites et échecs :** commande en cours d’exécution refusée; registre invalide, valeur non hexadécimale ou overflow rejeté sans mutation.
- **Taille :** 2 points / 1 journée-agent, incertitude faible; équivalent indicatif 40k–80k tokens.
- **Compétences/outils :** Rust `no_std`, contexte de trap RISC-V, QEMU UART, tests shell.
- **Parallélisable :** oui avec l’archivage Zfh/Q; non avec une modification concurrente de la grammaire UART guest.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-monitor.sh`, `docs/TUTORIAL-GUEST.md`, SPEC §§7/8/15/16.

### GUEST-003 — Diagnostiquer les erreurs de source multi-ligne — TERMINÉ

- **Jalon / exigences :** M3/M7; REQ-PROD-004, DIAG-001..006, ISO-001.
- **But :** rendre les erreurs de `assemble-program` identifiables par code stable et numéro de ligne dans le moniteur exécuté en M-mode.
- **Non-but :** édition persistante du source, expressions générales, suggestions automatiques et modification partielle de la cible.
- **Entrées/sources :** SPEC §§11/17–19/24; contrat guest 4B; `crates/guest-monitor/src/main.rs`.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-monitor.sh`, `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** format `error [GUEST-ASM-NNN]`; erreurs de labels, syntaxe, workspace et écriture codées; erreur d’instruction associée à la ligne source; validation en deux phases conservée avant toute écriture.
- **Dépendances et tâches bloquées :** GUEST-001/002; navigation et correction source restent à porter ultérieurement.
- **Tests :** programme valide chargé, programme invalide à la ligne 2, présence de `GUEST-ASM-008`, ancien mot toujours désassemblable après rejet.
- **Critères de sortie :** une erreur multi-ligne expose code + ligne; aucune écriture de mot avant validation complète; `bash scripts/test-guest-monitor.sh` passe.
- **Cas limites et échecs :** programme vide, trop long, label invalide, dépassement workspace, collision breakpoint et écriture impossible ont des codes distincts.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** Rust `no_std`, UART, assemblage en deux passes, QEMU.
- **Parallélisable :** oui avec la spécification de l’éditeur source; non avec une modification concurrente du protocole d’erreur UART.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-monitor.sh`, `docs/TUTORIAL-GUEST.md`, SPEC §§11/17–19.

### GUEST-004 — Exécution guest bornée — TERMINÉ

- **Jalon / exigences :** M7; REQ-PROD-002/003, DBG-001..014, ISO-001..004.
- **But :** exécuter au plus N instructions depuis un arrêt guest et revenir au prompt lorsque le budget est épuisé.
- **Non-but :** exécution asynchrone, conditions de breakpoint, watchpoints matériels et step-over ABI complet.
- **Entrées/sources :** SPEC §§6/8/16/18/24; contrat `StopReason`; mécanisme guest de breakpoint temporaire.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-run.sh`, `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** factoriser la reprise d’une instruction; ajouter `run <count>`; porter le budget dans le trap handler; arrêter sur breakpoint permanent, `ebreak` réel ou trap non-borne; refuser `0` et les budgets supérieurs à 100000.
- **Dépendances et tâches bloquées :** GUEST-001/003; les watchpoints nécessitent encore des événements d’accès mémoire ou une instrumentation dédiée.
- **Tests :** programme de deux `addi`, `run 2`, vérification `x1=2`, budget nul et budget excessif; QEMU timeout borné.
- **Critères de sortie :** aucun run ne dépasse son budget; l’état est observable au prompt; les erreurs de budget ne modifient pas la cible.
- **Cas limites et échecs :** budget nul, dépassement, breakpoint permanent rencontré, instruction `ebreak`, boucle de contrôle et trap illégal.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** trap M-mode, breakpoints logiciels, Rust `no_std`, QEMU UART.
- **Parallélisable :** oui avec le design des watchpoints; non avec une modification concurrente de `rust_trap` ou des breakpoints temporaires.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-run.sh`, `docs/TUTORIAL-GUEST.md`, SPEC §§8/16/18.

### GUEST-005 — Watchpoints logiciels guest sur ld/sd — TERMINÉ

- **Jalon / exigences :** M7; REQ-PROD-003, DBG-001..014, ISO-001..004.
- **But :** arrêter avant un accès `ld` ou `sd` qui recouvre une plage surveillée dans le programme U-mode.
- **Non-but :** watchpoints matériels QEMU, accès MMIO, instructions atomiques, conditions d’expression et surveillance des autres largeurs.
- **Entrées/sources :** SPEC §§6/8/14/16/18/24; encodages R1/R2 des loads/stores RV64I.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-watchpoint.sh`, `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** table de quatre watchpoints; commandes `watch`, `rwatch`, `awatch`, `info watch`, `delete watch`; calcul de l’adresse effective RV64; détection de recouvrement avant exécution; arrêt et remise à zéro du budget `run`.
- **Dépendances et tâches bloquées :** GUEST-004; accès autres que `ld`/`sd`, watchpoints matériels et conditions restent différés.
- **Tests :** programme `addi` puis `sd`, watchpoint écriture sur données, arrêt avant le store, mémoire inchangée, affichage et suppression.
- **Critères de sortie :** un store surveillé ne modifie pas la mémoire avant le prompt; les modes read/write/access sont distincts; largeur 1..8 et RAM cible validées.
- **Cas limites et échecs :** table pleine, plage hors RAM, overflow, largeur nulle/supérieure à 8, numéro invalide et absence de watchpoint.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** décodage RV64I, calcul d’adresse effective, Rust `no_std`, QEMU UART.
- **Parallélisable :** oui avec la conception des watchpoints host; non avec une modification concurrente du trap handler guest.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-watchpoint.sh`, `docs/TUTORIAL-GUEST.md`, SPEC §§8/14/16/18.

### GUEST-006 — Source guest persistant et réassemblage explicite — TERMINÉ

- **Jalon / exigences :** M3/M6/M7; REQ-PROD-003/004, REQ-OBS-001, ISO-001.
- **But :** conserver le dernier programme source validé, le consulter, corriger une ligne sans mutation et réassembler explicitement cette version.
- **Non-but :** éditeur plein écran, macros, expressions générales, historique des versions et sauvegarde persistante.
- **Entrées/sources :** SPEC §§11/17–19/21; contrat guest 4B; diagnostics `GUEST-ASM-*`.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-source.sh`, `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** buffer borné de 256 lignes de 128 caractères; commandes `source`, `source <n>`, `source replace <n> <text>`, `assemble-source`; conservation de l’adresse; validation atomique et texte quoté; scratch d’assemblage statique hors pile M-mode.
- **Dépendances et tâches bloquées :** GUEST-003; persistance projet, navigation clavier et undo source restent différés.
- **Tests :** assembler deux lignes, consulter/corriger la ligne 2, constater l’absence d’effet avant `assemble-source`, réassembler puis vérifier `x1=6` après deux pas.
- **Critères de sortie :** `source replace` n’écrit jamais la RAM; `assemble-source` applique uniquement le buffer validé; les lignes et erreurs sont lisibles par UART.
- **Cas limites et échecs :** source vide, ligne hors plage, remplacement trop long, document absent et `assemble-source` sans source.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** Rust `no_std`, buffers statiques, assemblage deux passes, QEMU UART.
- **Parallélisable :** oui avec l’éditeur terminal host; non avec une modification concurrente du buffer source guest.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-source.sh`, `docs/TUTORIAL-GUEST.md`, SPEC §§11/17–19.

### GUEST-007 — Snapshot et projet guest volatils — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** sauvegarder et restaurer pendant la session QEMU l’état U-mode, les régions cible utilisées, le source et les arrêts du moniteur.
- **Non-but :** fichier hôte, format de flux persistant, plusieurs slots, restauration après reset QEMU ou capture des 64 MiB complets.
- **Entrées/sources :** SPEC §§8/12/18/21/22; carte guest 4B; contrat snapshot versionné adapté au stockage bare-metal.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-snapshot.sh`, `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** slot RAM volatil; capture workspace 64 KiB, données 1 MiB, `TargetContext`, source, symboles, breakpoints et watchpoints; restauration des régions et réapplication des breakpoints; commandes `snapshot save|restore` et alias `project-save|project-load`.
- **Dépendances et tâches bloquées :** GUEST-005/006; format de projet persistant, reprise après reset et snapshots multi-slots restent différés.
- **Tests :** sauvegarde avec `x1=7` et un mot de données, mutations registre/mémoire/source, restauration et vérification des trois valeurs; alias projet vérifié.
- **Critères de sortie :** la restauration retrouve l’état capturé dans les régions bornées; une absence de snapshot est diagnostiquée; le reset QEMU invalide implicitement le slot.
- **Cas limites et échecs :** snapshot absent, breakpoint hors régions capturées, source vide, restauration avec watchpoints et budget actif.
- **Taille :** 6 points / 3 journées-agent, incertitude élevée.
- **Compétences/outils :** Rust `no_std`, linker/RAM, copie mémoire sûre, état de trap, QEMU.
- **Parallélisable :** oui avec le format projet host; non avec une modification concurrente de la carte mémoire guest ou du trap handler.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-snapshot.sh`, `docs/TUTORIAL-GUEST.md`, SPEC §§8/12/18/21.

### GUEST-008 — Transport UART de snapshot par blocs — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** rendre un slot guest inspectable et modifiable à distance sans supposer un système de fichiers dans la machine bare-metal.
- **Non-but :** persistance hôte, compression, checksum de flux complet, plusieurs slots ou modification directe de la cible active.
- **Entrées/sources :** SPEC §§12/18/21/22; contrat guest 4B; protocole UART du tutoriel guest.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-snapshot.sh`, `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** `snapshot info`; `snapshot manifest`; `snapshot dump <region> <offset> <length>` jusqu’à 4096 octets; `snapshot patch <region> <offset> <hex>` jusqu’à 32 octets; régions workspace/data bornées; CRC-32 IEEE par région; patch appliqué uniquement au slot jusqu’à `snapshot restore`.
- **Dépendances et tâches bloquées :** GUEST-007; le format persistant RVSNAP/RVPROJ et le transfert complet avec intégrité restent différés.
- **Tests :** QEMU sauvegarde un mot little-endian, vérifie deux manifestes et le changement de CRC après patch, le lit, le patche dans le slot, refuse les chunks invalides, restaure, puis vérifie registre, mémoire, source et alias projet.
- **Critères de sortie :** commande valide produisant une réponse déterministe; refus des régions, offsets, longueurs et hexadécimaux invalides; aucune mutation de la RAM active avant restauration.
- **Cas limites et échecs :** snapshot absent, offset hors région, chunk vide, dump de plus de 4096 octets, patch de plus de 32 octets, hexadécimal impair ou trop long, frontière exacte de région.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** Rust `no_std`, UART, QEMU, tests shell.
- **Parallélisable :** oui avec le format projet host; non avec une modification concurrente de `GuestSnapshot`.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-snapshot.sh`, `docs/TUTORIAL-GUEST.md`, SPEC §§12/18/21.

### GUEST-009 — Manifeste et intégrité des snapshots guest — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** permettre à l’hôte de vérifier qu’un transfert par blocs couvre exactement les deux régions du slot.
- **Non-but :** rendre le guest responsable du stockage hôte, du protocole de reprise ou de la restauration directe d’un fichier.
- **Entrées/sources :** SPEC §§12/18/21/22; profil de transport RVSNAP01; CRC-32 IEEE 802.3.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-snapshot.sh`, `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** commande `snapshot manifest`; tailles et compte source; CRC-32 séparé du workspace et de data; vérification du changement de CRC après patch.
- **Dépendances et tâches bloquées :** GUEST-008; le flux hôte complet, l’accusé de réception par bloc et la persistance restent différés.
- **Tests :** manifestes avant/après modification, CRC distincts, snapshot absent, offset hors région, longueur excessive et chaîne hexadécimale impaire.
- **Critères de sortie :** CRC reproductible sur QEMU; manifestes machine-lisibles; erreurs stables; aucune mutation de la cible active.
- **Cas limites et échecs :** région vide impossible, slot absent, frontière exacte, patch qui ne change pas les données et corruption détectée côté hôte.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** Rust `no_std`, CRC-32, UART, shell, QEMU.
- **Parallélisable :** oui avec le format persistant host; non avec une évolution concurrente de `GuestSnapshot`.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`, `scripts/test-guest-snapshot.sh`, `docs/TUTORIAL-GUEST.md`, SPEC §§12/18/21.

### GUEST-010 — Format hôte RVSNAP01 et collecteur générique — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** fournir une représentation binaire déterministe des régions guest et un collecteur indépendant du transport.
- **Non-but :** connexion UART concrète, persistance des registres/source/symboles, reprise réseau ou remplacement direct de l’état QEMU.
- **Entrées/sources :** SPEC §§12/18/21/22; manifeste `RVSNAP01` guest; contrat de blocs de lecture jusqu’à 4096 octets et patch jusqu’à 32 octets.
- **Fichiers/modules :** `crates/snapshot-format/src/lib.rs`, `crates/snapshot-format/Cargo.toml`, `Cargo.toml`, `Cargo.lock`.
- **Étapes réalisées :** en-tête little-endian de 32 octets; tailles bornées workspace/data; CRC-32 par région; encode/decode strict sans octets résiduels; trait `GuestCommandTransport`; collecteur qui demande le manifeste, récupère tous les blocs et vérifie les CRC.
- **Dépendances et tâches bloquées :** GUEST-008/009; l’adaptateur UART/TCP et l’extension du fichier aux registres, source et symboles restent différés.
- **Tests :** round-trip déterministe, corruption data, troncature, octets résiduels, régions surdimensionnées, collecte multi-blocs et corruption détectée après collecte.
- **Critères de sortie :** un flux de réponses guest valide produit une image identique; tout manifeste ou bloc incohérent est refusé avant export.
- **Cas limites et échecs :** longueur 0 autorisée pour le crate générique mais non produite par le guest, frontière 4096 octets, dernier bloc court, CRC invalide, ordre/région/offset inattendus.
- **Taille :** 5 points / 2,5 journées-agent, incertitude moyenne.
- **Compétences/outils :** Rust stable, formats binaires, CRC-32, protocole texte, tests unitaires.
- **Parallélisable :** oui avec l’adaptateur UART concret; non avec une modification du format RVSNAP01.
- **Paquet de contexte minimal :** `crates/snapshot-format/src/lib.rs`, `crates/guest-monitor/src/main.rs`, docs §§12/18/21.

### GUEST-011 — Adaptateur UART TCP et export hôte — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** relier le collecteur RVSNAP01 à l’UART virtuelle QEMU via TCP et produire un fichier hôte vérifié.
- **Non-but :** GDB RSP, reprise après coupure, import/restauration et transport réseau distant non authentifié.
- **Entrées/sources :** SPEC §§9/12/18/21; protocole UART guest et invite `rvmonitor> `.
- **Fichiers/modules :** `crates/snapshot-format/src/lib.rs`, `crates/app/src/main.rs`, `scripts/test-guest-snapshot-export.sh`, docs.
- **Étapes réalisées :** `TcpGuestCommandTransport`; synchronisation sur l’invite; options `--guest-uart-port` et `--snapshot-out`; `snapshot save` avant collecte; écriture atomique logique après validation du format.
- **Dépendances et tâches bloquées :** GUEST-010; import guest, reprise de transfert et métadonnées de contexte restent différés.
- **Tests :** QEMU réel avec UART TCP, export des 1 114 624 octets, magic `RVSNAP01`, taille exacte et validation CRC pendant la collecte.
- **Critères de sortie :** export non vide, magic et taille attendus, absence d’export si le manifeste ou un bloc est invalide.
- **Cas limites et échecs :** port indisponible, invite absente, EOF UART, réponse trop grande, erreur `snapshot save`, fichier impossible à écrire.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** Rust std, TCP, UART QEMU, formats binaires, shell.
- **Parallélisable :** oui avec l’import; non avec une modification du protocole d’invite guest.
- **Paquet de contexte minimal :** `crates/snapshot-format/src/lib.rs`, `crates/app/src/main.rs`, `scripts/test-guest-snapshot-export.sh`, docs §§9/12/18/21.

### GUEST-012 — Import hôte vers le slot guest — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** restaurer dans le slot guest les régions d’une image RVSNAP01 validée par l’hôte.
- **Non-but :** écrire la RAM active avant `snapshot restore`, importer des registres/source absents du format, optimiser davantage le débit UART ou reprendre une connexion interrompue.
- **Entrées/sources :** SPEC §§12/18/21; RVSNAP01; commandes guest `snapshot patch` et `snapshot restore`.
- **Fichiers/modules :** `crates/snapshot-format/src/lib.rs`, `crates/app/src/main.rs`, `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** `apply_guest_snapshot`; décodage et CRC avant transport; initialisation contrôlée du slot par `snapshot save`; commande `snapshot patchbin` avec payload brut; blocs binaires jusqu’à 4096 octets; détection des réponses guest négatives; restauration finale obligatoire; option `--snapshot-in`.
- **Dépendances et tâches bloquées :** GUEST-010/011; l’import complet QEMU dépasse actuellement 90 secondes sur UART 16550 à cause du débit émulé; reprise, compression/delta et métadonnées restent différées.
- **Tests :** image multi-blocs appliquée par fake guest, ordre patches puis restore, refus de réponse invalide, handshake/payload `patchbin` QEMU sur un bloc via `scripts/test-guest-snapshot-binary.sh`, export QEMU réel et couverture `cargo test --workspace`.
- **Critères de sortie :** fichier invalide rejeté avant connexion; aucun restore si un patch est refusé; image valide confirmée seulement après réponse `snapshot restored`.
- **Cas limites et échecs :** fichier tronqué/corrompu, image vide, patch binaire final court, slot absent, coupure transport et restore refusé.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** Rust, protocole UART, formats binaires, QEMU.
- **Parallélisable :** oui avec delta/reprise; non avec une modification concurrente des bornes guest.
- **Paquet de contexte minimal :** `crates/snapshot-format/src/lib.rs`, `crates/app/src/main.rs`, SPEC §§12/18/21.

### GUEST-014 — FIFO NS16550 et synchronisation UART — TERMINÉ

- **Jalon / exigences :** M8/M9; IO-001..004, OBS-001..003, REQ-ISO-003.
- **But :** activer le tampon RX/TX du NS16550 virtuel pour toutes les
  communications du moniteur guest, en conservant le protocole de transfert
  binaire et les invites comme points de synchronisation.
- **Non-but :** DMA, virtio-console et réglage dynamique d'un débit
  physique ou compression RLE dans cette sous-tranche.
- **Entrées/sources :** QEMU `virt` (NS16550 compatible), contrat UART du
  tutoriel guest, SPEC §§9/18/21.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, scripts E2E UART,
  `docs/TUTORIAL-GUEST.md`, `docs/TESTS.md`.
- **Étapes réalisées :** activation FCR FIFO, seuil RX minimal d'un octet,
  synchronisation des tests pipe après le boot, conservation du handshake
  `snapshot binary ready` pour les payloads bruts.
- **Dépendances :** GUEST-008/011/012; le regroupement logiciel des lectures,
  la compression/delta et la reprise de transfert restent différés.
- **Tests :** tous les scripts guest QEMU pipe, export TCP et handshake
  `snapshot patchbin`, suite workspace et contrôle R2.
- **Critères de sortie :** premier caractère conservé après boot; scripts
  guest verts; aucun changement du format binaire ni de la sémantique des
  commandes; le FIFO est documenté comme tampon matériel, non comme DMA.
- **Cas limites et échecs :** entrée injectée avant l'invite, FIFO vide,
  réponse binaire courte, timeout QEMU, surcharge persistante du polling.
- **Taille :** 2 points / 0,5 journée-agent, incertitude faible.
- **Compétences/outils :** Rust `no_std`, NS16550, QEMU, shell.
- **Parallélisable :** oui avec metadata/projets; non avec une modification
  concurrente du protocole UART guest.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`,
  `scripts/test-guest-*.sh`, `scripts/test-guest-snapshot-binary.sh`,
  `docs/TUTORIAL-GUEST.md`.

### GUEST-015 — Tampon logiciel de réception UART — TERMINÉ

- **Jalon / exigences :** M8/M9; IO-001..004, REQ-PROD-004, REQ-ISO-003.
- **But :** regrouper les octets déjà présents dans le FIFO NS16550 avant leur
  consommation par la console ou par un payload `snapshot patchbin`.
- **Non-but :** modifier la grammaire, ajouter DMA ou
  prétendre fournir une compression.
- **Entrées/sources :** QEMU `virt` NS16550; contrat `snapshot binary ready`;
  SPEC §§9/12/18/21.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, tests QEMU et
  documentation UART.
- **Étapes réalisées :** tampon RX logiciel fixe de 4096 octets; drainage du
  FIFO matériel; polling opportuniste pendant les sorties M-mode et service
  IRQ via PLIC pendant l'exécution U-mode; réutilisation par `uart_read_line`
  et la réception binaire.
- **Dépendances :** GUEST-014; compression RLE/delta, reprise et débit
  instrumenté restent différés.
- **Tests :** scénarios console pipe, snapshot binaire TCP, export TCP, suite
  workspace et contrôle R2.
- **Critères de sortie :** ordre conservé dans les limites du tampon, compteur
  d'overflow observable, payload de longueur exacte accepté et mêmes réponses
  fonctionnelles qu'avant.
- **Cas limites et échecs :** tampon plein, FIFO vide, payload supérieur à
  4096 octets, EOF/timeout, octets de commandes préchargés et interruption
  pendant une cible U-mode.
- **Taille :** 3 points / 1 journée-agent, incertitude moyenne.
- **Compétences/outils :** Rust `no_std`, NS16550, PLIC, traps RISC-V, QEMU,
  tests shell.
- **Parallélisable :** oui avec l'intégration metadata; non avec une autre
  modification des primitives UART.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`,
  `scripts/test-guest-snapshot-binary.sh`, `docs/TUTORIAL-GUEST.md`.

### GUEST-016 — Compression RLE optionnelle des patches guest — TERMINÉ

- **Jalon / exigences :** M8/M9; IO-004, IO-008, REQ-PROD-004/006.
- **But :** réduire le volume des blocs répétitifs pendant l'import de
  snapshots, sans imposer de compression aux données non compressibles.
- **Non-but :** chiffrement, reprise de session, compression générale du
  conteneur RVSNAP ou suppression du chemin `patchbin`.
- **Entrées/sources :** SPEC §§12/18/21; contrat UART guest; format RVSNAP01.
- **Fichiers/modules :** `crates/snapshot-format/src/lib.rs`,
  `crates/guest-monitor/src/main.rs`, script de smoke TCP et documentation.
- **Étapes réalisées :** encodeur déterministe par paires `(run, octet)` avec
  runs de 1..255; choix host RLE seulement si plus court; commande guest
  `snapshot patchrle`; décodage borné dans un tampon séparé; chunks de 4096
  octets avec repli automatique sur `patchbin`.
- **Dépendances :** GUEST-015; instrumentation de débit, delta inter-snapshot
  et reprise restent différés.
- **Tests :** round-trip RLE, fallback incompressible, patch RLE guest de 300
  octets, vérification du dump, raw `patchbin`, suite workspace et R2.
- **Critères de sortie :** aucun dépassement de tampon, longueur décodée exacte,
  ordre des octets conservé, import compatible avec un guest sans RLE seulement
  après négociation explicite (non activée dans cette version).
- **Cas limites et échecs :** run de 255, longueur impaire, expansion trop
  grande, longueur brute incohérente, chunk non compressible et zone hors
  bornes.
- **Taille :** 5 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** Rust `std`/`no_std`, protocole binaire, QEMU, tests
  Python et shell.
- **Parallélisable :** oui avec metadata; non avec une modification concurrente
  du framing `command_binary`.
- **Paquet de contexte minimal :** `crates/snapshot-format/src/lib.rs`,
  `crates/guest-monitor/src/main.rs`, `scripts/test-guest-snapshot-binary.sh`.

### GUEST-017 — Exposition guest du metadata RVMETA01 — TERMINÉ

- **Jalon / exigences :** M8; IO-006..009, OBS-001..006, REQ-PROD-006.
- **But :** rendre le contexte, le source et les symboles du slot snapshot
  lisibles depuis le guest avec le format `RVMETA01` déjà testé côté hôte.
- **Non-but :** modifier encore le conteneur `RVSNAP01`, importer le metadata,
  persister les historiques ou ajouter une négociation de capacités.
- **Entrées/sources :** SPEC §§8/12/21/22; contrat `RVMETA01` de GUEST-013.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, smoke snapshot et
  tutoriel/tests guest.
- **Étapes réalisées :** sérialisation little-endian bornée depuis le slot;
  commandes `snapshot metadata` et `snapshot metadata dump`; chunks de 128
  octets en hexadécimal; validation de la magic et de la longueur dans QEMU.
- **Dépendances :** GUEST-013/016; collecte hôte et intégration au fichier
  persistent restent la prochaine sous-tranche.
- **Tests :** snapshot QEMU sauvegardé, annonce `RVMETA01`, premier chunk
  `52564d4554413031`, suite workspace et contrôles R2.
- **Critères de sortie :** metadata absent refusé, buffer borné, octets exacts
  et ordre stable, aucune modification des régions snapshot.
- **Cas limites et échecs :** source/symboles vides, metadata trop grand,
  plage inversée ou hors limites, chunk de 128 octets.
- **Taille :** 4 points / 1 journée-agent, incertitude moyenne.
- **Compétences/outils :** Rust `no_std`, formats binaires, QEMU, UART.
- **Parallélisable :** oui avec l’intégration hôte; non avec une modification
  concurrente du contrat `RVMETA01`.
- **Paquet de contexte minimal :** `crates/guest-monitor/src/main.rs`,
  `crates/snapshot-format/src/lib.rs`, `scripts/test-guest-snapshot.sh`.

### GUEST-018 — Collecte host et conteneur RVPROJ01 — TERMINÉ

- **Jalon / exigences :** M8; IO-006..009, OBS-001..006, REQ-PROD-006.
- **But :** collecter le metadata guest et l’associer à l’image mémoire dans
  un projet hôte déterministe.
- **Non-but :** importer le metadata dans le guest, modifier `RVSNAP01` ou
  persister l’historique et les points d’arrêt dans cette tranche.
- **Entrées/sources :** SPEC §§12/21/22; contrats `RVSNAP01` et `RVMETA01`.
- **Fichiers/modules :** `crates/snapshot-format/src/lib.rs`,
  `crates/app/src/main.rs`, script d’export TCP et documentation.
- **Étapes réalisées :** collecte/validation des manifestes metadata et chunks;
  conteneur `RVPROJ01` version 1 avec longueurs strictes, image `RVSNAP01`
  et metadata `RVMETA01`; option CLI `--project-out`.
- **Dépendances :** GUEST-017; application guest et migration de projets
  existants restent différées.
- **Tests :** round-trip `RVPROJ01`, export QEMU image + projet, magics,
  tailles, workspace et contrôle R2; import host couvert par fake transport.
- **Critères de sortie :** un projet exporté se décode bit-exactement; un
  metadata tronqué ou une image incohérente est refusé avant écriture utile.
- **Cas limites et échecs :** metadata vide/tronqué, magic/version inconnue,
  longueur avec overflow, octets résiduels et symbole/source bornés.
- **Taille :** 5 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** Rust std, formats binaires, TCP, QEMU.
- **Parallélisable :** oui avec l’import guest; non avec une évolution du layout
  `RVPROJ01`.
- **Paquet de contexte minimal :** `crates/snapshot-format/src/lib.rs`,
  `crates/app/src/main.rs`, `scripts/test-guest-snapshot-export.sh`.

### GUEST-019 — Import guest du projet RVPROJ01 — IMPLÉMENTÉ, E2E DIFFÉRÉ

- **Jalon / exigences :** M8; IO-006..009, OBS-001..006.
- **But :** décoder un projet hôte, préparer le slot mémoire, appliquer le
  metadata par handshake binaire et restaurer en dernier.
- **Non-but :** optimiser la copie des 1 MiB du slot ou promettre un temps
  d’import interactif sur UART 16550 émulée.
- **Entrées/sources :** contrats `RVPROJ01`, `RVSNAP01`, `RVMETA01`.
- **Fichiers/modules :** `crates/snapshot-format/src/lib.rs`,
  `crates/app/src/main.rs`, `crates/guest-monitor/src/main.rs`.
- **Étapes réalisées :** `apply_guest_project`, `--project-in`, commande guest
  `snapshot metadata apply <length>`, validation complète avant mutation du
  slot et restauration finale unique.
- **Dépendances :** GUEST-018; le smoke QEMU complet reste différé car la
  copie/restauration des régions fixes de 1 MiB dépasse le budget d’exécution
  CI; le contrat est couvert par tests de format et fake transport.
- **Tests :** round-trip projet, application host fake dans l’ordre patches →
  metadata → restore, compilation guest; aucun E2E QEMU complet déclaré vert.
- **Critères de sortie :** payload invalide refusé sans restauration; metadata
  valide appliqué au slot; restauration demandée seulement après confirmation.
- **Cas limites et échecs :** metadata tronqué, source trop longue, symbole
  hors limite, longueur résiduelle et coupure avant restore.
- **Taille :** 5 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** Rust std/no_std, protocoles binaires, QEMU.
- **Parallélisable :** oui avec UI/profil; non avec une modification concurrente
  du layout `RVPROJ01`.
- **Paquet de contexte minimal :** `crates/snapshot-format/src/lib.rs`,
  `crates/guest-monitor/src/main.rs`, `crates/app/src/main.rs`.

### GUEST-013 — Contrat metadata RVSNAP01 — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** figer une section metadata versionnée pour persister le contexte RV64, le source et les symboles sans dépendre du transport.
- **Non-but :** modifier encore le fichier principal, lire les métadonnées par UART ou sérialiser les breakpoints/watchpoints dans cette première sous-tranche.
- **Entrées/sources :** SPEC §§8/12/21/22; `TargetContext`; contrat guest 4B.
- **Fichiers/modules :** `crates/snapshot-format/src/lib.rs`, `docs/TESTS.md`, `docs/TUTORIAL-GUEST.md`.
- **Étapes réalisées :** magic/version `RVMETA01`; 32 registres `x`, 32 registres `f`, PC, `fcsr`, `mstatus`, `mepc`, `mcause`, `mtval`; source borné à 1536 octets; huit symboles de 64 octets maximum; encode/decode little-endian strict.
- **Dépendances et tâches bloquées :** GUEST-010/012; intégration au conteneur RVSNAP01, commandes metadata guest, breakpoints/watchpoints et migration restent à faire.
- **Tests :** round-trip bit-exact du contexte/source/symboles; magic invalide; source surdimensionné; suite workspace.
- **Critères de sortie :** même structure et mêmes bits après decode; limites refusées avant allocation non bornée; version inconnue diagnostiquée.
- **Cas limites et échecs :** source vide, symbole au nom maximal, contexte NaN-boxé, payload tronqué, version future, octets résiduels.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** formats binaires, ABI RV64, Rust.
- **Parallélisable :** oui avec l’adaptateur de commandes guest; non avec une modification concurrente de la représentation metadata.
- **Paquet de contexte minimal :** `crates/snapshot-format/src/lib.rs`, `crates/target-api/src/lib.rs`, SPEC §§8/12/21.

## M3 — assembleur et désassembleur

### ISA-003 — Étendre la tranche RV64 aux loads/stores 64 bits — TERMINÉ

- **Jalon / exigences :** M2; ISA-001, MEM-001..004, REQ-PROD-001/002.
- **But :** supporter `ld` et `sd` de bout en bout dans le profil RV64 : encodage généré, décodage, mémoire little-endian, exécution, événements de largeur 8 octets et désassemblage.
- **Non-but :** bascule big-endian, MMU ou compatibilité ELF externe.
- **Entrées et sources :** R1 RV64I Load/Store; R2 `rv64_i`; SPEC §§5–8 et 23.
- **Fichiers/modules :** `crates/isa-core`, `crates/isa`, `crates/memory`, `crates/machine`, `crates/assembler`, `crates/disassembler`, `crates/guest-monitor`, linker et smoke test guest.
- **Étapes réalisées :** ajouter `encode_load/store` au cœur sans-std; ajouter les variantes `Ld/Sd`; sérialiser `u64` LE; exposer `MemoryAccess.width=8`; étendre assembleur/désassembleur; porter le parseur guest; réduire le workspace linker pour conserver un binaire guest dans 128 KiB.
- **Dépendances :** GEN-001 existant; aucune dépendance BE ou ELF externe.
- **Tests :** golden `ld x3,-8(x4)`/`sd x3,8(x4)`; round-trip assemble→decode→désassemble; mémoire LE et bornes; machine et événements watchpoint; `bash scripts/test-guest-monitor.sh`.
- **Critères de sortie :** tests ciblés verts, compilation `riscv64gc-unknown-none-elf` verte, smoke QEMU vert et sortie guest contenant `ld x3,-8(x4)`.
- **Cas limites :** immédiat signé 12 bits, adresse non mappée, débordement de plage 8 octets, `x0` destination inchangée.
- **Taille :** 4 points / 2 journées-agent, réalisée ; équivalent indicatif 80k–160k tokens.
- **Compétences/outils :** encodage RISC-V, Rust no-std, QEMU.
- **Parallélisable :** oui avec FP-004 et GEN-001 ; intégration guest séquentielle.
- **Paquet de contexte minimal :** SPEC §§5–8/23, `crates/isa-core/src/lib.rs`, `crates/isa/src/lib.rs`, `scripts/test-guest-monitor.sh`.

### ISA-004 — Construire une adresse de données distante dans le guest — TERMINÉ

- **Jalon / exigences :** M2/M3; ABI-001..004, ISA-001, MEM-001..004, DBG-001.
- **But :** valider sous QEMU le chemin guest `auipc → sd → ld` entre le workspace de code `0x81000000` et la zone de données `0x82000000`, sans masquer la règle RV64 de signe-extension.
- **Non-but :** ajouter des alias mémoire, modifier la sémantique ILP32 ou fournir un linker ELF externe.
- **Entrées et sources :** R1 RV64I, U-type et Loads/Stores; R2 commit généré; SPEC §§5–8/13/23; décision locale sur la carte 64 MiB.
- **Fichiers/modules :** `crates/isa-core`, `crates/isa`, `crates/assembler`, `crates/machine`, `crates/disassembler`, `crates/guest-monitor`, `docs/TUTORIAL-GUEST.md`, `scripts/test-guest-ld-sd.sh`.
- **Étapes réalisées :** généraliser l’encodage U-type généré à `lui`/`auipc`; décoder, désassembler et exécuter `auipc`; porter le parseur guest; construire l’adresse distante depuis le PC; écrire/lire 64 bits dans la zone data; vérifier les registres et octets via QEMU.
- **Dépendances et tâches bloquées :** ISA-003 et carte guest 64 MiB; aucune dépendance à l’interface terminale; ce test doit rester vert pendant M6–M8.
- **Tests :** golden `auipc`; round-trip encode/decode/disassemble; test machine PC-relative; compilation RV64 bare-metal; `bash scripts/test-guest-ld-sd.sh`; suite workspace.
- **Critères de sortie :** le guest assemble quatre instructions, `x4=0x0000000082000000`, `x5=42`, et affiche `2a 00 00 00 00 00 00 00` à `0x82000008`.
- **Cas limites et échecs :** `lui x4,0x82000` reste correctement signe-étendu en `0xffffffff82000000`; accès hors `[0x80000000,0x84000000)` → faute; PC relatif incorrect → adresse distante non validée.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible; équivalent indicatif 60k–120k tokens.
- **Compétences/outils :** encodage U-type, sémantique RV64, Rust no-std, linker, QEMU.
- **Parallélisable :** oui avec FP-004 et GEN-001; non avec une autre modification du parseur guest ou du linker sans contrat de fusion.
- **Paquet de contexte minimal :** SPEC §§5–8, `crates/isa-core/src/lib.rs`, `crates/machine/src/lib.rs`, `scripts/test-guest-ld-sd.sh`.

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

### ASM-001A — Stabiliser le lexer de la tranche M3 — TERMINÉ

- **Jalon / exigences :** M3; ASM-001..004, ASM-009, ASM-015, REQ-PROD-004.
- **But :** fournir au parseur des tokens distincts pour registres, nombres, chaînes et opérateurs de décalage, avec commentaires de ligne et positions diagnostiques stables.
- **Non-but :** normalisation NFC, identifiants Unicode généraux, macros et émission binaire.
- **Entrées/sources :** SPEC §11 (lexique, commentaires, registres, expressions) et §19 (diagnostics); R4 pour les formes de registres et expressions.
- **Fichiers/modules :** `crates/asm-lexer/src/lib.rs`, `crates/assembler/src/parser.rs`, `crates/diag/src/lib.rs`.
- **Étapes réalisées :** classification case-insensitive des registres `xN`/`fN` et alias ABI; commentaires `#`, `;`, `//` limités à la ligne; `<<`/`>>`; longueurs de spans en scalaires Unicode; longueur optionnelle des diagnostics; raccordement parser aux nouveaux tokens.
- **Dépendances :** aucune nouvelle dépendance externe; bloque les évolutions parser qui supposeraient encore que tout registre est un `Identifier`.
- **Tests :** cinq tests lexer et un test parser pour commentaires multilignes, alias, nombres, décalages, UTF-8, chaînes incomplètes et caractères invalides; `cargo test -p luna-asm-lexer -p luna-assembler`.
- **Critères de sortie :** les tests ciblés sont verts; `ADDI X1,A0,1` et `imm(sp)` sont reconnus; une erreur lexicale expose code, ligne, colonne et longueur; le commentaire n’empêche pas la ligne suivante d’être tokenisée.
- **Cas limites et échecs :** chaîne traversant une ligne, échappement inconnu, caractère inattendu, registre hors plage reste un identifiant et est rejeté plus loin par le parseur/assembleur.
- **Taille :** 3 points / 1–1,5 journée-agent, incertitude faible; équivalent indicatif 40k–100k tokens.
- **Compétences/outils :** Rust, lexing Unicode, diagnostics positionnés.
- **Parallélisable :** oui avec DIS-001 et FP-001; non avec une modification concurrente de `TokenKind` ou de l’API `Diagnostic`.
- **Paquet de contexte minimal :** SPEC §§11/19, `crates/asm-lexer/src/lib.rs`, `crates/assembler/src/parser.rs`, `crates/diag/src/lib.rs`.

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

### ASM-002A — Stabiliser les portées de symboles et les littéraux — TERMINÉ

- **Jalon / exigences :** M3; ASM-001, ASM-002, ASM-005, ASM-009, ASM-015.
- **But :** rendre la collecte de symboles déterministe sur deux passes, avec labels locaux `.L*` liés au dernier label global, détection des doublons et séparateurs de chiffres conformes à la grammaire.
- **Non-but :** relocations ELF exportables, `.equ`, macros et relaxation.
- **Entrées/sources :** SPEC §11 (labels, expressions, deux passes) et §19 (codes d’erreur); R4, sections expressions/relocations et syntaxe des littéraux.
- **Fichiers/modules :** `crates/assembler/src/expr.rs`, `crates/assembler/src/lib.rs`, `crates/asm-lexer/src/lib.rs`, `TESTS.md`.
- **Étapes réalisées :** séparer les clés internes `global::.Llocal`; maintenir la portée pendant les passes; fournir les alias locaux au contexte d’évaluation; refuser un local avant tout global et les doublons; accepter `_` et `'` dans les entiers décimaux et basés.
- **Dépendances :** ASM-001A; aucun nouvel outil externe; bloque les relocations locales qui supposeraient une table de symboles non déterministe.
- **Tests :** précédence et littéraux séparés; réutilisation d’un même `.Ldone` sous deux globaux; local sans portée; doublon global; suite assembleur complète.
- **Critères de sortie :** les mêmes bytes sont produits sur les références globales et locales; deux portées locales ne se collisionnent pas; les erreurs `ASM-LABEL-001/002` sont stables; le total de tests documenté correspond à l’exécution Cargo.
- **Cas limites et échecs :** local au début du fichier, local dupliqué dans une portée, symbole inconnu en seconde passe, débordement d’expression et séparation mal placée restent des erreurs explicites.
- **Taille :** 4 points / 1,5–2 journées-agent, incertitude moyenne; équivalent indicatif 60k–140k tokens.
- **Compétences/outils :** parseurs, tables de symboles, arithmétique signée 128 bits, tests Rust.
- **Parallélisable :** oui avec DIS-001; non avec une modification simultanée de `ObjectImage.symbols` ou de la grammaire des labels.
- **Paquet de contexte minimal :** SPEC §11, `crates/assembler/src/expr.rs`, `crates/assembler/src/lib.rs`, `crates/asm-lexer/src/lib.rs`, tests assembleur.

### ASM-002B — Intégrer `.equ` et `.set` dans l’assemblage — TERMINÉ

- **Jalon / exigences :** M3; ASM-002, ASM-005, ASM-009, ASM-015, IO-005.
- **But :** ajouter des symboles absolus signés évalués en `i128`, utilisables dans les données et immédiats, sans les confondre avec les adresses de labels.
- **Non-but :** relocations ELF exportables, `.if`, macros et résolution de `.equ` dépendant d’un symbole futur ou d’un `.align` dont la taille dépend d’une définition future.
- **Entrées/sources :** SPEC §11 (directives `.equ/.set`, expressions signées et deux passes), §12 (image/symboles) et §19 (diagnostics); R4, directives et symboles absolus.
- **Fichiers/modules :** `crates/assembler/src/expr.rs`, `crates/assembler/src/lib.rs`, `TESTS.md`.
- **Étapes réalisées :** élargir la table d’évaluation aux valeurs `i128`; ajouter `ObjectImage.constants`; traiter `.equ` immuable et `.set` séquentiel; conserver les constantes hors de `symbols` d’adresses; rendre ces directives de taille nulle; refuser le mode mono-ligne et les collisions avec labels.
- **Dépendances :** ASM-002A; aucun nouvel outil externe; bloque la suite des directives conditionnelles qui doit réutiliser le même environnement de symboles.
- **Tests :** `.equ` dans `.word` et `addi`; valeur négative dans `.byte/.word`; réaffectation séquentielle `.set`; `.equ` immutable; erreur en mode `assemble` mono-ligne; suite assembleur.
- **Critères de sortie :** les bytes attendus sont produits sans décalage de PC; `constants` conserve les valeurs exactes signées; un label ne peut pas devenir constant; `.set` ne modifie pas les lignes antérieures.
- **Cas limites et échecs :** symbole absolu inconnu, expression invalide, nom manquant, local hors portée et `.align` dépendant d’une valeur non définie produisent un diagnostic; les relocations non résolues restent hors périmètre.
- **Taille :** 4 points / 1,5–2 journées-agent, incertitude moyenne; équivalent indicatif 60k–140k tokens.
- **Compétences/outils :** assembleur, expressions signées, invariants de passes Rust.
- **Parallélisable :** oui avec DIS-001; non avec une modification simultanée du contrat `ObjectImage` ou des directives de données.
- **Paquet de contexte minimal :** SPEC §§11–12/19, R4 directives, `crates/assembler/src/lib.rs`, `crates/assembler/src/expr.rs`.

### ASM-003A — Compléter chaînes et alignement de données — TERMINÉ

- **Jalon / exigences :** M3; ASM-003, ASM-009, IO-003.
- **But :** fournir `.string` comme chaîne terminée par zéro et `.balign` comme alignement exprimé en octets, avec le même calcul en première et seconde passes.
- **Non-but :** fill pattern de `.balign`, sections nommées, macros, includes et listing.
- **Entrées/sources :** SPEC §11 (directives `.string`, `.align`, `.balign`) et §12 (image déterministe); R4, directives de données.
- **Fichiers/modules :** `crates/assembler/src/lib.rs`, `TESTS.md`.
- **Étapes réalisées :** alias `.string` vers le comportement nul-terminé; validation d’un unique opérande `.balign`; calcul de padding borné partagé par émission et sizing; diagnostics de plage et d’arité.
- **Dépendances :** ASM-002B; aucune dépendance externe; prépare ASM-003 listing et sections sans modifier `ObjectImage`.
- **Tests :** bytes exacts `.string`/`.ascii`; alignement à 4 et 3 octets; arité invalide et alignement nul; suite assembleur.
- **Critères de sortie :** PC et symbole suivants voient exactement le même padding que l’image; `.string "ok"` produit `6f 6b 00`; `.balign N` ne remplit jamais au-delà de la prochaine frontière.
- **Cas limites et échecs :** frontière déjà alignée, chaîne Unicode mesurée en octets UTF-8, opérande manquant ou supplémentaire, valeur zéro et dépassement d’alignement.
- **Taille :** 3 points / 1–1,5 journée-agent, incertitude faible; équivalent indicatif 40k–100k tokens.
- **Compétences/outils :** assembleur de données, arithmétique de plages, tests Rust.
- **Parallélisable :** oui avec DIS-001 et FP-001; non avec une modification concurrente de la grammaire des directives.
- **Paquet de contexte minimal :** SPEC §§11–12, R4 directives, `crates/assembler/src/lib.rs`, `TESTS.md`.

### ASM-003B — Produire un listing source-adresse-bytes — TERMINÉ

- **Jalon / exigences :** M3; ASM-003, IO-005, DBG-001.
- **But :** conserver dans `ObjectImage` un listing déterministe pour corréler chaque ligne source avec son adresse et les bytes effectivement émis.
- **Non-but :** format de listing texte final, couleurs UI, sections nommées et expansion de macros.
- **Entrées/sources :** SPEC §§9, 11, 12 et 23 (contrat assembler/listing/reproductibilité); R4 pour les directives.
- **Fichiers/modules :** `crates/assembler/src/lib.rs`, `TESTS.md`.
- **Étapes réalisées :** ajouter `ListingEntry`; capturer source originale et numéros 1-based; produire une entrée pour instructions, données, alignements et directives sans bytes; rendre le mode mono-ligne cohérent.
- **Dépendances :** ASM-003A; aucun nouvel outil externe; l’interface pourra consommer le listing sans reparcourir le source.
- **Tests :** adresses successives avec `.equ`, `.balign`, `.string`; bytes exacts; entrée vide pour directive sans émission; listing de `assemble` mono-ligne.
- **Critères de sortie :** chaque ligne source conserve son adresse de départ; les lignes sans émission n’avancent pas le PC; les bytes du listing concaténés égalent `ObjectImage.text`.
- **Cas limites et échecs :** padding, chaîne terminée, ligne label-only et source vide; une erreur d’assemblage ne retourne pas de listing partiel.
- **Taille :** 3 points / 1–1,5 journée-agent, incertitude faible; équivalent indicatif 40k–100k tokens.
- **Compétences/outils :** API Rust, invariants de mapping source/bytes, tests déterministes.
- **Parallélisable :** oui avec DIS-001; non avec une modification concurrente de `ObjectImage`.
- **Paquet de contexte minimal :** SPEC §§9/11/12/23, `crates/assembler/src/lib.rs`, `TESTS.md`.

### ASM-003C — Sections nommées et image multi-section aplatie — TERMINÉ

- **Jalon / exigences :** M3; ASM-003, IO-003, IO-005, MEM-001.
- **But :** reconnaître les sections V1, regrouper leurs bytes et conserver une image aplatie compatible avec le loader actuel.
- **Non-but :** linker ELF, placement configurable, relocations inter-sections exportables et permissions runtime imposées par la machine.
- **Entrées/sources :** SPEC §§9, 11 et 12 (sections, loader, formats); R4 directives `.text/.rodata/.data/.bss/.section`.
- **Fichiers/modules :** `crates/assembler/src/parser.rs`, `crates/assembler/src/lib.rs`, `TESTS.md`.
- **Étapes réalisées :** ajouter `SectionImage`; flags par section; sélection des sections built-in et custom; refus des flags contradictoires; bytes groupés et adresses de première émission; ajout du nom de section au listing; directives sans opérandes reconnues par le parser.
- **Dépendances :** ASM-003B; aucun nouvel outil externe; le moniteur continue de charger `ObjectImage.text`.
- **Tests :** programme intercalant `.text`, `.rodata`, `.data`, `.section` custom et `.bss`; symboles et adresses; flags; erreurs d’arité et de redéclaration.
- **Critères de sortie :** image aplatie et sections contiennent les mêmes bytes dans l’ordre source; chaque listing porte la section active; un changement de section n’ajoute aucun byte.
- **Cas limites et échecs :** section vide, sélection répétée, flags divergents, nom/flags non ASCII ou absents, labels sur ligne de section.
- **Taille :** 5 points / 2–2,5 journées-agent, incertitude moyenne; équivalent indicatif 80k–180k tokens.
- **Compétences/outils :** assembleur, layout de sections, API Rust.
- **Parallélisable :** oui avec FP-001 et DIS-001; non avec une modification simultanée du layout `ObjectImage`.
- **Paquet de contexte minimal :** SPEC §§9/11/12, R4 directives, `crates/assembler/src/parser.rs`, `crates/assembler/src/lib.rs`.

### ASM-003D — Rendu texte canonique du listing — TERMINÉ

- **Jalon / exigences :** M3; ASM-003, IO-005, OBS-001.
- **But :** exporter le listing source/adresse/section/bytes sous une forme texte stable et reproductible.
- **Non-but :** couleurs terminal, pagination, localisation des messages et fichier `.map` séparé.
- **Entrées/sources :** SPEC §§12, 20–21 et 23 (export reproductible, observabilité et tests); décision locale du format de listing.
- **Fichiers/modules :** `crates/assembler/src/lib.rs`, `TESTS.md`.
- **Étapes réalisées :** ajouter `render_listing`; lignes 1-based, adresses hex 64 bits, section, bytes hex minuscules, `-` pour émission vide; exclure horodatages et chemins hôte.
- **Dépendances :** ASM-003C; aucune dépendance externe; l’interface peut réutiliser directement la chaîne canonique.
- **Tests :** stabilité bit-à-bit de deux rendus; ligne d’instruction avec bytes; ligne `.equ` vide; source originale conservée.
- **Critères de sortie :** même `ObjectImage` produit exactement la même chaîne; le rendu ne dépend d’aucun état hôte; les bytes affichés correspondent aux entrées de listing.
- **Cas limites et échecs :** bytes vides, sections custom, adresse haute, source contenant des espaces/commentaires.
- **Taille :** 2 points / 0,5–1 journée-agent, incertitude faible; équivalent indicatif 20k–70k tokens.
- **Compétences/outils :** formatage Rust, contrats de reproductibilité.
- **Parallélisable :** oui avec DIS-001; non avec une modification concurrente de `ListingEntry`.
- **Paquet de contexte minimal :** SPEC §§12/20/21/23, `crates/assembler/src/lib.rs`, `TESTS.md`.

### ASM-003E — Macros paramétrées bornées — TERMINÉ

- **Jalon / exigences :** M3; ASM-003, ASM-009, ASM-015, IO-005.
- **But :** fournir une expansion déterministe de macros paramétrées avant le lexer, avec imbrication bornée, substitutions explicites et conservation de l’origine source dans le listing.
- **Non-but :** inclusions de fichiers, conditionnel `.if`, génération de fichiers, réécriture de labels locaux et compatibilité complète avec les syntaxes de macros GNU/LLVM.
- **Entrées/sources :** SPEC §11 (macros, diagnostics, passes et compatibilité de dialecte), §12 (listing/export reproductible), §18 (isolation des chemins); R4 pour le dialecte de directives; décision locale versionnée du préprocesseur RVMonitor.
- **Fichiers/modules :** `crates/assembler/src/lib.rs`, `TESTS.md`, `BACKLOG.md`.
- **Étapes réalisées :** reconnaître `.macro NAME params` et `.endm`/`.endmacro`; accepter `\\param` et `$param`; séparer les arguments par virgules de niveau supérieur; développer récursivement les macros imbriquées; refuser la récursion et les arités incorrectes; borner définitions, corps, paramètres, profondeur et lignes produites; conserver la ligne source du corps dans `ListingEntry`.
- **Dépendances :** ASM-003D et ASM-002B; aucune dépendance externe; bloque le conditionnel qui devra réutiliser le même environnement de symboles; les inclusions restent une tranche distincte avec sandbox de chemins.
- **Tests :** macro paramétrée `addi`; macro imbriquée; arguments avec virgules; mapping de ligne du listing; récursion, arité incorrecte et définition non terminée; suite assembleur complète.
- **Critères de sortie :** un programme contenant une définition et un appel produit exactement les bytes de son corps; les macros imbriquées restent dans les quotas; les erreurs `ASM-MACRO-002/003/004/005/006` sont déterministes; aucun fichier hôte n’est lu.
- **Cas limites et échecs :** nom ou paramètre invalide, paramètre dupliqué, macro vide, commentaire dans une définition, chaîne contenant un séparateur, profondeur maximale et source produite au-delà du quota.
- **Taille :** 5 points / 2–2,5 journées-agent, incertitude moyenne; équivalent indicatif 80k–180k tokens.
- **Compétences/outils :** préprocesseurs, analyse lexicale bornée, diagnostics Rust, tests de quotas et de reproductibilité.
- **Parallélisable :** oui avec DIS-001 et FP-001; non avec une modification concurrente de la structure `ListingEntry` ou de la grammaire des directives.
- **Paquet de contexte minimal :** SPEC §§11–12/18/19/23, R4 directives et macros, `crates/assembler/src/lib.rs`, `TESTS.md`.

### ASM-003F — Includes sous sandbox de chemins — TERMINÉ

- **Jalon / exigences :** M3; ASM-003, ASM-009, ASM-015, IO-005, ISO-001.
- **But :** permettre des sources `.include "path"` récursives et déterministes, avec une API d’options explicite et un contrôle de confinement avant chaque lecture.
- **Non-but :** recherche implicite dans le répertoire courant, includes système, expansion conditionnelle `.if`, préprocesseur compatible avec toutes les extensions GNU, import ELF ou accès distant.
- **Entrées/sources :** SPEC §§11–12 (directives, listing et formats), §18 (validation des chemins et isolation), §19 (diagnostics), §23 (fuzzing et reproductibilité); R4 pour la syntaxe des directives; décision locale du sandbox de sources.
- **Fichiers/modules :** `crates/assembler/src/lib.rs`, `TESTS.md`, `BACKLOG.md`.
- **Étapes réalisées :** ajouter `AssemblyOptions`; conserver `assemble_program` sans accès disque; résoudre les includes relativement au fichier parent; canonicaliser racines, fichiers et symlinks; refuser absolus et `..`; détecter cycles; borner profondeur, octets et nombre de fichiers; rejeter UTF-8 invalide; faire passer le flux inclus avant l’expansion des macros.
- **Dépendances :** ASM-003E et ASM-002B; aucune dépendance externe; prépare les directives conditionnelles; le monitor et le guest restent sur l’API hermétique par défaut.
- **Tests :** include imbriqué relatif; macro défini dans un fichier inclus; include sans racine; traversal `..`; cycle; quota d’octets; suite assembleur complète.
- **Critères de sortie :** le même arbre autorisé produit les mêmes bytes; un fichier hors racine n’est jamais lu; un include sans options est refusé; les codes `ASM-INCLUDE-001/002/003/004/005/006` sont réservés aux échecs correspondants; aucun recours implicite au cwd.
- **Cas limites et échecs :** racine inexistante, racine qui n’est pas un répertoire, chemin vide ou non quoté, source principale hors racines, symlink sortant, cycle indirect, limite atteinte exactement et source non UTF-8.
- **Taille :** 5 points / 2–2,5 journées-agent, incertitude moyenne; équivalent indicatif 80k–180k tokens.
- **Compétences/outils :** API filesystem Rust, canonicalisation et sécurité des chemins, préprocesseurs, tests hermétiques.
- **Parallélisable :** oui avec DIS-001 et FP-001; non avec une modification concurrente du contrat `AssemblyOptions` ou du pipeline de prétraitement.
- **Paquet de contexte minimal :** SPEC §§11/12/18/19/23, R4 directives, `crates/assembler/src/lib.rs`, `TESTS.md`.

### ASM-003G — Conditionnels bornés et séquentiels — TERMINÉ

- **Jalon / exigences :** M3; ASM-003, ASM-009, ASM-015, IO-005, ISO-001.
- **But :** sélectionner des lignes avec `.if expression`, `.else` et `.endif`, en réutilisant l’environnement séquentiel des constantes `.equ/.set` et en ignorant complètement les branches mortes.
- **Non-but :** comparateurs supplémentaires, symboles de labels futurs, conditionnels dans les corps de macros, exécution dynamique, compatibilité complète avec les syntaxes GNU/LLVM et directives `.ifdef`/`.ifndef`.
- **Entrées/sources :** SPEC §§11–12 (préprocesseur, expressions, listing), §18 (bornes d’exécution), §19 (diagnostics), §23 (tests génératifs et non-régression); R4 pour les directives et expressions; décision locale de conditionnel entier non nul.
- **Fichiers/modules :** `crates/assembler/src/lib.rs`, `TESTS.md`, `BACKLOG.md`.
- **Étapes réalisées :** ajouter les frames conditionnelles imbriquées; évaluer les expressions actives avec les constantes déjà définies; mettre à jour `.equ/.set` actifs; supprimer les lignes mortes avant expansion des macros; ignorer les expressions mortes; refuser `.else`/`.endif` orphelins, blocs non terminés, profondeur excessive et directives conditionnelles dans les macros.
- **Dépendances :** ASM-003E et ASM-003F; aucune dépendance externe; le conditionnel doit précéder l’expansion des macros pour éviter de traiter du code mort; prépare le sign-off des directives M3.
- **Tests :** sélection `.if/.else`; `.set` visible par le conditionnel suivant; expression parenthésée; imbrication; symbole inconnu dans une branche morte; structure invalide; mélange macro/conditionnel; quota de profondeur; suite assembleur complète.
- **Critères de sortie :** seules les lignes de la branche active entrent dans l’assembleur; les constantes restent cohérentes avec la seconde passe; les codes `ASM-CONDITIONAL-001/002/003/005/006` sont déterministes; une branche morte invalide n’empêche pas l’assemblage.
- **Cas limites et échecs :** valeur nulle, valeur négative non nulle, `.else` double, `.endif` sans `.if`, `.if` sans expression, parenthèses invalides, symbole inconnu actif, profondeur exactement maximale et macro inactive.
- **Taille :** 4 points / 1,5–2 journées-agent, incertitude moyenne; équivalent indicatif 60k–140k tokens.
- **Compétences/outils :** préprocesseurs à états, expressions entières, invariants de passes, tests de diagnostics.
- **Parallélisable :** oui avec DIS-001 et FP-001; non avec une modification concurrente de l’ordre includes/macros/conditionnels.
- **Paquet de contexte minimal :** SPEC §§11/12/18/19/23, R4 directives, `crates/assembler/src/lib.rs`, `TESTS.md`.

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

### DIS-001A — Désassemblage mixte code/données — TERMINÉ

- **Jalon / exigences :** M3; DIS-001..006, MEM-001..012, IO-005.
- **But :** fournir un parcours de désassemblage où l’appelant marque explicitement les régions code et données, sans décoder les données par erreur et sans perdre les adresses.
- **Non-but :** détection heuristique automatique du code, prise en charge C exécutée, pseudo-instructions non prouvées, format ELF complet et mutation de la mémoire cible.
- **Entrées/sources :** SPEC §§13–15 (désassembleur, données mêlées au code, mémoire), §23 (round-trip et opcodes illégaux); R1/R2 pour le décodage des unités 32 bits; décision locale de carte de régions explicite.
- **Fichiers/modules :** `crates/disassembler/src/lib.rs`, `TESTS.md`, `BACKLOG.md`.
- **Étapes réalisées :** ajouter `DisassemblyRegion`, `DisassemblyRegionKind` et `DisassembledItem`; exiger des régions non vides, contiguës et couvrant toute l’image; décoder uniquement les régions `Code`; rendre les régions `Data` en `.byte` réassemblables par groupes de 16; conserver les items illégaux dans le flux code.
- **Dépendances :** GEN-001 et contrat `Instruction`; aucune dépendance externe; prépare la vue mémoire qui devra fournir les marques code/données; le support C reste une tranche distincte.
- **Tests :** image code/données/code; région data réassemblable et non décodée; illegal dans le code; gap, recouvrement et dépassement; suite workspace.
- **Critères de sortie :** aucune donnée marquée `Data` n’atteint `luna_isa::decode`; les adresses des items suivent exactement les offsets; les `.byte` reconstruisent bit à bit la région; les codes `DISASM-REGION-001/002` sont stables.
- **Cas limites et échecs :** image vide, région vide, données de 16/17 octets, adresse origin haute, code tronqué, opcode illégal, régions non triées et overflow d’adresse.
- **Taille :** 3 points / 1–1,5 journée-agent, incertitude faible; équivalent indicatif 40k–100k tokens.
- **Compétences/outils :** désassemblage, API de régions, round-trip assembleur, tests de bornes.
- **Parallélisable :** oui avec FP-001 et ASM-003; non avec une modification concurrente du format `DisassembledItem`.
- **Paquet de contexte minimal :** SPEC §§13–15/23, R1/R2, `crates/disassembler/src/lib.rs`, `TESTS.md`.

### DIS-001B — Intégrer les régions mixtes dans les consoles — TERMINÉ

- **Jalon / exigences :** M3/M6; DIS-001..006, MEM-001..012, CMD-001..005, REQ-PROD-005.
- **But :** exposer le désassemblage code/données dans les consoles locale et backend sans modifier la commande `disasm` historique.
- **Non-but :** détection automatique des frontières, persistance des marques de type code/data, support C 16 bits ou édition de la carte par interface graphique.
- **Entrées/sources :** SPEC §§10, 13–15 (commandes, désassemblage et vues mémoire), §20 (limites interactives), §23 (tests interactifs); contrat `luna-disassembler::disassemble_regions`; décision locale de syntaxe `disasm-mixed [addr] code:n,data:n,...`.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, `TESTS.md`, `BACKLOG.md`.
- **Étapes réalisées :** ajouter la commande `disasm-mixed` et les alias `mixed`/`dm`; parser les segments code/data et leur taille; lire une plage bornée via `TargetBackend`; rendre instructions et `.byte` dans un format commun; mettre à jour l’adresse de vue sans modifier le PC; exposer l’aide dans les deux consoles.
- **Dépendances :** DIS-001A et `TargetBackend`; aucune dépendance externe; prépare une future carte de régions persistée et l’intégration QEMU/guest.
- **Tests :** commande locale code/data/code; commande backend code/data/code; données non décodées; affichage des deux instructions et de l’adresse de vue; suite workspace.
- **Critères de sortie :** `mixed 0 c:4,d:3,c:4` produit les deux instructions et une directive `.byte`; les lectures restent limitées à 4096 octets; la commande historique `disasm` conserve son résultat.
- **Cas limites et échecs :** segment inconnu, longueur nulle, spécification vide, taille supérieure à la limite, code tronqué et adresse de lecture hors mémoire.
- **Taille :** 3 points / 1–1,5 journée-agent, incertitude faible; équivalent indicatif 40k–100k tokens.
- **Compétences/outils :** commandes interactives, backend générique, formatage déterministe, tests Rust.
- **Parallélisable :** oui avec FP-001; non avec une modification concurrente de la syntaxe des commandes mémoire/désassemblage.
- **Paquet de contexte minimal :** SPEC §§10/13–15/20/23, `crates/monitor/src/lib.rs`, `crates/disassembler/src/lib.rs`, `TESTS.md`.

### DIS-001C — Désassemblage C 16 bits opt-in — TERMINÉ

- **Jalon / exigences :** M3/M6; DIS-001..006, CMD-001..005, REQ-PROD-005.
- **But :** parcourir explicitement des régions code contenant des instructions C 16 bits et des instructions 32 bits, en conservant l’adresse et la largeur de chaque item.
- **Non-but :** émission C par l’assembleur, exécution C par la machine, activation implicite du profil C, décodage des extensions flottantes compressées non présentes dans le corpus R2 épinglé.
- **Entrées/sources :** SPEC §§5, 11, 13, 23; R1 Volume I, chapitre C; R2 commit épinglé via norms/manifest.toml et norms/r2/extensions/rv_c; contrat local C opt-in.
- **Fichiers/modules :** crates/disassembler/src/lib.rs, crates/monitor/src/lib.rs, TESTS.md.
- **Étapes réalisées :** générer la reconnaissance opcode depuis GENERATED_OPCODES; décoder les formes C présentes dans R2; rendre les encodages réservés/invalides comme .half; ajouter DisassemblyOptions; exposer disasm-mixed-c/mixed-c; conserver le comportement C-off historique.
- **Dépendances :** GEN-001, DIS-001A, DIS-001B; aucun outil externe à l’exécution.
- **Tests :** flux c.nop + addi; rejet C-off; encodage C invalide sans arrêt du flux; commandes locale et backend.
- **Critères de sortie :** disasm-mixed-c 0 c:2,c:4 affiche une unité 16 bits puis une instruction 32 bits; disasm-mixed refuse la même région; les données restent non décodées; aucune table d’opcodes locale n’est introduite.
- **Cas limites et échecs :** demi-mot tronqué, opcode réservé, recouvrement R2 avec sélection par contraintes, flux C/32 non aligné sur quatre octets, limite interactive de 4096 octets.
- **Taille :** 5 points / 2–3 journées-agent, incertitude moyenne; équivalent indicatif 80k–180k tokens.
- **Compétences/outils :** ISA C, extraction de champs, désassemblage, tests Rust.
- **Parallélisable :** oui avec les travaux d’interface mémoire; non avec une modification concurrente de DisassembledItem ou des commandes disasm-mixed.
- **Paquet de contexte minimal :** SPEC §§5/10/11/13/23, R1 chapitre C, R2 rv_c, crates/disassembler/src/lib.rs, crates/monitor/src/lib.rs, TESTS.md.

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

### FP-001A — Formats binaires exacts et directives de données — TERMINÉ

- **Jalon / exigences :** M4/M5; FP-001..006, IO-001..009.
- **But :** représenter et sérialiser sans conversion hôte les motifs binary16, binary32, binary64 et binary128; permettre les directives de données exactes.
- **Non-but :** exécuter Q/Zfh, convertir arbitrairement une décimale vers binary128, garantir encore le shortest-round-trip décimal pour les quatre formats.
- **Entrées/sources :** SPEC §§5, 11.1, 12; IEEE 754 via SPEC; R1 F/D/Q; décision D-005.
- **Fichiers/modules :** crates/floatfmt, crates/asm-lexer, crates/assembler, docs/TESTS.md.
- **Étapes réalisées :** ajouter FloatFormat binary16/binary128; classifier zéro, subnormal, normal, infini et NaN; préserver les motifs 16/32/64/128; accepter bits16/bits32/bits64/bits128; ajouter .binary16/.float/.double/.binary128; sérialiser little-endian; convertir les décimaux binary16/32/64 déterministes.
- **Dépendances :** FP-001 et contrat d’endianness LE; ne dépend pas d’un moteur Q/Zfh.
- **Tests :** motifs exacts, classes IEEE, decimal binary16 1.5, binary128 1.5 exact, largeur bits incompatible, decimal binary128 refusé explicitement.
- **Critères de sortie :** les quatre directives produisent les octets attendus; un motif d’une mauvaise largeur est rejeté avec ASM-FLOAT-002; aucun binary128 n’est converti via un type hôte.
- **Cas limites et échecs :** ±0, subnormal, infini, qNaN/sNaN, motif trop large, decimal binary128 non supporté, bfloat16 non aliasé.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** IEEE 754, parser, sérialisation binaire, Rust.
- **Parallélisable :** oui avec FP-ORACLE-001; non avec une modification concurrente du contrat FloatDisplay.
- **Paquet de contexte minimal :** SPEC §§5/11.1/12, crates/floatfmt/src/lib.rs, crates/assembler/src/parser.rs et src/lib.rs.

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

### FP-ORACLE-001 — Prototyper et figer l’oracle F — TERMINÉ

- **Jalon / exigences :** M4; FP-001..018, ISA F.
- **But :** comparer une implémentation logicielle candidate à SoftFloat/Sail/Spike/GNU disponibles.
- **Non-but :** intégrer un oracle dans le runtime sans audit.
- **Entrées/sources :** R1 F; R5 Sail; SoftFloat version/licence candidate.
- **Fichiers/modules :** `tools/oracles`, `norms/dependencies.lock`, rapport ADR.
- **Étapes réalisées :** épingler QEMU user-mode 11.0.2 dans le manifeste; construire un probe RISC-V autonome pour quatre cas F, quatre cas D et cinq cas d’arrondi F; comparer motifs de résultat et fflags au moteur; vérifier qu’une mutation du candidat est détectée; enregistrer la décision D-016.
- **Dépendances/bloqués :** BOOT-003; BOOT-001 pour versions; bloque FP-002.
- **Tests :** bash tools/check-fp-oracle.sh; corpus fixe; comparaison QEMU/machine; mutation de la chaîne candidate.
- **Acceptation :** QEMU 11.0.2 et le moteur concordent sur treize cas; la mutation contrôlée diverge; choix et licence sont enregistrés.
- **Limites/échecs :** outil absent → capability `unavailable`, jamais comparaison self-to-self.
- **Taille :** 5 points / 2,5 j, incertitude élevée.
- **Compétences/outils :** IEEE, Sail/Spike, FFI éventuel.
- **Parallélisable :** oui avec FP-001, non FP-002.
- **Contexte minimal :** SPEC §§4/11.1/23, règle utilisateur 5/8.

### FP-002 — Exécuter fadd.s avec fcsr et NaN-boxing — PARTIELLEMENT TERMINÉ

- **Jalon / exigences :** M4; FP-001..018, DBG-001, REQ-PROD-002.
- **But :** ajouter f-register bits, `fadd.s`, rm dynamique/statique, flags et box validation.
- **Non-but :** toutes les instructions F/D.
- **Entrées/sources :** FP-001, FP-ORACLE-001; R1 F “fcsr”.
- **Fichiers/modules :** `float`, `machine`, `isa`.
- **Étapes :** f[32]:u64; fcsr fields; operation; sticky flags; NaN-box; trace changes; profile gate.
- **Dépendances/bloqués :** FP-ORACLE-001; MACHINE-001.
- **Tests :** `fadd.s` normal, ±0, subnormal, NaN, invalid box, all rm; les tests d’exécution F/D et l’oracle d’arrondi sont détaillés dans FP-002A.
- **Acceptation :** E2E 5/6/13 match motif oracle and exact flags; hôte x86/arm same output.
- **Limites/échecs :** rm reserved, nonbox, unsupported profile → stable diagnostic/trap.
- **Taille :** 5 points / 2,5 j, incertitude élevée.
- **Compétences/outils :** IEEE/RISC-V F.
- **Parallélisable :** non avec FP-ORACLE integration.
- **Contexte minimal :** SPEC §§5/8/11.1/24.

### FP-002A — Modes d’arrondi déterministes F/D — TERMINÉ

- **Jalon / exigences :** M4/M5; FP-003..006, FP-012..018, DBG-001.
- **But :** exécuter `fadd.s` et `fadd.d` avec RNE, RTZ, RDN, RUP et RMM, en mode statique ou dynamique via `frm`, sans dépendre de l’arrondi de l’hôte.
- **Non-but :** conversions entier/flottant, opérations autres que l’addition, binary16/binary128 exécutés, et preuve exhaustive des règles underflow de toutes les opérations.
- **Entrées/sources :** R1 §§11.1/11.2 (F/D, `rm`, `frm`, `fflags`); R2 commit épinglé pour les champs; QEMU 11.0.2 comme oracle externe D-016.
- **Fichiers/modules :** `crates/machine/src/lib.rs`, `crates/machine/examples/fp_probe.rs`, `tests/oracles/fp_qemu_probe.S`, `tools/check-fp-oracle.sh`.
- **Étapes réalisées :** ajouter une arithmétique de significandes entières avec bits de garde/sticky; traiter les cinq modes, les ties, les débordements, les subnormaux et les zéros signés; refuser `rm=5/6` et `frm=5/6` avec `TRAP-FP-RM-001`; conserver les flags sticky.
- **Dépendances :** FP-002, FP-ORACLE-001; prépare FP-004 conversions.
- **Tests :** tie `1 + 2^-24` dans les cinq modes; tie binary64; mode dynamique; modes réservés; QEMU sur treize cas F/D de résultats et flags.
- **Acceptation :** les motifs et `fflags` de `cargo test -p luna-machine` et `bash tools/check-fp-oracle.sh` sont stables sur l’hôte; QEMU 11.0.2 concorde; une mutation contrôlée de la sortie est détectée.
- **Limites/échecs :** les conversions, multiplication/division et l’exécution Q/Zfh restent à implémenter; le calcul exact est borné aux formats F/D actuels.
- **Taille :** 5 points / 2,5 journées-agent, incertitude moyenne.
- **Compétences/outils :** IEEE 754, RISC-V F/D, arithmétique multi-précision bornée, QEMU, Rust.
- **Parallélisable :** oui avec la préparation des conversions et la documentation; non avec une modification concurrente du modèle `fcsr`.
- **Contexte minimal :** SPEC §§5/8/11.1/24, `crates/machine/src/lib.rs`, `tests/oracles/fp_qemu_probe.S`, `norms/oracles/manifest.toml`.

### FP-003A — Conversions de format F/D — TERMINÉ

- **Jalon / exigences :** M5; FP-003..006, FP-012..018, IO-004.
- **But :** implémenter `fcvt.s.d` et `fcvt.d.s` avec bits exacts, NaN-boxing, modes d’arrondi et flags.
- **Non-but :** conversions entier↔flottant, binary16/binary128 exécutés, et instructions Q/Zfh.
- **Entrées/sources :** R1 §§11.1/11.2; R2 commit épinglé; QEMU 11.0.2 via D-016.
- **Fichiers/modules :** `crates/isa`, `crates/assembler`, `crates/disassembler`, `crates/machine`, probe QEMU.
- **Étapes réalisées :** ajouter `FloatConversionKind` dans l’AST ISA; générer l’encodage depuis R2; assembler/désassembler les deux mnémotechniques; réutiliser l’arithmétique exacte pour les ties; préserver les flags et rejeter les modes réservés.
- **Tests :** round-trip ISA, assembleur/désassembleur, binary64→binary32 RNE/RUP, binary32→binary64, infini négatif, sNaN et oracle QEMU indépendant.
- **Acceptation :** 134 tests Cargo et `bash tools/check-fp-conversion-oracle.sh` passent; QEMU 11.0.2 concorde sur trois cas résultat/flags.
- **Limites/échecs :** les conversions entier↔flottant W/L sont maintenant couvertes pour F/D; les payloads NaN non canoniques sont conservées seulement lorsque la règle de conversion le permet; binary16/binary128 restent hors exécution.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** RISC-V F/D, IEEE 754, tables R2, QEMU, Rust.
- **Parallélisable :** oui avec la préparation de FP-004; non avec une modification concurrente de l’AST `Instruction`.
- **Contexte minimal :** SPEC §§5/11.1/13/23, R2 `rv_f`/`rv_d`, `crates/isa/src/lib.rs`, oracle de conversion.

### FP-003 — Étendre D et refuser proprement Q/Zfh exécutable

- **Jalon / exigences :** M5; F/D/Zfh/Q matrix.
- **But :** implémenter D nécessaire, données Zfh/Q et capability refusée.
- **Non-but :** Q runtime.
- **Entrées/sources :** R1 D/Q; R2; DECISIONS D-005.
- **Fichiers/modules :** `float`, `machine`, `isa`, profile.
- **Étapes :** `fadd.d`, conversions de format et variantes W/L entier/flottant réalisées; binary16/128 data; execute gate; diagnostics.
- **Dépendances/bloqués :** FP-002, GEN-001; bloque M5.
- **Tests :** D motifs/flags; Q decode/data; executing Q trap; matrix generated.
- **Acceptation :** E2E 5/7 et les conversions de format/W sont disponibles; aucune extension non exécutée n’est présentée comme support.
- **Limites/échecs :** wrong FLEN, invalid Q opcode, Zfh instruction → explicit unsupported.
- **Taille :** 5 points / 2,5 j, incertitude moyenne.
- **Compétences/outils :** IEEE/RISC-V D/Q.
- **Parallélisable :** oui avec CMD-001 après contract.
- **Contexte minimal :** SPEC §5.3/11.1.

### GEN-003 — Archiver et décoder les extensions Zfh/Zfhmin/Q — TERMINÉ

- **Jalon / exigences :** M5; ENC-001..006, ISA-001..010, FP-001..018, IO-004.
- **But :** intégrer dans les artefacts générés les fichiers R2 `rv_zfhmin`, `rv_zfh`, `rv64_zfh`, `rv_q` et `rv64_q`, puis distinguer un mot décodable mais non exécutable d’un opcode illégal.
- **Non-but :** exécuter binary16 ou binary128, convertir des littéraux décimaux binary128, ajouter une table opcode manuelle, ou annoncer une compatibilité toolchain complète.
- **Entrées/sources :** R1 F/D/Q et formats flottants; R2 commit `c6edca7d8c3f92694963a0a0baeb511930fb2af4`; DECISIONS D-005/D-006/D-017.
- **Fichiers/modules :** `norms/r2/extensions/`, `norms/r2/SHA256SUMS`, `crates/isa-core/build.rs`, `crates/isa/src/lib.rs`, `crates/assembler/src/lib.rs`, `crates/disassembler/src/lib.rs`, `crates/machine/src/lib.rs`, `tools/check-r2.sh`.
- **Étapes réalisées :** archiver les cinq snapshots avec SHA vérifiés contre le commit R2; générer leurs entrées; introduire `GeneratedInstruction`; assembler les formes réelles par champs R2; réencoder sans perte; afficher `.word` et le mnémonique `[decode-only]`; refuser l’exécution par `TRAP-UNSUPPORTED-EXTENSION`.
- **Dépendances et tâches bloquées :** BOOT-005/GEN-002; les pseudo-instructions, le désassemblage canonique des opérandes et l’exécution restent différés.
- **Tests :** `bash tools/check-r2.sh`; tests assembleur de formes Zfh/Q générées avec registres et mémoire; test ISA de quatre mots Zfh/Q; round-trip decode→encode; tests machine ciblant le diagnostic d’extension non exécutée; tests de désassemblage `.word` réassemblable.
- **Critères de sortie :** les cinq extensions apparaissent dans `GENERATED_EXTENSIONS`; les fichiers sont identiques au SHA upstream; un mot `fadd.h`, `fmv.h.x`, `fadd.q` ou `flq` est distingué d’un opcode illégal et conserve ses bits; aucune branche machine ne l’exécute.
- **Cas limites et échecs :** fichier R2 manquant ou altéré, extension 16 bits dans le chemin 32 bits, mot non reconnu, pseudo-instruction non traitée comme opcode réel; chaque cas produit un échec ou une représentation explicite.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne; équivalent indicatif 80k–160k tokens.
- **Compétences/outils :** RISC-V F/D/Q/Zfh, génération R2, Rust, désassemblage, contrôle SHA.
- **Parallélisable :** oui avec la documentation de matrice; non avec une modification concurrente de `Instruction` ou du générateur.
- **Paquet de contexte minimal :** `crates/isa-core/build.rs`, `crates/isa/src/lib.rs`, `crates/disassembler/src/lib.rs`, D-017, SPEC §§5/11/13/23.

### FP-004A — Conversions entières W F/D — TERMINÉ

- **Jalon / exigences :** M5; FP-007..011, FP-015..018, IO-004.
- **But :** exécuter `fcvt.w.s`, `fcvt.wu.s`, `fcvt.s.w`, `fcvt.s.wu`, et leurs variantes D avec résultat RV64 conforme.
- **Non-but :** variantes `fcvt.l*`, binary16/binary128 exécutés, et instructions Q/Zfh.
- **Entrées/sources :** R1 §§11.1/11.2; R2 commit épinglé; QEMU 11.0.2 via D-016.
- **Fichiers/modules :** `crates/isa`, `crates/assembler`, `crates/disassembler`, `crates/machine`, probe QEMU.
- **Étapes réalisées :** ajouter les huit kinds de conversion dans les tables de dispatch; arrondir les fractions avec `rm/frm`; produire `NV` pour NaN/infini/dépassement et `NX` pour les résultats inexactes; appliquer les bornes signées/non signées et l’extension RV64 des résultats W.
- **Tests :** 1.75, -1.75 sous RDN, 3.5 unsigned, -1 unsigned, +inf signed, -123, `0xffffffff` unsigned→float, `INT_MIN`; round-trip assembleur/désassembleur; oracle QEMU sur huit cas.
- **Acceptation :** 137 tests Cargo et `bash tools/check-fp-integer-oracle.sh` passent pour la tranche W; les huit motifs/flags concordent avec QEMU 11.0.2.
- **Limites/échecs :** les variantes L sont traitées par FP-004B; les résultats NaN et les bornes supplémentaires doivent encore être ajoutés au corpus large IEEE.

- **Taille :** 5 points / 2,5 journées-agent, incertitude moyenne.
- **Compétences/outils :** psABI/ISA F/D, IEEE 754, conversions signées, QEMU, Rust.
- **Parallélisable :** oui avec le corpus IEEE; non avec une modification concurrente de `FloatConversionKind`.
- **Contexte minimal :** SPEC §§5/11.1/23/24, R2 `rv_f`/`rv_d`, `crates/machine/src/lib.rs`, oracle integer.

### FP-004B — Conversions entières L RV64 — TERMINÉ

- **Jalon / exigences :** M5; FP-007..011, FP-015..018, IO-004.
- **But :** exécuter les huit formes RV64 `fcvt.l.s`, `fcvt.lu.s`, `fcvt.s.l`, `fcvt.s.lu`, `fcvt.l.d`, `fcvt.lu.d`, `fcvt.d.l`, `fcvt.d.lu` avec résultats, arrondis et `fflags` déterministes.
- **Non-but :** binary16/binary128 exécutés, instructions Q/Zfh, et corpus exhaustif de toutes les opérations IEEE.
- **Entrées/sources :** R1 §§11.1/11.2; R2 commit épinglé; QEMU 11.0.2 via D-016.
- **Fichiers/modules :** `crates/isa`, `crates/assembler`, `crates/disassembler`, `crates/machine`, `crates/machine/examples/fp_integer_probe.rs`, `tests/oracles/fp_integer_qemu_probe.S`, `tools/check-fp-integer-oracle.sh`.
- **Étapes réalisées :** ajouter les kinds L aux tables générées et aux dispatches encode/decode; accepter les classes de registres correctes; généraliser les conversions aux bornes signées/non signées 64 bits; conserver les résultats W sign-étendus; produire `NV`/`NX` conformément aux cas validés par QEMU.
- **Tests :** arrondi de `1.75`, négatif unsigned, `i64::MIN`, `u64::MAX`, infini, round-trip assembleur/désassembleur et oracle externe.
- **Acceptation :** 138 tests Cargo et `bash tools/check-fp-integer-oracle.sh` passent; QEMU 11.0.2 concorde sur treize cas W/L de résultat et flags.
- **Limites/échecs :** les conversions L sont limitées aux sources F/D exécutées; les formats binary16/binary128 restent représentables mais non exécutables.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** RISC-V F/D, IEEE 754, conversions signées, tables R2, QEMU, Rust.
- **Parallélisable :** oui avec le corpus IEEE; non avec une modification concurrente de `FloatConversionKind` ou de la sémantique `fcsr`.
- **Contexte minimal :** SPEC §§5/11.1/23/24, R2 `rv_f`/`rv_d`, `crates/machine/src/lib.rs`, oracle integer.

### FP-005A — Mouvements binaires F/D — TERMINÉ

- **Jalon / exigences :** M5; FP-001..006, FP-015..018, IO-004.
- **But :** assembler, désassembler et exécuter `fmv.x.w`, `fmv.w.x`, `fmv.x.d` et `fmv.d.x` avec transfert bit-à-bit entre registres entiers et flottants.
- **Non-but :** nouvelles opérations arithmétiques F/D, formats binary16/binary128 exécutés, et extensions Zfh/Q.
- **Entrées/sources :** R1 §§11.1/12; R2 commit épinglé, entrées `rv_f`, `rv64_f`, `rv_d`, `rv64_d`; QEMU 11.0.2 via oracle local.
- **Fichiers/modules :** `crates/isa`, `crates/assembler`, `crates/disassembler`, `crates/machine`, `crates/machine/examples/fp_move_probe.rs`, `tests/oracles/fp_move_qemu_probe.S`, `tools/check-fp-move-oracle.sh`.
- **Étapes réalisées :** générer les formes depuis les entrées R2 déjà archivées; distinguer les classes x/f des opérandes; appliquer la sign-extension RV64 de `fmv.x.w`; appliquer le NaN-boxing de `fmv.w.x`; préserver intégralement les 64 bits pour les formes D; ne modifier aucun flag.
- **Tests :** round-trip ISA, classes de registres assembleur, désassemblage canonique et quatre motifs limites indépendants QEMU.
- **Acceptation :** 139 tests Cargo et `bash tools/check-fp-move-oracle.sh` passent; les quatre sorties bit-à-bit concordent avec QEMU 11.0.2.
- **Limites/échecs :** les opérations F/D autres que `fadd.*`, conversions et mouvements restent hors de cette tranche; Zfh/Q restent non exécutés.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** RISC-V F/D, NaN-boxing, génération R2, QEMU, Rust.
- **Parallélisable :** oui avec la préparation de `CMD-001`; non avec une modification concurrente de `Instruction` ou du format de registre flottant.
- **Contexte minimal :** SPEC §§5/8/11.1/15/23/24, R2 `rv_f`/`rv64_f`/`rv_d`/`rv64_d`, `crates/isa/src/lib.rs`, oracle move.

## M6–M8 — interface, debug, persistance

### CMD-001A — Stabiliser le parseur partagé des lignes de commande — TERMINÉ

- **Jalon / exigences :** M6; CMD-001, CMD-003, REQ-PROD-005.
- **But :** fournir un découpage déterministe commun à `Monitor` et `BackendConsole`, sans laisser une commande mal formée muter la cible.
- **Non-but :** évaluer encore les expressions 128 bits, valider les plages par commande, ou construire le frontend terminal.
- **Entrées/sources :** SPEC §10/19; DECISIONS D-001/D-007.
- **Fichiers/modules :** `crates/monitor/src/command.rs`, `crates/monitor/src/lib.rs`.
- **Étapes réalisées :** parser le nom ASCII et la queue brute; tokenizer espaces/guillemets/échappements; exposer tokens et argument original; limiter une ligne à 64 KiB; utiliser les diagnostics `CMD-001`, `CMD-003` et `CMD-006`.
- **Dépendances/bloqués :** aucun; prépare CMD-001 et doit précéder l’AST des expressions et la validation contextuelle.
- **Tests :** lignes vides, alias `?`, queue assembleur préservée, guillemets, échappements, noms invalides, commande inconnue et absence d’effet de bord.
- **Critères de sortie :** les deux consoles passent par le même parseur; une commande inconnue ou mal quotée est rejetée avant toute opération cible; la suite `cargo test -p luna-monitor` passe.
- **Cas limites et échecs :** ligne de 64 KiB acceptée selon la limite, dépassement rejeté; guillemet ou échappement final produit `CMD-003`; Unicode dans le nom refusé.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** Rust, parsing déterministe, diagnostics.
- **Parallélisable :** oui avec le modèle mémoire MON-001; non avec une modification concurrente du contrat de dispatch.
- **Paquet de contexte minimal :** SPEC §10/19, `crates/monitor/src/lib.rs`, `docs/TESTS.md`.

### CMD-001B — AST d’expressions et plages d’adresses contrôlées — TERMINÉ

- **Jalon / exigences :** M6; CMD-001, CMD-003, CMD-004, REQ-PROD-005.
- **But :** évaluer des expressions entières signées 128 bits avant conversion contrôlée en adresse RV64 et partager cette sémantique entre les deux consoles.
- **Non-but :** conditions `if` des breakpoints, expressions de watchpoints, mutation pendant `run`, ou un langage de script.
- **Entrées/sources :** SPEC §10/19; DECISIONS D-001/D-003/D-007.
- **Fichiers/modules :** `crates/monitor/src/command.rs`, `crates/monitor/src/lib.rs`.
- **Étapes réalisées :** AST littéral/symbole/unaires/binaires; bases décimale, hexadécimale et binaire; précédence, parenthèses, décalages et opérations contrôlées; symboles, `pc`, registres ABI/numériques et marques; plages `start..end`; support de `disasm pc..pc+N`.
- **Dépendances/bloqués :** CMD-001A; la validation des commandes structurées et des conditions de breakpoint reste à faire.
- **Tests :** précédence, overflow signé, division par zéro, symboles, plages inversées/multiples, adresses `pc`, registre et marque sans modification du PC.
- **Critères de sortie :** aucun wrap implicite vers u64; valeur négative, dépassement, symbole inconnu, opération invalide ou plage non alignée produisent `CMD-003` ou le code contextuel; le même AST est utilisé par `Monitor` et `BackendConsole`.
- **Cas limites et échecs :** `i128::MAX+1`, division par zéro, décalage ≥128, plage inversée, longueur non multiple de 4, registre hors `x0..x31`.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** parsing Pratt, arithmétique signée, débogueur RV64.
- **Parallélisable :** oui avec MON-001; non avec une modification concurrente de la syntaxe d’adresse.
- **Paquet de contexte minimal :** SPEC §10/19, `crates/monitor/src/command.rs`, contrat `TargetContext`.

### CMD-001C — Valider l’arité et les plages avant dispatch — TERMINÉ

- **Jalon / exigences :** M6; CMD-001, CMD-002, CMD-004, REQ-PROD-005.
- **But :** rejeter les commandes manifestement incomplètes et les plages inversées avant tout accès ou mutation de la cible.
- **Non-but :** valider encore les conditions `if`, les watchpoints structurés, les expressions de script ou l’état `run` asynchrone.
- **Entrées/sources :** SPEC §10/19; DECISIONS D-007/D-008.
- **Fichiers/modules :** `crates/monitor/src/command.rs`, `crates/monitor/src/lib.rs`.
- **Étapes réalisées :** arité minimale commune aux deux consoles; `CMD-002` pour argument obligatoire absent; `CMD-004` pour `start..end` inversé; validation exécutée avant le dispatch; conservation des diagnostics spécialisés pour la syntaxe détaillée.
- **Dépendances/bloqués :** CMD-001A/CMD-001B; la validation de contexte cible et des mutations pendant l’exécution reste à faire.
- **Tests :** `view` sans argument, commande optionnelle `run`, plage inversée, absence d’effet de bord et compatibilité des deux surfaces de commande.
- **Critères de sortie :** une commande incomplète ne lit ni n’écrit la cible; `disasm 0x20..0x10` renvoie `CMD-004`; `cargo test -p luna-monitor` passe.
- **Cas limites et échecs :** alias et casse mélangés, argument quoté, expression complexe, plage non alignée; ces derniers restent diagnostiqués par le handler spécialisé.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** validation de grammaire, diagnostics, tests Rust.
- **Parallélisable :** oui avec MON-001; non avec une modification concurrente du catalogue de commandes.
- **Paquet de contexte minimal :** SPEC §10/19, `crates/monitor/src/command.rs`, `crates/monitor/src/lib.rs`.

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

### MON-001A — Modèle de curseur/sélection et rendu mémoire partagé — TERMINÉ

- **Jalon / exigences :** M6; MEM-001..012, REQ-PROD-003/005.
- **But :** stabiliser un modèle backend-neutre de curseur/sélection et partager le rendu hex/ASCII entre le simulateur et `BackendConsole`.
- **Non-but :** raccorder les raccourcis clavier, construire les widgets terminaux, ajouter encore recherche/fill/copy ou persister une sélection UI.
- **Entrées/sources :** SPEC §14/15; A1 ch. 6–9; DECISIONS D-007/D-008.
- **Fichiers/modules :** `crates/monitor/src/memory_view.rs`, `crates/monitor/src/lib.rs`.
- **Étapes réalisées :** invariants de plage inclusive/exclusive; curseur avec navigation sans wrap; jump qui efface la sélection; sélection normalisée; rendu hex/ASCII byte-exact commun aux deux consoles.
- **Dépendances/bloqués :** CMD-001B/CMD-001C; l’intégration des événements clavier, marques persistées dans ce modèle et opérations recherche/fill/copy reste dans MON-001.
- **Tests :** overflow de curseur, sélection inversée, jump/clear, rendu ASCII des octets non imprimables, conservation d’adresse et undo existants.
- **Critères de sortie :** un seul renderer produit les deux vues textuelles; aucune navigation ne wrappe une adresse; le rollback mémoire conserve le rendu et l’adresse; `cargo test -p luna-monitor` passe.
- **Cas limites et échecs :** adresse `0` avec déplacement négatif, `u64::MAX` avec déplacement positif, sélection vide, ligne mémoire finale partielle.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** modèle d’état, arithmétique d’adresses, rendu terminal, tests Rust.
- **Parallélisable :** oui avec REG-001; non avec une modification concurrente du format hex/ASCII.
- **Paquet de contexte minimal :** SPEC §14/15, `crates/monitor/src/memory_view.rs`, `crates/monitor/src/lib.rs`.

### MON-001B — Recherche, remplissage et copie mémoire transactionnels — TERMINÉ

- **Jalon / exigences :** M6; MEM-001..012, REQ-PROD-003/005, ISO-001.
- **But :** compléter la surface mémoire avec `find`, `fill` et `copy`, bornées et annulables, sur le simulateur comme sur le backend cible.
- **Non-but :** recherche regex, wildcards, MMIO spécialisé, sélection clavier ou copie directe vers la mémoire hôte.
- **Entrées/sources :** SPEC §§14/18/19; A1 ch. 6–9; DECISIONS D-007/D-008.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, `crates/monitor/src/memory_view.rs`.
- **Étapes réalisées :** `find <addr> <count> <hex-bytes>`; `fill <addr> <count> <byte>`; `copy <src> <dst> <count>`; limite 4096 octets; lecture complète avant écriture; undo des destinations; buffer temporaire pour les copies chevauchantes; aide des deux consoles.
- **Dépendances/bloqués :** MON-001A/CMD-001C; la sélection interactive et les marques dans le modèle UI restent à intégrer.
- **Tests :** correspondances multiples, motif absent, remplissage et undo, copie chevauchante, compte nul, sémantique identique du backend.
- **Critères de sortie :** aucune écriture partielle avant validation et lecture de l’ancien contenu; `undo` restaure exactement la destination; une copie `src`/`dst` recouvrante respecte les octets source initiaux; `cargo test -p luna-monitor` passe.
- **Cas limites et échecs :** motif vide, octet mal formé, compte nul ou >4096, mémoire non mappée, destination non accessible; la cible reste inchangée en cas d’échec avant écriture.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** mémoire transactionnelle, backend abstrait, tests d’overlap.
- **Parallélisable :** oui avec REG-001; non avec une modification concurrente de `MemoryEdit` ou des limites mémoire.
- **Paquet de contexte minimal :** SPEC §§14/18/19, `crates/monitor/src/lib.rs`, `crates/monitor/src/memory_view.rs`.

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

### REG-001A — Parsing et édition exacte des registres internes — TERMINÉ

- **Jalon / exigences :** M6/M7; REQ-PROD-003/005, DBG-001..014, FP-001..018.
- **But :** stabiliser les noms de registres et l’édition bit-exacte des registres entiers/flottants du simulateur interne.
- **Non-but :** écrire les registres d’un backend distant avant extension de `TargetBackend`, modifier `fcsr` par commande ou construire les widgets terminaux.
- **Entrées/sources :** SPEC §§8/15/16; R1 F/Zicsr; R3 convention de registres; DECISIONS D-003/D-005/D-007.
- **Fichiers/modules :** `crates/monitor/src/register_view.rs`, `crates/monitor/src/lib.rs`.
- **Étapes réalisées :** parsing `x0..x31` et aliases ABI; parsing `f0..f31`; motifs u64 décimaux/hexadécimaux avec séparateurs; `set` et `setf` sur le simulateur; `x0` read-only; représentation flottante sans conversion hôte.
- **Dépendances/bloqués :** MON-001B; l’édition CSR, le diff de registres à chaque stop et le contrat d’écriture du backend distant restent à faire.
- **Tests :** aliases `a0`, bit pattern flottant exact, overflow, classes incorrectes, protection de x0 et absence de mutation après erreur.
- **Critères de sortie :** toute édition valide modifie exactement le registre demandé; aucun float n’est converti par l’hôte; x0 reste nul; le backend distant reste explicitement read-only.
- **Cas limites et échecs :** index hors 31, valeur >64 bits, f-register utilisé avec `set`, x0, motif NaN/non-boxé; chaque erreur conserve l’état.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** ABI RISC-V, bit patterns IEEE, contrats backend, Rust.
- **Parallélisable :** oui avec la finalisation MON-001; non avec une modification concurrente de `TargetContext`.
- **Paquet de contexte minimal :** SPEC §§8/15/16, `crates/monitor/src/register_view.rs`, `crates/target-api/src/lib.rs`.

### REG-001B — Capturer les deltas exacts des registres après exécution — TERMINÉ

- **Jalon / exigences :** M7; REQ-DBG-004, REQ-DBG-008, REQ-OBS-001.
- **But :** exposer après chaque `step` et dans l’historique les changements exacts de `x`, `f` et `fcsr`, sans conversion flottante hôte.
- **Non-but :** construire l’interface terminale interactive, modifier les registres distants ou fournir encore un historique arrière réversible.
- **Entrées/sources :** SPEC §§9, 15, 16, 21; R1 chapitre F pour `fcsr`, `frm` et `fflags`; contrat `TargetContext`.
- **Fichiers/modules :** `crates/monitor/src/register_view.rs`, `crates/monitor/src/lib.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** définir un snapshot de registres; comparer avant/après host et backend; formater les motifs 64 bits et les champs `fcsr`; conserver le delta dans chaque entrée d’historique.
- **Dépendances et tâches bloquées :** REG-001A; l’interface terminale peut maintenant consommer le format, tandis que l’édition distante reste bloquée par un contrat d’écriture backend non disponible.
- **Tests :** `addi` affiche le changement de `x1`; `fadd.s` affiche le motif NaN-boxé de `f3`; `fcsr` affiche `frm`/`fflags`; historique et backend conservent le même delta; aucun changement donne `changes: none`.
- **Critères de sortie :** format déterministe indépendant de l’hôte, registre modifié affiché une seule fois, `x0` jamais signalé comme modifié par une instruction légale, tests monitor verts.
- **Cas limites et échecs :** instruction sans effet → `none`; seule modification de `fcsr` → champs exacts; flags sticky visibles; trap avant retirement ne crée pas de delta d’instruction.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** Rust, contrat backend, IEEE 754/RISC-V FCSR, tests snapshot.
- **Parallélisable :** oui avec la préparation UI; non avec une modification concurrente de `HistoryEntry` ou du format de sortie.
- **Paquet de contexte minimal :** SPEC §§9/15/16/21, `crates/target-api/src/lib.rs`, `crates/monitor/src/register_view.rs`, `docs/TESTS.md`.

### REG-001C — Éditer fcsr par champs avec validation architecturale — TERMINÉ

- **Jalon / exigences :** M7; REQ-DBG-004, REQ-DBG-008, FP-CSR-001.
- **But :** permettre au simulateur interne d’éditer `fcsr`, `frm` ou `fflags` sans écraser silencieusement les autres champs.
- **Non-but :** écrire les CSR d’un backend distant, exposer les CSR privilégiés ou ajouter un mode d’arrondi non défini par l’ISA.
- **Entrées/sources :** SPEC §§8/15/16; R1 chapitre F, registre `fcsr`; contrat `TargetBackend` read-only.
- **Fichiers/modules :** `crates/monitor/src/register_view.rs`, `crates/monitor/src/command.rs`, `crates/monitor/src/lib.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** parser `setcsr fcsr|frm|fflags`; préserver les champs non ciblés; refuser bits hors `fcsr[7:0]`, flags hors 5 bits et `frm=5..7`; conserver l’état inchangé après erreur.
- **Dépendances et tâches bloquées :** REG-001A/REG-001B; l’édition CSR distante reste bloquée par l’absence d’opération d’écriture dans `TargetBackend`.
- **Tests :** écritures partielles, valeur complète, modes réservés, overflow et absence de mutation; diagnostics `MON-REG-008..011`.
- **Critères de sortie :** toute écriture valide donne un `fcsr` déterministe et les champs `frm`/`fflags` correspondants; aucune écriture invalide ne modifie l’état.
- **Cas limites et échecs :** `frm=7`, `fcsr=0x100`, `fflags=0x20`, nom CSR inconnu et arité incorrecte.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** Rust, RISC-V F/Zicsr, diagnostics structurés.
- **Parallélisable :** oui avec la préparation de l’interface; non avec une modification concurrente du format `fcsr`.
- **Paquet de contexte minimal :** SPEC §§8/15/16, R1 chapitre F, `crates/monitor/src/register_view.rs`.

### REG-001D — Surligner les registres modifiés dans la vue `regs` — TERMINÉ

- **Jalon / exigences :** M7; REQ-DBG-004, REQ-DBG-008, REQ-OBS-001.
- **But :** rendre visibles dans les consoles host et backend les registres modifiés depuis le dernier arrêt, sans altérer leur représentation exacte.
- **Non-but :** introduire une bibliothèque TUI, modifier le protocole QEMU ou rendre l’édition distante accessible.
- **Entrées/sources :** SPEC §§15/16/21; contrat `TargetContext`; format de delta REG-001B.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, `crates/monitor/src/register_view.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** mémoriser une baseline à la création, au chargement et au dernier step/run; marquer `x`, `f` et `fcsr` avec `*`; réinitialiser la baseline après reset/restauration.
- **Dépendances et tâches bloquées :** REG-001B/REG-001C; les widgets clavier et le rendu couleur restent une étape UI ultérieure.
- **Tests :** `addi`, `fadd.s`, `fcsr`, `run/continue`, reset et backend vérifient le marqueur; les bits et décimaux existants restent inchangés.
- **Critères de sortie :** `regs` affiche `*` uniquement quand la valeur diffère de la baseline; le marqueur disparaît après une nouvelle baseline; aucun changement de valeur n’est introduit.
- **Cas limites et échecs :** `x0` reste zéro; instruction sans effet sans marque; restauration de snapshot ne conserve pas une baseline étrangère.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** Rust, modèle de vue, débogage interactif.
- **Parallélisable :** oui avec le shell interactif; non avec une modification concurrente de `RegisterSnapshot`.
- **Paquet de contexte minimal :** SPEC §§15/16/21, `crates/monitor/src/lib.rs`, `crates/monitor/src/register_view.rs`.

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

#### DBG-001A — Conditions de breakpoint entières — TERMINÉ

- **Jalon / exigences :** M7; DBG-001..004, REQ-PROD-003.
- **But :** permettre `break <adresse> if <expression>` dans le moniteur host
  et le backend générique, avec évaluation sans effet de bord avant
  l’instruction ciblée.
- **Non-but :** conditions flottantes ou mémoire, opérateurs relationnels,
  step-over/out et transport série.
- **Entrées/sources :** SPEC §10/16; contrat d’expression existant; état
  `TargetContext` du backend.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, tests de commandes et
  d’exécution.
- **Étapes réalisées :** parser l’expression après `if`; exposer `pc`, `fcsr`
  et les registres entiers numériques/ABI; considérer zéro comme faux et toute
  valeur non nulle comme vraie; afficher la présence de la condition dans
  `info break`.
- **Dépendances et tâches bloquées :** DBG-001; la sérialisation des conditions
  dans les projets/sessions et les conditions sur mémoire restent différées.
- **Tests :** condition fausse puis vraie sur `x1`; expression invalide;
  absence de mutation pendant l’évaluation.
- **Critères de sortie :** le breakpoint est ignoré tant que l’expression vaut
  zéro, puis arrête l’exécution avant l’instruction lorsqu’elle devient non
  nulle; l’erreur de syntaxe est stable.
- **Cas limites et échecs :** registre inconnu, parenthèse invalide, adresse
  non alignée et collision sont refusés sans mutation partielle.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** Rust, évaluateur d’expressions, débogage RV64.
- **Parallélisable :** oui avec QUAL-001; non avec une modification simultanée
  du contrat de breakpoint.
- **Paquet de contexte minimal :** ce sous-ensemble, SPEC §16 et les tests
  `conditional_breakpoint_*`.

#### DBG-001B — Step-over et step-out call-aware — TERMINÉ

- **Jalon / exigences :** M7; DBG-001..004, DBG-009, REQ-PROD-003.
- **But :** fournir `step-over`/`next` et `step-out` dans le moniteur host et
  le backend générique, avec une heuristique explicite basée sur `jal`/`jalr`
  et les adresses de retour `ra`.
- **Non-but :** DWARF, analyse complète des appels indirects, exécution
  asynchrone et transport série.
- **Entrées/sources :** SPEC §16; R1 chap. 2 sur `jal`/`jalr`; historique et
  contrat `TargetBackend`.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, tests du moniteur.
- **Étapes réalisées :** mémoriser les cadres lors de `jal ra,...`, les retirer
  sur `jalr x0,0(ra)`; exécuter un appel jusqu’à `pc+4` pour `step-over`; exécuter
  le cadre actif jusqu’à son adresse de retour pour `step-out`; réutiliser les
  breakpoints/watchpoints et le budget borné.
- **Dépendances et tâches bloquées :** DBG-001; les appels non conventionnels,
  tail calls, DWARF et la persistance de la pile restent différés.
- **Tests :** appel avec fonction et retour `jalr`, vérification des registres,
  PC de retour, pile vide après retour et refus de `step-out` sans cadre.
- **Critères de sortie :** l’appel est franchi sans exécuter l’instruction
  située à l’adresse de retour; `step-out` s’arrête à cette adresse; le budget
  et les arrêts existants restent effectifs.
- **Cas limites et échecs :** absence de cadre, boucle sans retour, breakpoint
  dans la fonction et dépassement du budget produisent un résultat borné.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** débogage RV64, conventions `ra`, tests Rust.
- **Parallélisable :** oui avec QUAL-001; non avec une modification concurrente
  de la représentation de l’historique.
- **Paquet de contexte minimal :** ce sous-ensemble, SPEC §16 et les tests
  `step_over_*`/`step_out_*`.

#### DBG-001C — Persister les conditions de breakpoint — TERMINÉ

- **Jalon / exigences :** M8; DBG-001..004, IO-001..009, OBS-001..006.
- **But :** conserver l’expression des breakpoints dans snapshots host,
  projets host et sessions du backend générique.
- **Non-but :** persister l’historique complet, la pile d’appels ou étendre le
  langage d’expressions.
- **Entrées/sources :** SPEC §§8/12/21/22; contrat de versionnement local des
  formats; AST d’expression de la ligne de commande.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, tests de round-trip.
- **Étapes réalisées :** sérialiser l’AST avec tags déterministes; restaurer
  les littéraux, symboles, opérateurs unaires/binaires et conditions host/
  backend; augmenter la version de persistance à 2; refuser les nœuds inconnus
  et les expressions imbriquées au-delà de 128 niveaux.
- **Dépendances et tâches bloquées :** DBG-001A, FORMAT-001; une migration
  automatique des fichiers version 1 reste différée et ceux-ci sont refusés
  explicitement comme versions incompatibles.
- **Tests :** snapshot host et session backend avec condition `if x1`, contrôle
  du marqueur `condition=expression`, troncature et validation des limites.
- **Critères de sortie :** save/load restitue une condition sémantiquement
  identique et l’évaluation après restauration conserve le même arrêt.
- **Cas limites et échecs :** version inconnue, AST tronqué, opérateur invalide
  ou profondeur excessive → diagnostic stable, sans mutation de la cible.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** formats binaires versionnés, Rust, tests de parser.
- **Parallélisable :** oui avec QUAL-001; non avec une modification simultanée
  du format snapshot/session.
- **Paquet de contexte minimal :** ce sous-ensemble, `ByteWriter`/
  `ByteReader`, SPEC §§12/21/22.

#### DBG-001D — Persister la pile de stepping — TERMINÉ

- **Jalon / exigences :** M8; DBG-009, IO-001..009, OBS-001..006.
- **But :** restaurer l’état minimal nécessaire à `step-out` après un snapshot
  host ou une session backend.
- **Non-but :** persister l’historique d’exécution, les baselines d’affichage
  ou une trace complète des appels indirects.
- **Entrées/sources :** SPEC §§8/12/16/21/22; `CallFrame` déduit de `jal`/
  `jalr`; contrat de versionnement local.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, tests de restauration.
- **Étapes réalisées :** sérialiser `return_pc` et `target` pour chaque cadre;
  restaurer la pile dans snapshots host et sessions backend; passer le format
  de persistance en version 3; réinitialiser la pile sur reset/restore cible.
- **Dépendances et tâches bloquées :** DBG-001B/C; migration automatique des
  formats antérieurs et persistance de l’historique restent différées.
- **Tests :** snapshot host puis `step-out`; session backend puis `step-out`;
  contrôle du retour à l’adresse `ra+4`.
- **Critères de sortie :** un cadre actif avant sauvegarde est encore actif
  après chargement et `step-out` s’arrête à la même adresse de retour.
- **Cas limites et échecs :** pile vide, fichier tronqué, profondeur excessive
  et version inconnue sont refusés sans mutation partielle.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** formats binaires versionnés, débogage RV64, Rust.
- **Parallélisable :** oui avec QUAL-001; non avec une modification concurrente
  des schémas snapshot/session.
- **Paquet de contexte minimal :** ce sous-ensemble, `CallFrame`,
  `ByteWriter`/`ByteReader`, SPEC §§12/16/22.

### FORMAT-001 — Projets, snapshots et replay — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** formats versionnés, état/symboles/breakpoints, intégrité et replay
  reproductible du périmètre v4.
- **Non-but :** ELF général.
- **Entrées/sources :** SPEC §§8/12/21/22; DECISIONS D-008/D-010.
- **Fichiers/modules :** `formats`, `app`, `memory`, `machine`.
- **Étapes réalisées :** sérialisation canonique des snapshots/projets/sessions;
  état machine et debugger; checksum déterministe; manifeste inspectable;
  replay `save → reset → load → save` byte-identique.
- **Dépendances/bloqués :** profile, assembler et debugger sont intégrés; le
  journal de commandes, la reprise après crash et la migration de schéma sont
  explicitement des fonctions ultérieures, pas des critères de FORMAT-001 v4.
- **Tests :** round-trip, checksum, corruption/troncature, manifeste host et
  backend, pile de stepping et replay byte-déterministe.
- **Acceptation :** E2E 12 restaure bytes/symboles/état/breakpoints identiques;
  un projet rechargé puis resauvegardé produit les mêmes octets et checksum.

#### FORMAT-001A — Manifeste d’intégrité déterministe — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, ISO-001..004.
- **But :** détecter les fichiers snapshot, projet et session tronqués ou
  accidentellement altérés avant tout décodage et toute mutation de cible.
- **Non-but :** signature cryptographique, chiffrement, migration multi-version
  ou transport série.
- **Entrées/sources :** SPEC §§12/18/21/22; format local v4.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, tests de persistance.
- **Étapes réalisées :** ajouter un checksum FNV-1a 64 bits déterministe en fin
  de chaque conteneur; vérifier magic, longueur et checksum dans `ByteReader`;
  refuser toute divergence avec `MON-PERSIST-038`; passer le format en version
  4.
- **Dépendances et tâches bloquées :** FORMAT-001; une signature ou un hash
  cryptographique reste explicitement hors périmètre.
- **Tests :** troncature, corruption du payload, round-trip snapshot/projet/
  session et absence de mutation après rejet.
- **Critères de sortie :** tout fichier produit par le moniteur est accepté
  après écriture; toute altération d’un octet est rejetée avant application.
- **Cas limites et échecs :** fichier trop court, checksum invalide, trailing
  bytes et magic inconnu produisent un diagnostic stable.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** formats binaires, checksums déterministes, Rust.
- **Parallélisable :** oui avec QUAL-001; non avec une modification concurrente
  de `ByteReader` ou du schéma v4.
- **Paquet de contexte minimal :** `ByteWriter`, `ByteReader`,
  `persistence_checksum`, SPEC §§12/18/22.
- **Limites/échecs :** version majeure, missing source, tampered hash → refusal with remedy.
- **Taille :** 5 points / 2,5 j, incertitude moyenne.
- **Compétences/outils :** formats canoniques, persistence.
- **Parallélisable :** oui avec UI-001 après contracts.
- **Contexte minimal :** SPEC §§12/21/22.

#### FORMAT-001B — Inspection de manifeste — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006.
- **But :** rendre l’intégrité et l’identité d’un fichier inspectables sans
  restaurer ni modifier la cible.
- **Non-but :** extraction de l’état complet, signature cryptographique ou
  transport série.
- **Entrées/sources :** SPEC §§12/18/21/22; conteneurs locaux v4.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, aide et tutoriel.
- **Étapes réalisées :** ajouter `manifest <path>` dans host/backend; détecter
  `RVSNAP01`, `RVPROJ01` et `RVSESS01`; valider le décodage et le checksum;
  afficher type, version, taille, checksum et état `integrity=valid`.
- **Dépendances et tâches bloquées :** FORMAT-001A; manifeste multi-profils,
  SBOM et signature de release restent dans REL-001.
- **Tests :** inspection host snapshot/projet et backend session; fichiers
  corrompus refusés par le même contrat d’intégrité.
- **Critères de sortie :** la commande ne change ni PC, ni registres, ni
  mémoire, et toute structure invalide est rejetée avant affichage.
- **Cas limites et échecs :** chemin absent, magic inconnu, taille minimale
  insuffisante et checksum invalide produisent des diagnostics stables.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** CLI, formats binaires, diagnostics Rust.
- **Parallélisable :** oui avec QUAL-001; non avec une modification concurrente
  de `ByteReader`.
- **Paquet de contexte minimal :** commande `manifest`, `ByteReader`,
  `persistence_checksum`, SPEC §§12/18/21.

#### FORMAT-001C — Replay de projet byte-déterministe — TERMINÉ

- **Jalon / exigences :** M8; IO-001..009, OBS-001..006, REQ-PROD-006.
- **But :** garantir qu’un projet sauvegardé puis rechargé et resauvegardé
  produit exactement le même conteneur canonique.
- **Non-but :** journal d’exécution complet, reverse debugging, migration de
  schéma ou transport série.
- **Entrées/sources :** SPEC §§8/12/21/22; format RVPROJ01 v4; checksum local.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, tests de persistance.
- **Étapes réalisées :** construire un projet avec source, symbole, breakpoint
  conditionnel et état machine; effectuer reset → project-load → project-save;
  comparer byte-à-byte les deux fichiers.
- **Dépendances et tâches bloquées :** FORMAT-001A/B; journal de commandes,
  trace d’exécution et replay temporel restent différés.
- **Tests :** round-trip projet byte-identique et manifestes valides avant/après.
- **Critères de sortie :** les octets, checksum et taille sont identiques après
  un cycle de restauration sans mutation supplémentaire.
- **Cas limites et échecs :** source vide, breakpoint conditionnel, pile
  d’appels active, fichier corrompu et mémoire modifiée restent diagnostiqués
  sans résultat non déterministe.
- **Taille :** 1 point / 0,5 journée-agent, incertitude faible.
- **Compétences/outils :** tests de formats binaires, Rust, reproductibilité.
- **Parallélisable :** oui avec QUAL-001; non avec une modification concurrente
  du schéma RVPROJ01.
- **Paquet de contexte minimal :** test `project_roundtrip_is_byte_deterministic`,
  `project_save_file`, `project_load_file`, SPEC §§12/21/22.

### UI-000A — Historique de commandes du shell host/QEMU — TERMINÉ

- **Jalon / exigences :** M6/M7; REQ-PROD-001, REQ-PROD-005, REQ-OBS-001.
- **But :** permettre le rappel borné des commandes dans les deux boucles interactives de `luna-app` avec `!!` et `!N`.
- **Non-but :** navigation fléchée ou édition de ligne native, historique persistant, modification de la grammaire du moniteur et TUI complète.
- **Entrées/sources :** SPEC §§10/17/21; contraintes d’isolation et de limites §18; `luna-app` host/QEMU.
- **Fichiers/modules :** `crates/app/src/main.rs`, tests binaires, `docs/TESTS.md`.
- **Étapes réalisées :** ajouter une capacité de 256 commandes; développer `!!` et `!N` 1-based; refuser références invalides avec codes `APP-SHELL-001..004`; partager le comportement avec les scripts non interactifs.
- **Dépendances et tâches bloquées :** commandes monitor existantes; l’édition de ligne et les raccourcis clavier restent dans UI-001.
- **Tests :** rappel dernier/numéroté, historique vide, référence invalide, limites et exécution workspace.
- **Critères de sortie :** une commande rappelée est exécutée exactement comme son texte original; aucune commande cible n’est envoyée en cas de référence invalide; l’historique est borné à 256 entrées.
- **Cas limites et échecs :** `!!` sans entrée, `!0`, `!N` hors plage, `!foo`, ligne vide et doublon consécutif.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** Rust stdio, tests de binaire, ergonomie shell.
- **Parallélisable :** oui avec la préparation de UI-001; non avec une modification concurrente de la boucle interactive.
- **Paquet de contexte minimal :** SPEC §§10/17/18/21, `crates/app/src/main.rs`, `crates/monitor/src/lib.rs`.

### UI-000B — Éditeur de ligne TTY avec navigation clavier — TERMINÉ

- **Jalon / exigences :** M6/M7; REQ-PROD-001, REQ-PROD-005, NFR-010.
- **But :** fournir dans les deux modes interactifs TTY une édition de ligne avec `↑/↓`, `←/→`, `Home/End`, `Backspace`, `Delete`, `Ctrl-C` et `Ctrl-D`.
- **Non-but :** TUI multipanneaux, coloration syntaxique, édition de source multi-lignes et comportement raw mode pour les scripts/pipes.
- **Entrées/sources :** SPEC §§10/17/18/20; `crossterm` 0.29.0; contrats host et backend QEMU.
- **Fichiers/modules :** `crates/app/src/main.rs`, `crates/app/Cargo.toml`, `Cargo.lock`.
- **Étapes réalisées :** détecter `stdin.is_terminal`; activer le raw mode seulement sur TTY; gérer insertion/suppression/navigation et resize; restaurer le terminal via guard RAII; conserver le chemin BufRead pour scripts.
- **Dépendances et tâches bloquées :** UI-000A; les panneaux, raccourcis fonctionnels et navigation diagnostics restent dans UI-001.
- **Tests :** compilation/test host et QEMU, suite workspace; chemin script inchangé; dépendance épinglée dans Cargo.lock.
- **Critères de sortie :** un TTY reçoit des lignes éditables sans caractères d’échappement visibles; Ctrl-D quitte proprement; une panne restaure le mode terminal; un pipe reste déterministe.
- **Cas limites et échecs :** terminal non interactif, resize, ligne vide, historique absent, Unicode de commande, échec activation raw mode.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** Rust stdio, Crossterm, états clavier et restauration terminal.
- **Parallélisable :** oui avec le modèle des panneaux UI; non avec une modification concurrente de la boucle `interactive_tty`.
- **Paquet de contexte minimal :** SPEC §§10/17/20, `crates/app/src/main.rs`, `crates/app/Cargo.toml`.

### UI-000C — Panneau dashboard déterministe host/QEMU — TERMINÉ

- **Jalon / exigences :** M6/M7; REQ-PROD-001, REQ-PROD-005, REQ-OBS-001.
- **But :** fournir une commande `dashboard`/`dash` qui regroupe position, registres et mémoire dans des sections lisibles et scriptables.
- **Non-but :** créer une UI plein écran, persister la mise en page ou ajouter des mutations implicites de la cible.
- **Entrées/sources :** SPEC §§14–16/21; modèles `Monitor` et `BackendConsole`; rendu mémoire hex/ASCII et registres exacts existants.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** composer `=== location ===`, `=== registers ===`, `=== memory ===`; réutiliser les renderers existants; exposer la commande et l’aide dans les deux consoles.
- **Dépendances et tâches bloquées :** REG-001D, UI-000B; les panneaux interactifs redimensionnables et raccourcis fonctionnels restent dans UI-001.
- **Tests :** sections présentes en host et backend après exécution; valeurs exactes et marqueurs conservés; suite workspace.
- **Critères de sortie :** `dashboard` ne modifie pas le PC ni la mémoire; les trois en-têtes sont stables; toute erreur mémoire reste explicitement visible.
- **Cas limites et échecs :** mémoire non mappée, cible distante indisponible, vue à la fin de RAM, terminal étroit.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** composition de vues, Rust, tests d’intégration.
- **Parallélisable :** oui avec le futur keymap; non avec une modification concurrente des renderers `regs`/`memory`.
- **Paquet de contexte minimal :** SPEC §§14–16, `crates/monitor/src/lib.rs`, `docs/TESTS.md`.

### UI-000D — Raccourcis clavier vers les commandes existantes — TERMINÉ

- **Jalon / exigences :** M6/M7; REQ-PROD-005, NFR-010, E2E 2/3/4.
- **But :** relier les touches fonctionnelles et panneaux aux commandes déjà contractuelles, dans les deux shells TTY.
- **Non-but :** prétendre fournir un step-over/step-out call-aware avant le modèle debugger correspondant; navigation fléchée de source multi-pane.
- **Entrées/sources :** SPEC §§15–17; raccourcis `F5`, `F10`, `F11`, `Ctrl+G`, `Ctrl+F`; UI-000C.
- **Fichiers/modules :** `crates/app/src/main.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** `F5→run`, `F10/F11→step`, `Ctrl+1→regs`, `Ctrl+2→memory`, `Ctrl+3→dashboard`, `Ctrl+G→view ` et `Ctrl+F→find `; conserver l’exécution par commande existante.
- **Dépendances et tâches bloquées :** UI-000B/UI-000C; le vrai step-over/step-out reste bloqué par la sémantique call-stack du debugger.
- **Tests :** table de mapping des touches, suites app/workspace et absence de nouveaux appels backend directs.
- **Critères de sortie :** chaque raccourci implémenté produit une commande connue; les raccourcis différés ne sont pas annoncés comme disponibles; les scripts restent inchangés.
- **Cas limites et échecs :** terminal non-TTY, touche inconnue, modificateur inattendu, ligne en cours préremplie pour `view`/`find`.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** Crossterm, key mapping, contrats commande.
- **Parallélisable :** oui avec la navigation de panneaux; non avec une modification concurrente de `shortcut_command`.
- **Paquet de contexte minimal :** SPEC §§15–17, `crates/app/src/main.rs`, `crates/monitor/src/lib.rs`.

### UI-000E — Sélection explicite des panneaux dashboard — TERMINÉ

- **Jalon / exigences :** M6/M7; REQ-PROD-005, NFR-010.
- **But :** permettre de rendre un seul panneau `location`, `regs` ou `memory`, ou l’ensemble avec `all`.
- **Non-but :** redimensionnement dynamique, disposition persistante et navigation source/diagnostic.
- **Entrées/sources :** SPEC §§14–17; UI-000C/UI-000D; commandes `dashboard`, `regs`, `memory`, `where`.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** parser l’argument optionnel; partager les renderers existants; fournir `MON-UI-001`/`MON-UI-101` pour les noms invalides; conserver `dashboard` sans argument comme `all`.
- **Dépendances et tâches bloquées :** UI-000C/UI-000D; le vrai layout redimensionnable reste dans UI-001.
- **Tests :** sélection host `regs`, sélection backend `memory`, sections `all`, erreur de panneau inconnu.
- **Critères de sortie :** chaque sélection produit exclusivement le panneau demandé; aucune sélection ne modifie PC ou registres; les alias `dash` et `where` restent compatibles.
- **Cas limites et échecs :** argument vide, casse non reconnue, mémoire non mappée et backend indisponible.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** parsing de commandes, composition de vues, tests Rust.
- **Parallélisable :** oui avec source/diagnostic; non avec une modification concurrente des signatures dashboard.
- **Paquet de contexte minimal :** SPEC §§14–17, `crates/monitor/src/lib.rs`, `docs/TESTS.md`.

### UI-000F — Navigation source et diagnostics structurés — TERMINÉ

- **Jalon / exigences :** M3/M6/M7; REQ-PROD-004, REQ-OBS-001, DIAG-001..006.
- **But :** conserver le dernier diagnostic, afficher le texte source numéroté et relier code/ligne/colonne à un extrait avec caret.
- **Non-but :** éditeur multi-lignes, correction automatique et navigation graphique entre plusieurs documents.
- **Entrées/sources :** SPEC §§11/17/19/21; `luna_diag::Diagnostic`; source lexer/assembler avec spans.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** ajouter `source [line]`, `diagnostic`/`diag`; capturer les erreurs host/backend; conserver le texte tenté lors d’un assemblage invalide; formater gravité, code, position, ligne et caret.
- **Dépendances et tâches bloquées :** UI-000E; la correction/navigation source interactive et le multi-document restent dans UI-001.
- **Tests :** assemblage invalide sans mutation machine; extrait source et caret; code/position; source host/backend; workspace.
- **Critères de sortie :** après erreur, `diagnostic` est consultable sans réexécuter la cible; `source N` renvoie une ligne stable; une opération réussie n’efface pas silencieusement le diagnostic précédent.
- **Cas limites et échecs :** aucun diagnostic, ligne hors plage, source vide, colonne absente, diagnostic sans span et backend distant en erreur.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** diagnostics structurés, spans Unicode, Rust formatting.
- **Parallélisable :** oui avec le modèle de document; non avec une modification concurrente de `Diagnostic` ou `source_text`.
- **Paquet de contexte minimal :** SPEC §§11/17/19/21, `crates/diag/src/lib.rs`, `crates/monitor/src/lib.rs`.

### UI-000G — Remèdes diagnostics et affichage automatique après erreur — TERMINÉ

- **Jalon / exigences :** M3/M6/M7; REQ-PROD-004, REQ-OBS-001, DIAG-001..006.
- **But :** associer aux familles de diagnostics fréquentes un remède stable et afficher automatiquement le diagnostic source après une erreur dans les shells host/QEMU.
- **Non-but :** appliquer automatiquement une correction, modifier la source sans commande explicite ou inférer un remède pour une norme inconnue.
- **Entrées/sources :** SPEC §§17/19/21; `luna_diag::Diagnostic`; UI-000F; codes ASM/CMD/MON existants.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, `crates/app/src/main.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** catalogue de remèdes par préfixe; ajout `remedy:` au rendu; affichage automatique de `diagnostic` après erreur dans les quatre boucles host/QEMU TTY/script.
- **Dépendances et tâches bloquées :** UI-000F; correction interactive et suggestions de patch restent différées à l’éditeur UI-001.
- **Tests :** erreur d’immédiat avec code, source, caret et remède; absence de mutation machine; tests app/workspace.
- **Critères de sortie :** une erreur ne disparaît pas derrière un message brut; le remède reste informatif; aucune erreur secondaire ne remplace le diagnostic original.
- **Cas limites et échecs :** code inconnu sans remède, diagnostic sans span, commande `diagnostic` elle-même sans historique, backend inaccessible.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** diagnostics, UX CLI, Rust closures et gestion d’erreur.
- **Parallélisable :** oui avec l’éditeur de source; non avec une modification concurrente de `format_diagnostic`.
- **Paquet de contexte minimal :** SPEC §§17/19/21, `crates/diag/src/lib.rs`, `crates/monitor/src/lib.rs`, `crates/app/src/main.rs`.

### UI-000H — Correction source explicite avant réassemblage — TERMINÉ

- **Jalon / exigences :** M3/M6/M7; REQ-PROD-003/004, REQ-OBS-001.
- **But :** permettre une correction contrôlée avec `source replace <ligne> <texte>`, sans appliquer automatiquement le résultat à la cible.
- **Non-but :** patch automatique basé sur le diagnostic, écriture mémoire implicite, assemblage silencieux et édition multi-document.
- **Entrées/sources :** SPEC §§11/17–19/21; UI-000F/UI-000G; transactions et isolation §18.
- **Fichiers/modules :** `crates/monitor/src/lib.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** parser ligne 1-based et texte restant; accepter texte entre guillemets et ligne vide; remplacer le document source uniquement; invalider le diagnostic; retourner l’instruction de réassembler explicitement.
- **Dépendances et tâches bloquées :** UI-000F; le patch proposé automatiquement et la confirmation graphique restent différés à UI-001.
- **Tests :** correction de ligne, diagnostic effacé, PC/mémoire inchangés, texte relu par `source`, erreurs de ligne hors plage.
- **Critères de sortie :** aucune commande `source replace` n’écrit la cible; seul `assemble`/`assemble-program` applique la nouvelle source.
- **Cas limites et échecs :** ligne inexistante, arité incomplète, texte vide, guillemets, source vide.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** édition textuelle transactionnelle, parsing de ligne, tests d’isolation.
- **Parallélisable :** oui avec l’éditeur multi-document; non avec une modification concurrente de `source_text`.
- **Paquet de contexte minimal :** SPEC §§11/17–19/21, `crates/monitor/src/lib.rs`, `docs/TESTS.md`.

### UI-000I — Blinkenlights registres et rendu mémoire CP437 — TERMINÉ

- **Jalon / exigences :** M6/M7; REQ-PROD-003/005, NFR-010.
- **But :** proposer une vue de registres façon blinkenlights et une vue texte CP437 optionnelle sans altérer les octets ni perdre l’adresse.
- **Non-but :** couleur ANSI obligatoire, animation temps réel, conversion de données ou émulation d’un terminal matériel Amiga.
- **Entrées/sources :** SPEC §§14/15/17/20; A1 ch.6–9 pour l’inspiration d’affichage; contrat renderer mémoire partagé.
- **Fichiers/modules :** `crates/monitor/src/register_view.rs`, `crates/monitor/src/memory_view.rs`, `crates/monitor/src/lib.rs`, `docs/TESTS.md`.
- **Étapes réalisées :** commandes `blinkenlights` et `regs blinkenlights`; panneau `dashboard blinkenlights`; `memory [addr] [count] cp437`; mapping CP437 étendu déterministe avec colonne hex inchangée; support host/backend.
- **Dépendances et tâches bloquées :** UI-000C/E; animation, navigation clavier dédiée et palette terminal restent différées.
- **Tests :** glyphes ASCII/CP437, bytes inchangés, panneaux host/backend, marqueurs de baseline des registres.
- **Critères de sortie :** même adresse et même hex avant/après changement de rendu; mode blinkenlights lisible en terminal sans couleur obligatoire; aucune mutation de la cible.
- **Cas limites et échecs :** octets de contrôle rendus par points en ASCII, glyphes CP437 étendus, terminal ne supportant pas Unicode.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** rendu texte, CP437, UX terminal, tests Rust.
- **Parallélisable :** oui avec UI-001; non avec une modification concurrente des renderers mémoire/registres.
- **Paquet de contexte minimal :** `crates/monitor/src/register_view.rs`, `crates/monitor/src/memory_view.rs`, `crates/monitor/src/lib.rs`, SPEC §§14/15/17.

### UI-001 — Frontend terminal et cycle ASM-One modernisé

- **Jalon / exigences :** M6–M9; REQ-PROD-001/005, E2E 2/3/4.
- **But :** connecter commandes, éditeur minimal, vues, registre, diagnostics et keymap.
- **Non-but :** frontend graphique ou compatibilité Amiga pixel-perfect.
- **Entrées/sources :** SPEC §§10/14–17; A1 ch.6–9.
- **Fichiers/modules :** `frontend-terminal`, `app`.
- **Étapes :** pane model; keymap; error navigation; Ctrl+Enter/F5=`run`/F10=`step-over`/F11=`step-out`; state highlighting; accessibility baseline.
- **Dépendances/bloqués :** CMD-001, MON-001, DBG-001, FORMAT-001 partiel.
- **Tests :** scripted terminal interaction, keymap coverage, no address loss, crash restore.
- **Acceptation :** démonstration M6/M7 sans mutation hors commande et clavier complet.
- **Limites/échecs :** terminal étroit, unicode, resize, unavailable color.
- **Taille :** 6 points / 3 j, incertitude élevée.
- **Compétences/outils :** terminal UI/accessibilité.
- **Parallélisable :** oui mais interface publique gelée; fusion avec M6/M7.
- **Contexte minimal :** SPEC §§14–17, A1.

## Priorités révisées après M9

Les tâches suivantes deviennent prioritaires avant la clôture publique de la
release. Elles n’ajoutent aucune extension ISA.

### BASIC-LOAD-001 — Figer l’ABI et la carte mémoire du MiniBASIC chargé — TERMINÉ

- **Priorité :** P0, première tâche.
- **But :** séparer le runtime MiniBASIC de l’image résidente et documenter
  l’entrée U-mode, la pile, les services ecall, les zones code/données,
  l’état initial et les codes de sortie.
- **Non-but :** réécrire immédiatement tout l’interpréteur en assembleur.
- **Entrées :** `docs/GUEST_PAYLOAD_ABI.md`, `docs/BASIC_LANGUAGE.md`,
  `crates/guest-monitor/src/minibasic.rs`, contrat ecall existant.
- **Dépendances :** aucune nouvelle dépendance ISA ; bloque BASIC-LOAD-002..006.
- **Tests :** fixture d’ABI, vérification des bornes workspace/pile et échec
  explicite d’un point d’entrée invalide.
- **Étapes réalisées :** contrat `RVMPAY01`, commande non mutante `info
  payload`, tailles de piles et fenêtres mémoire exposées depuis le guest,
  vérification QEMU intégrée au test `run-at`.
- **Acceptation :** contrat versionné, carte mémoire publiée, valeurs observées
  depuis l’image guest et aucun résultat dépendant d’une adresse codée en dur.
- **Taille :** 2 points / 1 journée-agent, incertitude moyenne.

### BASIC-LOAD-002 — Extraire et inspecter le squelette assembleur du runtime — TERMINÉ

- **Priorité :** P0.
- **But :** produire un squelette RV64 accepté par l’assembleur guest avec
  sections, labels, données et appels de services vérifiables.
- **Non-but :** prétendre que le désassemblage Rust constitue la source finale.
- **Entrées :** ABI BASIC-LOAD-001, `nm`/`objdump`, assembleur guest actuel.
- **Dépendances :** BASIC-LOAD-001 ; bloque BASIC-LOAD-003.
- **Tests :** `bash scripts/test-guest-payload-skeleton.sh`; assemble→load→
  `run-at`, labels, `jal`, `ld`/`sd`, ecall console/exit, listing et mémoire.
- **Étapes réalisées :** fixture `examples/minibasic-payload-skeleton.rv`,
  chargement sous QEMU, carte des symboles `entry`/`payload_exit`, et
  construction d’adresse distante par `auipc`/`addi` sans immédiat LUI signé.
- **Acceptation :** un payload assembleur minimal est chargé et exécuté sous
  QEMU, avec listing, carte de symboles et données observables reproductibles.
- **Taille :** 4 points / 2 journées-agent, incertitude élevée.

### BASIC-LOAD-003 — Ajouter les primitives assembleur du runtime BASIC

- **Priorité :** P0.
- **But :** porter lexer, parseur et évaluation dans des modules assembleur
  testables, en commençant par le mode direct et les expressions binary64.
- **Non-but de cette tranche :** implémenter immédiatement les chaînes,
  tableaux, fichiers ou compilation native. Les chaînes et tableaux restent
  toutefois des fonctionnalités obligatoires de la trajectoire et ne sont pas
  retirés du périmètre produit.
- **Entrées :** BASIC_LANGUAGE, oracle MiniBASIC résident temporaire, ISA D.
- **Dépendances :** BASIC-LOAD-002 ; bloque BASIC-LOAD-004/005.
- **Tests :** expressions, `PRINT`, `fadd.d`/`fdiv.d`, bits et fflags comparés
  au runtime résident et à une référence IEEE indépendante.
- **Acceptation :** aucun résultat n’est calculé par l’hôte ; le payload produit
  la sortie depuis ses propres données et instructions D.
- **Taille :** 8 points / 4 journées-agent, incertitude très élevée.

#### BASIC-LOAD-003A — Primitive binary64 D dans le payload — TERMINÉ

- **Priorité :** P0.
- **But :** rendre le dialecte guest capable d’assembler `fld`, `fsd`,
  `fsub.d` et `fmul.d`, puis d’exécuter une expression D dans U-mode.
- **Non-but :** lexer BASIC, conversion décimale, variables ou sortie texte.
- **Entrées :** R1 chapitre F/D, tables R2 `rv_d`, contrat `RVMPAY01`.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`, fixture
  `examples/minibasic-expression-d.rv`, test QEMU associé.
- **Dépendances :** BASIC-LOAD-002 ; prépare BASIC-LOAD-003B.
- **Tests :** chargement de 2, 3 et 4 en binary64, `fmul.d` (12), `fadd.d`
  (14), `fsub.d` (10), stockage et comparaison des motifs exacts/fcsr.
- **Acceptation :** le résultat `2+3*4` et les opérations intermédiaires sont
  produits par les instructions D du payload, sans calcul hôte ni sortie
  préenregistrée.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.

#### BASIC-LOAD-003A-ISA — Étendre le sous-ensemble assembleur pour le lexer — TERMINÉ

- **Priorité :** P0, en parallèle du portage des primitives D.
- **But :** fournir au runtime assembleur les loads/stores octet, demi-mot et
  mot, les opérations logiques/shift et les immédiats nécessaires à la lecture
  de texte et aux index internes.
- **Non-but :** accepter silencieusement tout l’ISA ; chaque mnémotechnique
  ajoutée doit rester vérifiée par les tables R2 générées.
- **Entrées :** R1 chapitres RV64I/M, tables R2 générées, ABI `RVMPAY01`.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`,
  `examples/minibasic-runtime-isa.rv`, `scripts/test-guest-runtime-isa.sh`.
- **Étapes réalisées :** `lb/lh/lw/ld`, `lbu/lhu/lwu`, `sb/sh/sw/sd`, R-type
  logique/shift/M et I-type logique/shift ; encodage construit depuis
  `GENERATED_OPCODES`, sans table manuelle.
- **Dépendances :** BASIC-LOAD-002A et BASIC-LOAD-003A ; prépare le lexer et
  le magasin de lignes assembleur.
- **Tests :** payload QEMU lit trois octets, vérifie les branches, stocke et
  recharge un mot, puis calcule le statut de sortie par opérations RV64.
- **Critères de sortie :** `scripts/test-guest-runtime-isa.sh` passe avec
  `target exit status=85`; le désassemblage et les symboles restent visibles.
- **Cas limites et échecs :** immédiat hors plage, shift supérieur à 63,
  registre invalide, mnémotechnique absente de R2 et accès cible hors RAM.
- **Écart à reproduire :** une fixture isolée avec `sd` à l’offset 1280 a
  produit `GUEST-ASM-008`, alors que les offsets 512/1024 utilisés ici
  passent. La cause exacte du parseur/encodeur guest doit être établie avant
  de déclarer toute la largeur signée 12 bits couverte ; la fixture tableau
  reste volontairement dans la fenêtre validée.
- **Taille :** 4 points / 2 journées-agent, incertitude faible.
- **Compétences/outils :** encodage RV64I/M, assembleur guest, QEMU et tests
  UART.
- **Parallélisable :** oui avec BASIC-LOAD-003B ; non avec une modification
  concurrente de `parse_source_instruction`.
- **Paquet de contexte minimal :** `main.rs`, `examples/minibasic-runtime-isa.rv`,
  `scripts/test-guest-runtime-isa.sh`, `BASIC_TBXL_NOTES.md`.

#### BASIC-LOAD-003B — Conversion binary64 vers affichage décimal cible — TERMINÉ

- **Priorité :** P0, prochaine sous-tranche.
- **But :** ajouter une conversion bornée target-side compatible avec le format
  V1, afin que `PRINT` puisse afficher un résultat sans aide de l’hôte.
- **Non-but :** dtoa général illimité ou compatibilité de toutes les locales.
- **Dépendances :** BASIC-LOAD-003A, ABI `write-buffer` ; bloque le lexer
  d’expressions et le `PRINT` chargé.
- **Tests :** `bash scripts/test-guest-decimal-print.sh`; `22/7`, six
  décimales fixes, boucles entières target-side, buffer ASCII et absence de
  conversion hôte.
- **Étapes réalisées :** support guest `fcvt.l.d`, `fcvt.d.l` et `sb`; routine
  de décomposition, arrondi positif borné, génération ASCII et `write-buffer`.
- **Acceptation :** le payload produit déterministement `3.142857` depuis
  binary64 et l’envoie uniquement par ecall.
- **Limites restantes :** négatifs, ±0, infinis, NaN, débordements, valeurs
  hors plage et raccord au lexer BASIC sont explicitement différés.
- **Taille :** 5 points / 2,5 journées-agent, incertitude élevée.

#### BASIC-LOAD-003C — Lexer cible et stockage de ligne — EN COURS

- **Priorité :** P0, première tranche chaînes.
- **But :** lire une ligne par l’ABI `RVMPAY01`, la stocker en mémoire cible et
  reconnaître un mot-clé sans interprétation hôte.
- **Non-but :** fournir encore les variables chaînes, tableaux, expressions ou
  dispatch complet des instructions BASIC.
- **Entrées :** `BASIC_LANGUAGE.md`, `BASIC_TBXL_NOTES.md`, ABI ecall, R1/R2
  pour les loads/stores et branches.
- **Fichiers/modules :** `examples/minibasic-runtime-lexer.rv`,
  `scripts/test-guest-runtime-lexer.sh`; futurs modules `lexer.rv` et
  `string_store.rv` du payload.
- **Dépendances :** BASIC-LOAD-002A, BASIC-LOAD-003A-ISA et ABI `RVMPAY01`.
- **Tests :** saisie réelle de `PRINT`, stockage octet par octet, comparaison
  cible des cinq caractères, sortie `OK` par `write-buffer`, statut d’échec
  distinct et exécution QEMU.
- **Critères de sortie :** `scripts/test-guest-runtime-lexer.sh` passe avec
  `target exit status=0`; aucun résultat n’est préenregistré et aucune analyse
  BASIC n’est effectuée par l’hôte.
- **Cas limites :** ligne vide, mot-clé inconnu, longueur dépassant le buffer,
  entrée interrompue et débordement d’index doivent être traités avant le
  portage du parser complet.
- **Taille :** 5 points / 2,5 journées-agent, incertitude élevée.
- **Parallélisable :** oui avec la définition du layout chaînes ; non avec une
  autre modification du buffer de payload.

#### BASIC-LOAD-003D — Primitive de buffer et descripteur de chaîne — TERMINÉE

- **Priorité :** P0, preuve de contrat avant le parser complet.
- **But :** valider en QEMU le stockage d’une chaîne ASCII en mémoire cible et
  sa transmission par l’ABI `write-buffer`, sans interpréteur hôte.
- **Non-but :** déclarer disponibles les variables chaînes, `DIM` ou les
  tableaux ; leur représentation complète est fixée par D-018.
- **Entrées :** D-018, `GUEST_PAYLOAD_ABI.md`, R1/R2 pour `lbu`, `sb`, `ld` et
  `sd`.
- **Fichiers/modules :** `examples/minibasic-runtime-string.rv`,
  `scripts/test-guest-runtime-string.sh`, `BASIC_LANGUAGE.md` et
  `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-003C et `BASIC-LOAD-003A-ISA`.
- **Tests :** entrée réelle `Hammurabi`, stockage octet par octet, arrêt sur
  LF, descripteur observable et statut de sortie contrôlé par QEMU.
- **Critères de sortie :** `bash scripts/test-guest-runtime-string.sh` passe,
  le motif du descripteur est vérifié en mémoire et aucune sortie n’est
  préenregistrée.
- **Cas limites :** buffer borné à 128 octets ; le chemin overflow produit
  `ERR` et un statut distinct. Le pool complet reste à tester dans
  `BASIC-STR-001`.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec la documentation du parser ; non avec une
  modification concurrente de l’ABI de chaînes.

#### BASIC-LOAD-003E — Lexer cible des chaînes et de DIM — TERMINÉE

- **Priorité :** P0, préparation du parser et du magasin d’objets.
- **But :** reconnaître dans un payload assembleur une affectation de chaîne
  et la forme d’une déclaration de tableau, puis construire un `StringDesc`
  cible sans analyse ni copie effectuée par l’hôte.
- **Non-but :** évaluer les expressions, exécuter `DIM`, indexer un tableau ou
  fournir encore le dispatch complet des instructions BASIC.
- **Entrées :** D-018, `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, R1/R2 pour
  les accès octet/mot et les branches.
- **Fichiers/modules :** `examples/minibasic-runtime-string-lexer.rv`,
  `scripts/test-guest-runtime-string-lexer.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-003C, BASIC-LOAD-003D et
  `BASIC-LOAD-003A-ISA`.
- **Tests :** ligne réelle avec identifiant chaîne de 16 caractères et
  `DIM A(10)`, copie du littéral, descripteur `{data_addr,length,capacity}`,
  reconnaissance target-side de la forme `DIM` et statut d’exécution QEMU.
- **Critères de sortie :** le script passe avec `target exit status=0`, le
  mot-clé et la forme d’index sont validés par des instructions exécutées en
  U-mode et aucune sortie attendue n’est injectée par l’hôte.
- **Cas limites :** guillemet absent, variable sans suffixe `$`, dimension
  non numérique, dimension négative ou ligne dépassant le buffer restent des
  erreurs à couvrir par le parser complet.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Parallélisable :** oui avec le design du parseur d’expressions ; non avec
  une modification concurrente du layout D-018.

#### BASIC-ARRAY-001A — Construire le descripteur de tableau cible — TERMINÉE

- **Priorité :** P0, prérequis de l’indexation.
- **But :** construire et relire en U-mode un `ArrayDesc` de rang 1 pour
  `DIM A(10)`, avec 11 éléments `binary64`, sans allocation hôte.
- **Non-but :** indexer, remplir ou redimensionner le tableau ; les tableaux
  de chaînes restent dans BASIC-ARRAY-001.
- **Entrées :** D-018, `BASIC_LANGUAGE.md`, R1/R2 pour `ld`, `sd`, `sb` et les
  branches.
- **Fichiers/modules :** `examples/minibasic-runtime-array.rv`,
  `scripts/test-guest-runtime-array.sh`, `BASIC_BUILD.md` et
  `BASIC_TEST_PLAN.md`.
- **Dépendances :** D-018 et `BASIC-LOAD-003A-ISA`.
- **Tests :** écriture/relecture de l’adresse de base et du nombre d’éléments,
  vérification de l’élément `binary64`, du rang, de la borne 10 et du dump de
  64 octets sous QEMU.
- **Critères de sortie :** le script passe avec `target exit status=0` et le
  dump correspond exactement au layout normatif.
- **Cas limites :** rang nul, dimension négative, produit de dimensions et
  index hors bornes sont réservés à BASIC-ARRAY-001.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le parseur d’expressions ; non avec une
  modification du layout D-018.

#### BASIC-ARRAY-001B — Indexer et protéger un tableau numérique — TERMINÉE

- **Priorité :** P0, première opération observable sur un tableau.
- **But :** calculer l’adresse row-major de `A(10)` pour des éléments
  `binary64`, écrire puis relire une valeur cible, et rejeter l’index 11 avant
  tout accès.
- **Non-but :** syntaxe complète de `DIM`, tableaux multidimensionnels,
  tableaux de chaînes ou redimensionnement.
- **Entrées :** D-018, `BASIC_LANGUAGE.md`, `BASIC-ARRAY-001A` et les
  instructions RV64I `slli`, `add`, `ld`, `sd`, `sltiu`.
- **Fichiers/modules :** `examples/minibasic-runtime-array.rv`,
  `scripts/test-guest-runtime-array.sh`, `BASIC_BUILD.md` et
  `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-ARRAY-001A et `BASIC-LOAD-003A-ISA`.
- **Tests :** motif binary64 `42.0`, offset `10 * 8`, relecture exacte,
  index égal au nombre d’éléments, absence d’écriture hors bornes et dump
  cible sous QEMU.
- **Critères de sortie :** le script passe avec `target exit status=0`; le
  payload ne lit ni ne calcule la valeur sur l’hôte.
- **Cas limites :** index 0, dernière dimension, rang supérieur à 1 et
  overflow de produit restent dans BASIC-ARRAY-001.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le parser d’expressions ; non avec une
  modification du descripteur.

#### BASIC-LOAD-003F — Première expression target-side avec précédence — TERMINÉE

- **Priorité :** P0, preuve d’intégration D avant le parser général.
- **But :** reconnaître les tokens de `2+3*4` en mémoire cible et exécuter la
  multiplication avant l’addition dans les registres flottants du payload.
- **Non-but :** nombres décimaux généraux, parenthèses, variables, divisions,
  comparaisons ou gestion complète des diagnostics.
- **Entrées :** `BASIC_LANGUAGE.md`, D-005/D-013, R1 chapitre F/D, R2 et
  `GUEST_PAYLOAD_ABI.md`.
- **Fichiers/modules :** `examples/minibasic-runtime-expression.rv`,
  `scripts/test-guest-runtime-expression.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-003A, BASIC-LOAD-003B et
  `BASIC-LOAD-003A-ISA`.
- **Tests :** octets des tokens, chargement de 2/3/4 en binary64, `fmul.d`
  donnant 12.0, `fadd.d` donnant 14.0, bits exacts, `fcsr=0` et stockage.
- **Critères de sortie :** arrêt QEMU sur `ebreak`, registres `f4/f5` et dump
  mémoire conformes au motif attendu, sans calcul Rust pendant l’exécution.
- **Cas limites :** ordre inversé, opérateur absent, parenthèses, `/`, division
  par zéro et flags sont réservés au parser général et à ses tests négatifs.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Parallélisable :** oui avec un oracle Rust indépendant ; non avec une
  modification concurrente de la convention des registres du payload.

#### BASIC-LOAD-003G — Division target-side et fflags — TERMINÉE

- **Priorité :** P0, fermeture du premier chemin arithmétique.
- **But :** lire `22/7`, convertir les chiffres en binary64 dans la cible,
  exécuter `fdiv.d` et rendre le motif du quotient ainsi que `fflags.NX`
  observables au débogueur.
- **Non-but :** parser général, nombres fractionnaires source, arrondi choisi
  par l’utilisateur, division par zéro et conversion décimale de sortie.
- **Entrées :** R1 chapitres F/D, D-005/D-013, R2 et politique `fcsr` de
  `BASIC_LANGUAGE.md`.
- **Fichiers/modules :** `examples/minibasic-runtime-expression-div.rv`,
  `scripts/test-guest-runtime-expression-div.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-003F et `BASIC-LOAD-003A`.
- **Tests :** quotient binary64 `0x4009249249249249`, `fcsr=1`, dump little
  endian et arrêt QEMU sur `ebreak`.
- **Critères de sortie :** le motif et le flag sont produits par les
  instructions du payload, sans opération flottante hôte pendant l’exécution.
- **Cas limites :** `±0`, infini, NaN, division exacte et division par zéro
  sont réservés à la matrice IEEE du parser complet.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec les tests différentiels Rust/QEMU ; non avec
  une modification de l’ABI des registres flottants.

#### BASIC-LOAD-003H — Littéraux entiers multi-chiffres target-side — TERMINÉE

- **Priorité :** P0, préparation du lexer numérique du payload assembleur.
- **But :** parcourir les chiffres de `12+3*4` dans la cible, construire le
  premier opérande par accumulation décimale entière, puis convertir les
  opérandes en `binary64` avant le calcul flottant.
- **Non-but :** fractions décimales, exposants, parenthèses, variables et
  parser général.
- **Entrées :** `BASIC_LANGUAGE.md`, D-005/D-013, R1 chapitres I et D/F, R2,
  et `GUEST_PAYLOAD_ABI.md`.
- **Fichiers/modules :** `examples/minibasic-runtime-expression-digits.rv`,
  `scripts/test-guest-runtime-expression-digits.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-003F et BASIC-LOAD-003A.
- **Tests :** octets source `12+3*4`, accumulation `1*10+2`, conversions
  `fcvt.d.l`, `fmul.d` donnant 12.0, `fadd.d` donnant 24.0, bits exacts et
  stockage little-endian observé sous QEMU.
- **Critères de sortie :** le payload s’arrête sur `ebreak`; `f1`, `f2`, `f4`,
  `f5` et la case mémoire de résultat correspondent aux motifs attendus; aucun
  calcul ou résultat n’est fourni par l’hôte.
- **Cas limites :** chiffre non valide, dépassement de l’accumulateur, signe,
  zéro initial et littéral vide sont réservés au lexer général et à ses tests
  négatifs.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Parallélisable :** oui avec l’oracle Rust et la conversion décimale; non
  avec une modification concurrente de l’ABI flottante.

#### BASIC-LOAD-003I — Littéral décimal signé target-side — TERMINÉE

- **Priorité :** P0, fermeture de la première tranche de lexer numérique.
- **But :** parcourir `-12.5` dans la cible, accumuler séparément partie
  entière et fraction, convertir en `binary64` et appliquer le signe avec les
  instructions D.
- **Non-but :** exposants, parenthèses, variables, débordements généraux et
  parser complet.
- **Entrées :** `BASIC_LANGUAGE.md`, D-005/D-013, R1 chapitres I et D/F, R2,
  `GUEST_PAYLOAD_ABI.md`.
- **Fichiers/modules :** `examples/minibasic-runtime-number.rv`,
  `scripts/test-guest-runtime-number.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-003H, BASIC-LOAD-003A et ABI `RVMPAY01`.
- **Tests :** signe `-`, accumulation `12`, fraction `5`, diviseur `10`,
  `fdiv.d`, `fadd.d`, `fsub.d`, motif `0xc029000000000000`, écriture mémoire
  little-endian et arrêt QEMU sur `ebreak`.
- **Critères de sortie :** le script passe avec `target exit status=0`; `f1`
  et la case mémoire sont produits par le payload, sans calcul hôte.
- **Cas limites :** `+12.5`, fraction vide, signe isolé, caractère non chiffre,
  fraction longue et dépassement de puissance de dix restent à ajouter au
  parser général et à ses diagnostics.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec la conception AST ; non avec une modification
  concurrente de la convention des registres flottants.

#### BASIC-LOAD-003J — Première table de variables numériques target-side — TERMINÉE

- **Priorité :** P0, prérequis du magasin BASIC cible.
- **But :** lire `X=12.5`, valider un nom `A..Z`, calculer son offset dans une
  table contiguë de 26 `binary64`, écrire puis relire la valeur en cible.
- **Non-but :** noms longs, affectations générales, magasin de lignes,
  contrôle de flot ou dispatch complet.
- **Entrées :** `BASIC_LANGUAGE.md`, D-005/D-013, R1 chapitres I et D/F, D-018,
  `GUEST_PAYLOAD_ABI.md`.
- **Fichiers/modules :** `examples/minibasic-runtime-variable.rv`,
  `scripts/test-guest-runtime-variable.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-003I, `BASIC-LOAD-003A-ISA` et ABI `RVMPAY01`.
- **Tests :** validation ASCII `A..Z`, index `X=23`, offset `184` octets,
  `fdiv.d`/`fadd.d`, relecture dans `f2`, dump little-endian sous QEMU et rejet
  d’un nom hors plage dans le chemin d’échec.
- **Critères de sortie :** le script passe avec `target exit status=0`; la
  table et le résultat proviennent exclusivement de la RAM et des instructions
  du payload cible.
- **Cas limites :** `A`, `Z`, nom vide, minuscule, nom de 16 caractères,
  table pleine et variable non initialisée restent à intégrer au parser général.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le magasin de lignes ; non avec une modification
  concurrente du layout de la table numérique.

#### BASIC-LOAD-004A — Insertion ordonnée dans le magasin de lignes target-side — TERMINÉE

- **Priorité :** P0, première sous-tranche de BASIC-LOAD-004.
- **But :** définir des enregistrements fixes `{line, length, body}` en RAM
  cible et insérer `20` puis `10` en conservant l’ordre croissant.
- **Non-but :** lexer complet des lignes, `LIST`, suppression, remplacement,
  exécution, variables longues et contrôle de flot.
- **Entrées :** `BASIC_LANGUAGE.md`, D-018, `GUEST_PAYLOAD_ABI.md`, R1 I et
  R2 pour `ld/sd/lbu/sb`, comparaisons et branches.
- **Fichiers/modules :** `examples/minibasic-runtime-lines.rv`,
  `scripts/test-guest-runtime-lines.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-003J, `BASIC-LOAD-002A` et ABI `RVMPAY01`.
- **Tests :** insertion hors ordre `20`/`10`, déplacement des champs, compteur
  égal à 2, corps `A`/`B`, dump des 64 octets et erreur de montage du payload.
- **Critères de sortie :** QEMU atteint `ebreak`; la RAM contient ligne 10 à
  l’offset 0, ligne 20 à l’offset 32 et le compteur 2 dans la cible.
- **Cas limites :** égalité, table pleine, suppression, remplacement, ligne
  zéro et numéro négatif sont réservés au magasin complet.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le lexer de ligne ; non avec une modification
  concurrente du layout des enregistrements.

#### BASIC-LOAD-004B — Lexer d’une ligne numérotée vers un record cible — TERMINÉE

- **Priorité :** P0, raccord lexer → magasin.
- **But :** parcourir `20 PRINT B` en RAM cible, accumuler le numéro décimal,
  copier le corps ASCII borné et écrire sa longueur dans un record.
- **Non-but :** lecture UART, plusieurs lignes, `LIST`, suppression,
  remplacement et exécution.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, R1 I et R2 pour
  branches, `lbu`, `sb`, `ld` et `sd`.
- **Fichiers/modules :** `examples/minibasic-runtime-line-lexer.rv`,
  `scripts/test-guest-runtime-line-lexer.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004A, BASIC-LOAD-003I et ABI `RVMPAY01`.
- **Tests :** source ASCII, numéro 20, longueur 7, corps `PRINT B`, dump du
  record et compteur target-side.
- **Critères de sortie :** QEMU atteint `ebreak`; le record cible contient les
  champs attendus et aucun parsing n’est réalisé par l’hôte.
- **Cas limites :** ligne vide, séparateur absent, numéro non numérique,
  longueur maximale, caractère nul dans le corps et ligne sans numéro sont
  réservés au lexer complet.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le test UART ; non avec une modification du
  layout `{line,length,body}`.

#### BASIC-LOAD-004C — Parser et insertion de plusieurs lignes target-side — TERMINÉE

- **Priorité :** P0, fermeture de l’insertion minimale.
- **But :** parser deux sources numérotées en cible (`20 PRINT B`, puis
  `10 PRINT A`), insérer dans l’ordre et déplacer le record complet, y compris
  son corps.
- **Non-but :** UART, capacité complète, `LIST`, suppression, remplacement et
  exécution des instructions BASIC.
- **Entrées :** `BASIC_LANGUAGE.md`, D-018, `GUEST_PAYLOAD_ABI.md`, R1 I et R2
  pour les branches et accès mémoire.
- **Fichiers/modules :** `examples/minibasic-runtime-line-input.rv`,
  `scripts/test-guest-runtime-line-input.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004A/B, `jal/jalr` guest et layout record 32 octets.
- **Tests :** deux numéros, deux longueurs de 7, corps `PRINT A`/`PRINT B`,
  déplacement complet, compteur 2 et dump target-side.
- **Critères de sortie :** la RAM contient la ligne 10 à l’offset 0 et la ligne
  20 à l’offset 32 avec leurs corps intacts ; QEMU atteint `ebreak`.
- **Cas limites :** égalité, troisième ligne, table pleine, suppression,
  remplacement et source vide restent dans BASIC-LOAD-004D/004E.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Parallélisable :** oui avec le pilote UART ; non avec une modification du
  protocole de retour `jalr` ou du layout record.

#### BASIC-LOAD-004D — Remplacement et suppression target-side — TERMINÉE

- **Priorité :** P0, édition minimale du magasin.
- **But :** remplacer le corps de la ligne 20, supprimer la ligne 10 par
  compactage du slot suivant et maintenir un compteur cohérent.
- **Non-but :** commande `LIST`, insertion de capacité générale, UART,
  exécution BASIC et annulation transactionnelle.
- **Entrées :** `BASIC_LANGUAGE.md`, D-018, layout record 32 octets et ABI
  payload guest.
- **Fichiers/modules :** `examples/minibasic-runtime-line-edit.rv`,
  `scripts/test-guest-runtime-line-edit.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004A/B/C et accès `ld/sd/lbu/sb`.
- **Tests :** table initiale `10 PRINT A`/`20 PRINT B`, remplacement par
  `PRINT C`, compactage, compteur égal à 1, dump exact et arrêt QEMU.
- **Critères de sortie :** la cible contient uniquement `20 PRINT C` dans le
  premier slot, avec longueur 7 et compteur 1 dans la zone de métadonnées.
- **Cas limites :** suppression de la dernière ligne, remplacement de longueur
  différente, table vide, ligne absente et corps maximal restent à couvrir.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec `LIST`; non avec une modification du layout.

#### BASIC-LOAD-004E — LIST target-side et sortie du magasin — TERMINÉE

- **Priorité :** P0, première commande observable du magasin cible.
- **But :** parcourir les records ordonnés, convertir les numéros de ligne,
  copier les corps et produire la sortie par `write-buffer`.
- **Non-but :** REPL, lecture UART, exécution des instructions BASIC, formatage
  complet des erreurs et annulation.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, D-018, R1 I/M et
  R2 pour `divu`, `remu`, loads/stores et ecall.
- **Fichiers/modules :** `examples/minibasic-runtime-list.rv`,
  `scripts/test-guest-runtime-list.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004A..D, ABI `write-buffer` et table record.
- **Tests :** deux records `10 PRINT A`/`20 PRINT B`, conversion ASCII
  target-side, longueur du buffer, sortie UART et statut de fin 0.
- **Critères de sortie :** QEMU observe exactement les deux lignes dans l’ordre
  et le programme termine par le service cible d’arrêt.
- **Cas limites :** table vide, numéro à 1/3 chiffres, corps vide, corps plein,
  buffer de sortie plein et compteur incohérent restent à traiter.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le REPL UART ; non avec une modification de
  l’ABI `write-buffer`.

#### BASIC-LOAD-004F — Lecture UART d’une ligne dans le payload — TERMINÉE

- **Priorité :** P0, frontière transport du REPL target-side.
- **But :** appeler `read-char` jusqu’au LF, stocker les octets dans la RAM
  cible et restituer la ligne par `write-buffer`.
- **Non-but :** parser le numéro, insertion, `LIST`, gestion des lignes vides,
  historique et buffering UART avancé.
- **Entrées :** `GUEST_PAYLOAD_ABI.md`, contrat `RVMPAY01`, R1 I et accès
  `lbu/sb`, services ecall 2/4.
- **Fichiers/modules :** `examples/minibasic-runtime-line-uart.rv`,
  `scripts/test-guest-runtime-line-uart.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004B/E, UART guest et ABI console.
- **Tests :** entrée réelle `20 PRINT B`, arrêt sur LF, longueur et sortie
  exacte target-side, statut de fin 0 sous QEMU.
- **Critères de sortie :** les octets reçus par UART sont ceux renvoyés par le
  payload ; aucune ligne n’est injectée dans la sortie par le script hôte.
- **Cas limites :** CRLF, ligne vide, buffer plein, EOF, interruption Ctrl-C et
  caractères non ASCII restent au raccord REPL complet.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le parser de ligne ; non avec une modification
  de l’ABI `read-char`/`write-buffer`.

#### BASIC-LOAD-004G — Première boucle verticale UART → record — TERMINÉE

- **Priorité :** P0, intégration du chemin utilisateur.
- **But :** recevoir une ligne UART, la stocker, parser son numéro et son corps,
  puis remplir un record BASIC cible avant de rendre la main au moniteur.
- **Non-but :** boucle REPL persistante, plusieurs lignes par lancement,
  `LIST`, `NEW`, `RUN`, historique et édition interactive complète.
- **Entrées :** `GUEST_PAYLOAD_ABI.md`, `BASIC_LANGUAGE.md`, D-018, contrats
  ecall 2/3 et layout record.
- **Fichiers/modules :** `examples/minibasic-runtime-repl-line.rv`,
  `scripts/test-guest-runtime-repl-line.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004B/E/F, `read-char`, `write-buffer` et retour
  contrôlé par `ecall exit`/breakpoint.
- **Tests :** séance réelle avec `20 PRINT B`, parsing, record en RAM,
  compteur 1, commandes `regs`/`memory` après arrêt et smoke QEMU.
- **Critères de sortie :** aucune commande de diagnostic n’est consommée comme
  donnée utilisateur ; le record target-side contient ligne 20 et `PRINT B`.
- **Cas limites :** ligne vide, CRLF, syntaxe invalide, buffer plein, Ctrl-C et
  reprise après erreur restent dans la boucle REPL complète.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Parallélisable :** oui avec `LIST` ; non avec une modification du protocole
  de séance UART.

#### BASIC-LOAD-004H — Deux tours REPL UART et insertion ordonnée — TERMINÉE

- **Priorité :** P0, première session multi-lignes target-side.
- **But :** recevoir deux lignes dans le même payload, conserver un compteur
  indépendant des temporaires lexer et insérer `20 PRINT B` puis `10 PRINT A`
  dans l’ordre cible.
- **Non-but :** boucle infinie, commandes textuelles, `LIST` interactif,
  `NEW`, `RUN`, historique et capacité non bornée.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, layout record,
  contrats UART `read-char` et interruptions de ligne.
- **Fichiers/modules :** `examples/minibasic-runtime-repl-two-lines.rv`,
  `scripts/test-guest-runtime-repl-two-lines.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004C/G et registre de compteur réservé.
- **Tests :** deux entrées UART, compteur 2, ligne 10 au slot 0, ligne 20 au
  slot 1, longueurs 7 et corps ASCII intacts.
- **Critères de sortie :** le payload ne consomme pas les commandes de
  diagnostic comme données et rend deux records corrects après `ebreak`.
- **Cas limites :** troisième ligne, ligne vide, erreur intermédiaire, Ctrl-C,
  table pleine et EOF restent dans la boucle REPL complète.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Parallélisable :** oui avec le renderer `LIST`; non avec une modification du
  protocole d’entrée UART.

#### BASIC-LOAD-004I — Dispatch target-side de la commande LIST — TERMINÉE

- **Priorité :** P0, première commande REPL réellement reconnue dans la cible.
- **But :** recevoir `LIST`, valider ses quatre caractères, sélectionner le
  parcours de records et produire la sortie par `write-buffer`.
- **Non-but :** parseur de commandes général, commandes invalides structurées,
  `NEW`, `RUN`, historique et boucle persistante sans borne.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, ABI UART/ecall,
  layout record et sortie LIST précédente.
- **Fichiers/modules :** `examples/minibasic-runtime-command-list.rv`,
  `scripts/test-guest-runtime-command-list.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004E/H, `read-char`, `write-buffer` et table
  target-side.
- **Tests :** ligne `LIST`, validation caractère par caractère, sortie de deux
  records dans l’ordre, statut de fin 0 et absence de parsing hôte.
- **Critères de sortie :** `LIST` déclenche la sortie cible exacte
  `10 PRINT A` puis `20 PRINT B`; une autre commande prend le chemin d’erreur.
- **Cas limites :** minuscules, suffixe, commande vide, CRLF, buffer plein et
  commande inconnue restent dans le parseur de commandes complet.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec `NEW` ; non avec une modification du protocole
  de dispatch.

#### BASIC-LOAD-004J — Dispatch target-side de la commande NEW — TERMINÉE

- **Priorité :** P0, réinitialisation explicite du magasin.
- **But :** recevoir `NEW`, valider ses caractères, effacer le compteur et les
  records bornés dans la RAM cible, puis produire `NEW OK` via `write-buffer`.
- **Non-but :** invite persistante, capacité finale, snapshots, annulation,
  gestion des commandes inconnues et `RUN`.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, layout record,
  ABI UART/ecall et contrat `LIST`.
- **Fichiers/modules :** `examples/minibasic-runtime-command-new.rv`,
  `scripts/test-guest-runtime-command-new.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004I, table target-side et `write-buffer`.
- **Tests :** commande `NEW`, réponse exacte, compteur nul, records nuls et
  statut de fin 0 sous QEMU.
- **Critères de sortie :** aucune donnée résiduelle des deux records n’est
  observable après la commande ; l’accusé est produit par le payload.
- **Cas limites :** table de capacité complète, commande minuscule, suffixe,
  CRLF, erreur d’E/S et réinitialisation répétée restent à couvrir.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec commandes invalides ; non avec une modification
  du layout de métadonnées.

#### BASIC-LOAD-004K — Boucle de commandes et erreur inconnue target-side — TERMINÉE

- **Priorité :** P0, robustesse minimale du REPL.
- **But :** maintenir la boucle après une commande inconnue, produire
  `ERR UNKNOWN`, puis accepter et exécuter `NEW` au tour suivant.
- **Non-but :** invite persistante finale, historique, `RUN`, `TRACE`, parsing
  général et interruption interactive.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, contrats ecall 2/3/4,
  dispatch `LIST`/`NEW`.
- **Fichiers/modules :** `examples/minibasic-runtime-command-loop.rv`,
  `scripts/test-guest-runtime-command-loop.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004I/J et UART target.
- **Tests :** séquence UART `BOGUS`, puis `NEW`, erreur target-side, accusé
  `NEW OK`, compteur nul et terminaison propre.
- **Critères de sortie :** l’erreur ne provoque ni sortie du payload ni
  consommation définitive de la session ; le second command est exécuté.
- **Cas limites :** commande vide, CRLF, erreurs répétées, Ctrl-C, EOF et
  buffer saturé restent à traiter.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le prompt `READY>` ; non avec une modification
  du protocole de boucle.

#### BASIC-LOAD-004L — Prompt READY target-side persistant — TERMINÉE

- **Priorité :** P0, visibilité du REPL cible.
- **But :** produire `READY> ` depuis le payload avant chaque commande, puis
  conserver la boucle après une erreur et accepter `NEW`.
- **Non-but :** historique, édition de ligne, `RUN`, `TRACE`, couleurs ou
  prompt fourni par le moniteur hôte.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, ABI
  `read-char`/`write-buffer` et BASIC-LOAD-004K.
- **Fichiers/modules :** `examples/minibasic-runtime-prompt.rv`,
  `scripts/test-guest-runtime-prompt.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004K et services console target-side.
- **Tests :** deux prompts, `BOGUS`, `ERR UNKNOWN`, `NEW`, `NEW OK`, sortie
  target-side et absence de prompt injecté par le script.
- **Critères de sortie :** au moins deux occurrences de `READY> ` sont produites
  par le payload dans une seule séance QEMU.
- **Cas limites :** prompt interrompu, ligne vide, CRLF, sortie pleine et
  interruption Ctrl-C restent à couvrir.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le dispatch `RUN` ; non avec une modification
  de l’ABI console.

#### BASIC-LOAD-005A — RUN minimal de PRINT entier — TERMINÉE

- **Priorité :** P0, premier consommateur du magasin.
- **But :** recevoir `RUN`, vérifier un record `10 PRINT 2+3`, calculer la somme
  dans les registres entiers cible et produire `5` par `write-buffer`.
- **Non-but :** expressions binary64, variables, contrôle de flot, plusieurs
  lignes exécutées et diagnostics complets.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, layout record,
  dispatch UART et services console.
- **Fichiers/modules :** `examples/minibasic-runtime-command-run-print.rv`,
  `scripts/test-guest-runtime-command-run-print.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-004E/L, `read-char`, `write-buffer` et record
  source cible.
- **Tests :** commande `RUN`, reconnaissance `PRINT`, lecture des chiffres,
  addition target-side, sortie `5\n` et statut 0 sous QEMU.
- **Critères de sortie :** le résultat n’est ni dans l’entrée ni dans le script
  hôte ; il est calculé par les instructions du payload.
- **Cas limites :** expression flottante, variable, corps vide, instruction
  inconnue, plusieurs records et erreur d’exécution restent à couvrir.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le parser D ; non avec une modification du
  layout record ou de l’ABI console.

#### BASIC-LOAD-005B — RUN expression binary64 avec fadd.d — TERMINÉE

- **Priorité :** P0, première évaluation flottante du chemin RUN.
- **But :** convertir les opérandes entiers du record `PRINT 2+3` en
  `binary64`, exécuter `fadd.d` dans la cible et rendre le motif `5.0`
  observable par le débogueur.
- **Non-but :** formatage décimal général, variables, parenthèses, division,
  contrôle de flot et gestion complète des `fflags`.
- **Entrées :** R1 F/D, R2 généré, `BASIC_LANGUAGE.md`, ABI payload et D-019
  seulement comme extension future non prioritaire.
- **Fichiers/modules :** `examples/minibasic-runtime-command-run-fadd-d.rv`,
  `scripts/test-guest-runtime-command-run-fadd-d.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-005A, `fadd.d`, `fcvt.d.l`, breakpoint guest et
  mémoire cible.
- **Tests :** commande RUN, `f3=0x4014000000000000`, dump little-endian de 5.0,
  arrêt `ebreak` et inspection des registres flottants.
- **Critères de sortie :** le résultat est produit par `fadd.d` target-side et
  non par le script, l’hôte ou une chaîne préenregistrée.
- **Cas limites :** `fsub.d`, `fmul.d`, `fdiv.d`, ±0, NaN, flags, fractions et
  variables restent à intégrer au parser/exécuteur général.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Parallélisable :** oui avec le parser des variables ; non avec une
  modification de la convention des registres flottants.

#### BASIC-LOAD-005C — RUN lecture d’une variable numérique target-side — TERMINÉE

- **Priorité :** P0, raccordement de la table de variables au chemin RUN.
- **But :** reconnaître le record `10 PRINT X`, valider l’identifiant `X`,
  calculer son index `23`, charger la valeur depuis la table cible et la rendre
  observable dans un registre flottant et en mémoire.
- **Non-but :** affectations, expressions composées, noms de plus d’un
  caractère, formatage décimal, contrôle de flot et dispatch général.
- **Entrées :** `BASIC_LANGUAGE.md`, D-018, `GUEST_PAYLOAD_ABI.md`, R1
  chapitre F et D, R2 généré.
- **Fichiers/modules :** `examples/minibasic-runtime-command-run-variable.rv`,
  `scripts/test-guest-runtime-command-run-variable.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-005A/B, table target-side des 26 variables et
  instructions `fld`/`fsd`.
- **Tests :** commande `RUN`, record `PRINT X`, `f1=0x4014000000000000`, dump
  little-endian exact de `5.0`, breakpoint `ebreak` et statut QEMU 0.
- **Critères de sortie :** la valeur est lue et chargée par le payload RV ; le
  script hôte ne calcule ni ne fournit le résultat.
- **Cas limites :** `A`/`Z`, variable non initialisée, minuscule, nom long,
  table absente et erreur de syntaxe restent à traiter par l’exécuteur général.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, ABI payload, QEMU et inspection
  des registres flottants.
- **Parallélisable :** oui avec le lexer d’identifiants et le formatage
  binary64 ; non avec une modification du layout de la table.
- **Paquet de contexte :** BASIC-LOAD-005A/B, `BASIC_LANGUAGE.md`,
  `GUEST_PAYLOAD_ABI.md`, fixture de table `variable.rv` et contrat `fld`/`fsd`.

#### BASIC-LOAD-005D — RUN expression variable avec fadd.d — TERMINÉE

- **Priorité :** P0, premier opérateur binaire combinant une variable et un
  littéral dans le payload.
- **But :** évaluer le corps `PRINT X+3`, lire `X` depuis la table cible,
  convertir `3` en binary64, exécuter `fadd.d` et exposer `8.0`.
- **Non-but :** parser général, parenthèses, précédence complète, autres
  opérateurs, affectations, formatage décimal et contrôle de flot.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 F/D, R2 généré, `GUEST_PAYLOAD_ABI.md`
  et le layout target-side des lignes/variables.
- **Fichiers/modules :** `examples/minibasic-runtime-command-run-variable-add.rv`,
  `scripts/test-guest-runtime-command-run-variable-add.sh`, documentation BASIC.
- **Dépendances :** BASIC-LOAD-005C, `fcvt.d.l`, `fadd.d`, `fld`/`fsd` et
  breakpoint guest.
- **Tests :** QEMU réel, `f3=0x4020000000000000`, dump exact de `8.0`, arrêt
  `ebreak` et statut 0.
- **Critères de sortie :** ni `X`, ni `8.0` ne sont fournis par le script hôte ;
  le calcul est réalisé par les instructions du payload.
- **Cas limites :** soustraction, multiplication, division, variables A/Z,
  espaces, parenthèses, comparaison et erreurs restent à intégrer.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, extension D, QEMU et inspection
  des registres flottants.
- **Parallélisable :** oui avec le lexer d’expressions ; non avec un changement
  du contrat de table numérique.
- **Paquet de contexte :** BASIC-LOAD-005B/C, fixtures `run-fadd-d` et
  `run-variable`, `BASIC_LANGUAGE.md` et `GUEST_PAYLOAD_ABI.md`.

#### BASIC-LOAD-005E — RUN expression variable avec fsub.d — TERMINÉE

- **Priorité :** P0, deuxième opérateur binaire target-side.
- **But :** évaluer `PRINT X-3`, charger `X=5.0`, convertir `3`, exécuter
  `fsub.d` et exposer le résultat `2.0`.
- **Non-but :** parser général, parenthèses, multiplication, division,
  formatage décimal et contrôle de flot.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 F/D, R2 généré et
  `GUEST_PAYLOAD_ABI.md`.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation ajoutée
  dans `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005D, `fsub.d`, `fcvt.d.l`, `fld`/`fsd`.
- **Tests :** breakpoint QEMU, `f3=0x4000000000000000`, dump exact de `2.0`
  et statut 0.
- **Critères de sortie :** le calcul et le résultat sont produits dans le
  payload RV, sans résultat injecté par l’hôte.
- **Cas limites :** signe négatif, zéro signé, NaN, infini et sous-normal
  seront traités par la matrice flottante générale.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, extension D et QEMU.
- **Parallélisable :** oui avec la préparation du parser ; non avec un
  changement de convention des registres flottants.
- **Paquet de contexte :** BASIC-LOAD-005C/D, fixtures `run-variable` et
  `run-variable-add`, et règles flottantes de `BASIC_LANGUAGE.md`.

#### BASIC-LOAD-005F — RUN expression variable avec fmul.d — TERMINÉE

- **Priorité :** P0, troisième opérateur binaire target-side.
- **But :** évaluer `PRINT X*3`, charger `X=5.0`, convertir `3`, exécuter
  `fmul.d` et exposer le résultat `15.0`.
- **Non-but :** parser général, précédence, division, formatage décimal et
  contrôle de flot.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 F/D, R2 généré et
  `GUEST_PAYLOAD_ABI.md`.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005E, `fmul.d`, `fcvt.d.l`, `fld`/`fsd`.
- **Tests :** breakpoint QEMU, `f3=0x402e000000000000`, dump exact de `15.0`
  et statut 0.
- **Critères de sortie :** le calcul est réalisé par le payload RV, sans
  résultat fourni ou calculé par le script hôte.
- **Cas limites :** zéro signé, infinis, NaN, sous-normaux et arrondis seront
  couverts par la matrice flottante générale.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, extension D et QEMU.
- **Parallélisable :** oui avec le lexer d’expressions ; non avec un changement
  de la convention des registres flottants.
- **Paquet de contexte :** BASIC-LOAD-005D/E, fixtures `run-variable-add` et
  `run-variable-sub`, `BASIC_LANGUAGE.md` et `GUEST_PAYLOAD_ABI.md`.

#### BASIC-LOAD-005G — RUN expression variable avec fdiv.d — TERMINÉE

- **Priorité :** P0, quatrième opérateur binaire target-side.
- **But :** évaluer `PRINT X/2`, charger `X=5.0`, convertir `2`, exécuter
  `fdiv.d` et exposer le résultat `2.5`.
- **Non-but :** détection générale de division par zéro, parser complet,
  formatage décimal, comparaisons et contrôle de flot.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 F/D, R2 généré et
  `GUEST_PAYLOAD_ABI.md`.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005F, `fdiv.d`, `fcvt.d.l`, `fld`/`fsd`.
- **Tests :** breakpoint QEMU, `f3=0x4004000000000000`, dump exact de `2.5`
  et statut 0.
- **Critères de sortie :** le quotient est calculé par le payload RV et non
  par le script hôte.
- **Cas limites :** zéro, ±0, infinis, NaN, flags et division par zéro seront
  traités par l’évaluateur général.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, extension D et QEMU.
- **Parallélisable :** oui avec le lexer d’expressions ; non avec un changement
  du contrat de registres flottants.
- **Paquet de contexte :** BASIC-LOAD-005E/F, fixtures des trois opérateurs
  précédents, `BASIC_LANGUAGE.md` et `GUEST_PAYLOAD_ABI.md`.

#### BASIC-LOAD-005H — RUN division binary64 par zéro et fflags.DZ — TERMINÉE

- **Priorité :** P0, verrouillage du cas IEEE avant le diagnostic BASIC.
- **But :** exécuter `PRINT X/0` dans la cible, observer `+inf` et vérifier que
  `fdiv.d` positionne `fflags.DZ` dans `fcsr`.
- **Non-but :** message utilisateur, récupération REPL, classification
  complète des opérandes et politique générale de division par zéro.
- **Entrées :** R1 F/D, politique flottante de `BASIC_LANGUAGE.md`, R2 généré
  et `GUEST_PAYLOAD_ABI.md`.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005G, `fdiv.d`, registres `fcsr`/`fflags` et
  inspection des registres flottants.
- **Tests :** breakpoint QEMU, `f3=0x7ff0000000000000`, `fcsr=0x8`, dump
  exact `+inf` et statut 0.
- **Critères de sortie :** le résultat et le flag viennent de l’exécution RV,
  sans calcul ou injection par le test hôte.
- **Cas limites :** `0/0`, signe du zéro, infini, NaN et message BASIC restent
  à traiter par l’évaluateur général.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** IEEE 754, extension D, QEMU et débogueur guest.
- **Parallélisable :** oui avec le diagnostic parser ; non avec une modification
  de la politique `fflags`.
- **Paquet de contexte :** BASIC-LOAD-005F/G, fixture `run-variable-div`,
  `BASIC_LANGUAGE.md` et règles R1 F/D sur `DZ`.

#### BASIC-LOAD-005I — Diagnostic target-side de division par zéro — TERMINÉE

- **Priorité :** P0, premier chemin d’erreur visible depuis le payload.
- **But :** après `fdiv.d` avec diviseur nul, émettre `ERR DIV0` par le service
  `write-buffer` et terminer proprement avec le statut cible 0.
- **Non-but :** retour à l’invite REPL générale, code d’erreur structuré,
  lecture directe de `fcsr` par le langage assembleur et autres diagnostics.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, R1 F/D et ABI
  `write-buffer`.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005H, `fdiv.d` et service target-side 4.
- **Tests :** sortie réelle `ERR DIV0`, `target exit status=0`, absence de
  sortie préenregistrée et statut QEMU valide.
- **Critères de sortie :** le message est stocké et transmis par le payload,
  pas imprimé par le script hôte.
- **Cas limites :** `0/0`, signe du zéro, ligne BASIC et reprise après erreur
  restent à intégrer dans l’exécuteur général.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, ABI console et QEMU.
- **Parallélisable :** oui avec le support CSR ; non avec la boucle REPL finale.
- **Paquet de contexte :** BASIC-LOAD-005H, fixture `divzero`,
  `GUEST_PAYLOAD_ABI.md` et règles de diagnostics BASIC.

#### BASIC-LOAD-005J — Cas IEEE `0/0`, NaN et fflags.NV — TERMINÉE

- **Priorité :** P0, couverture du second résultat exceptionnel de division.
- **But :** exécuter `PRINT X/0` avec `X=0.0`, vérifier le NaN quiet canonique
  et le positionnement de `fflags.NV` dans `fcsr`.
- **Non-but :** diagnostic BASIC, lecture CSR dans l’assembleur, conservation
  des payloads NaN utilisateur et parser général.
- **Entrées :** R1 F/D, politique flottante de `BASIC_LANGUAGE.md`, R2 généré
  et affichage exact des registres guest.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005H, `fdiv.d`, registre `fcsr` et convention
  RISC-V des bits `NV/DZ/OF/UF/NX`.
- **Tests :** breakpoint QEMU, `f3=0x7ff8000000000000`, `fcsr=0x10`, dump
  exact du NaN et statut d’exécution vérifié.
- **Critères de sortie :** le NaN et le flag viennent de l’exécution cible ;
  aucune valeur n’est injectée par le script.
- **Cas limites :** NaN signalant, signes des zéros, infinis et propagation
  des payloads restent à couvrir.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** IEEE 754, RISC-V F/D, QEMU et inspection `fcsr`.
- **Parallélisable :** oui avec les tests de littéraux spéciaux ; non avec une
  modification de la convention de flags.
- **Paquet de contexte :** BASIC-LOAD-005H/I, fixtures `divzero`,
  `BASIC_LANGUAGE.md` et R1 chapitre F.

#### BASIC-LOAD-005K — Signe d’une variable binary64 dans RUN — TERMINÉE

- **Priorité :** P0, couverture du signe dans le chemin flottant target-side.
- **But :** charger `X=-5.0`, exécuter `PRINT X/2` avec `fdiv.d` et vérifier
  le résultat `-2.5` jusque dans le registre et la mémoire cible.
- **Non-but :** parsing de littéraux négatifs, négation unaire, zéro signé,
  NaN signés et formatage décimal.
- **Entrées :** R1 F/D, `BASIC_LANGUAGE.md`, layout de table numérique et
  affichage raw bits du débogueur.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005G/J, `fld`, `fdiv.d`, mémoire binary64 et
  inspection des registres flottants.
- **Tests :** breakpoint QEMU, `f3=0xc004000000000000`, dump little-endian
  exact de `-2.5` et statut d’exécution vérifié.
- **Critères de sortie :** le signe provient de la valeur chargée par le
  payload et le quotient est calculé par `fdiv.d` target-side.
- **Cas limites :** `-0.0`, division par `-0.0`, infinis et NaN restent à
  couvrir dans la matrice IEEE générale.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** IEEE 754 binary64, RISC-V D et QEMU.
- **Parallélisable :** oui avec les tests de littéraux signés ; non avec une
  modification du layout des variables.
- **Paquet de contexte :** BASIC-LOAD-005G/J, fixtures `variable-div` et
  `zerozero`, `BASIC_LANGUAGE.md` et R1 chapitre F.

#### BASIC-LOAD-005L — Conservation du zéro négatif dans RUN — TERMINÉE

- **Priorité :** P0, couverture du signe zéro IEEE dans l’évaluation cible.
- **But :** charger `X=-0.0`, exécuter `PRINT X/2` et vérifier `-0.0` dans le
  registre flottant et la mémoire, sans flag parasite.
- **Non-but :** syntaxe des littéraux spéciaux, négation unaire, division par
  zéro, formatage décimal et parser général.
- **Entrées :** R1 F/D, `BASIC_LANGUAGE.md`, layout de table numérique et
  affichage raw bits du débogueur.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005K, `fld`, `fdiv.d`, représentation binary64
  et inspection `fcsr`.
- **Tests :** breakpoint QEMU, `f3=0x8000000000000000`, `fcsr=0`, dump
  little-endian exact et statut d’exécution vérifié.
- **Critères de sortie :** le signe zéro est issu de la donnée cible et le
  quotient est exécuté par `fdiv.d`.
- **Cas limites :** `+0/-0`, `-0/-0`, addition de zéros signés et arrondis
  dirigés restent à couvrir.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** IEEE 754 binary64, RISC-V D et QEMU.
- **Parallélisable :** oui avec les tests de littéraux spéciaux ; non avec une
  modification du layout des variables.
- **Paquet de contexte :** BASIC-LOAD-005K, fixtures `negative-div` et
  `zerozero`, `BASIC_LANGUAGE.md` et R1 chapitre F.

#### BASIC-LOAD-005M — Signe du zéro dénominateur dans RUN — TERMINÉE

- **Priorité :** P0, couverture du signe des résultats infinis IEEE.
- **But :** exécuter `+5.0/-0.0`, vérifier `-inf` et `fflags.DZ` dans la cible.
- **Non-but :** syntaxe des littéraux spéciaux, diagnostic BASIC, parser
  général, payloads NaN et traitement utilisateur du zéro signé.
- **Entrées :** R1 F/D, `BASIC_LANGUAGE.md`, représentation raw bits et
  affichage `fcsr` du moniteur.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005L, `fld`, `fdiv.d`, table binary64 et
  inspection des registres flottants.
- **Tests :** breakpoint QEMU, `f3=0xfff0000000000000`, `fcsr=0x8`, dump
  exact de `-inf` et statut d’exécution vérifié.
- **Critères de sortie :** le signe du dénominateur est fourni par la donnée
  cible et le résultat est calculé par `fdiv.d`.
- **Cas limites :** `-0/-0`, `+0/-0`, `0/0` et propagation des signes NaN
  restent couverts par les scénarios IEEE dédiés.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** IEEE 754 binary64, RISC-V D et QEMU.
- **Parallélisable :** oui avec les tests de littéraux spéciaux ; non avec une
  modification de la convention `fflags`.
- **Paquet de contexte :** BASIC-LOAD-005J/L, fixtures `zerozero` et
  `negative-zero-div`, `BASIC_LANGUAGE.md` et R1 chapitre F.

#### BASIC-LOAD-005N — RUN addition de deux variables target-side — TERMINÉE

- **Priorité :** P0, premier calcul avec deux opérandes de la table numérique.
- **But :** charger `X=5.0` et `Y=3.0`, évaluer `PRINT X+Y` avec `fadd.d` et
  rendre les deux opérandes ainsi que `8.0` observables au débogueur.
- **Non-but :** parser général, précédence, parenthèses, variables longues,
  affectation et formatage décimal.
- **Entrées :** R1 F/D, `BASIC_LANGUAGE.md`, layout de table A..Z et
  convention d’affichage raw bits.
- **Fichiers/modules :** fixture et test QEMU dédiés ; documentation dans
  `BASIC_BUILD.md` et `BASIC_TEST_PLAN.md`.
- **Dépendances :** BASIC-LOAD-005C/D, `fld`, `fadd.d` et table
  target-side des variables.
- **Tests :** `f1=5.0`, `f2=3.0`, `f3=8.0`, dump exact et breakpoint QEMU.
- **Critères de sortie :** les deux valeurs et le résultat proviennent du
  payload RV, sans calcul de l’hôte.
- **Cas limites :** A/Z, opérandes identiques, valeurs non initialisées,
  addition de signes et expressions de longueur générale restent à intégrer.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, table binary64, extension D et QEMU.
- **Parallélisable :** oui avec le lexer d’identifiants ; non avec une
  modification du layout variable.
- **Paquet de contexte :** BASIC-LOAD-005C/D, fixtures `run-variable-add` et
  `run-variable-negative-div`, `BASIC_LANGUAGE.md` et R1 chapitre F.

#### BASIC-LOAD-005O — Premier noyau REPL assembleur intégré — TERMINÉE

- **Priorité :** P0, première tranche verticale du portage complet.
- **But :** assembler la source `.rv` dans le moniteur, lire une ligne BASIC
  target-side, la stocker, exécuter `LIST`/`NEW`/`RUN` et atteindre `fadd.d`
  avec deux variables.
- **Non-but :** parseur général, affectations, formatage décimal, contrôle de
  flot complet, chaînes/tableaux et remplacement immédiat du binaire Rust.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, R1 F/D, R2 généré
  et `MINIBASIC_ASM_SOURCE_GUIDE.md`.
- **Fichiers/modules :** `examples/minibasic-asm/payload-repl.rv`,
  `scripts/test-guest-runtime-asm-repl.sh` et documentation BASIC.
- **Dépendances :** BASIC-LOAD-004L, BASIC-LOAD-005N, ABI console et
  assembleur source du moniteur.
- **Tests :** QEMU réel, saisie `10 PRINT X+Y`, sortie `LIST`, breakpoint,
  `f1=5.0`, `f2=3.0`, `f3=8.0` et dump exact en RAM.
- **Critères de sortie :** aucun résultat n’est fourni par l’hôte ; le payload
  assembleur assure la boucle de commande, le stockage et le calcul.
- **Cas limites :** ligne invalide, plusieurs lignes, expressions générales,
  mémoire pleine, reprise après erreur et interruption restent à intégrer.
- **Taille :** 5 points / 2,5 journées-agent, incertitude moyenne.
- **Compétences/outils :** assembleur guest, ABI ecall, QEMU et débogage D.
- **Parallélisable :** lexer général et formatage peuvent avancer en parallèle ;
  le layout des records et la boucle de commande doivent rester synchronisés.
- **Paquet de contexte :** `payload-repl.rv`, guide assembleur, fixtures
  `run-variable` et contrat `RVMPAY01`.

#### BASIC-LOAD-005P — Dispatch des quatre opérateurs dans le payload REPL — TERMINÉE

- **Priorité :** P0, première sélection d’opérateur depuis une ligne stockée.
- **But :** lire l’octet opérateur de `PRINT X<op>Y` dans la RAM cible et
  exécuter `fadd.d`, `fsub.d`, `fmul.d` ou `fdiv.d` selon sa valeur.
- **Non-but :** lexer général, précédence, parenthèses, variables arbitraires,
  diagnostics, formatage décimal et contrôle de flot.
- **Entrées :** R1 F/D, `BASIC_LANGUAGE.md`, source `payload-repl.rv` et ABI
  `RVMPAY01`.
- **Fichiers/modules :** `examples/minibasic-asm/payload-repl.rv`,
  `scripts/test-guest-runtime-asm-repl-mul.sh` et documentation BASIC.
- **Dépendances :** BASIC-LOAD-005O, encodages R2 des quatre instructions D et
  chargement target-side de la ligne.
- **Tests :** `X+Y` (`8.0`) et `X*Y` (`15.0`) sous QEMU, registres `f1/f2/f3`,
  breakpoint et dumps exacts.
- **Critères de sortie :** l’opérateur vient de la ligne en RAM ; il n’est pas
  choisi par le script hôte et le calcul est exécuté par le payload.
- **Cas limites :** `-` et `/` possèdent encore seulement le chemin de calcul
  brut ; division par zéro, syntaxe invalide et opérateur inconnu restent à
  raccorder aux diagnostics.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, extension D, QEMU et débogueur.
- **Parallélisable :** oui avec le lexer ; non avec une modification du layout
  de la ligne stockée.
- **Paquet de contexte :** BASIC-LOAD-005O, `payload-repl.rv`, tests `X+Y` et
  fixtures opérateurs précédentes.

#### BASIC-LOAD-005Q — Lexer borné des facteurs target-side — TERMINÉE

- **Priorité :** P0, première lecture d’opérandes depuis le corps BASIC.
- **But :** décoder `X`, `Y` et les chiffres `0`–`9` dans
  `PRINT <atome><op><atome>`, convertir les chiffres en binary64 et alimenter
  le dispatch des quatre opérateurs D.
- **Non-but :** espaces, signes, littéraux décimaux, parenthèses, précédence,
  noms longs, chaînes et diagnostics structurés.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 F/D, ABI `RVMPAY01` et layout du record
  target-side.
- **Fichiers/modules :** `examples/minibasic-asm/payload-repl.rv`,
  `scripts/test-guest-runtime-asm-repl-literal.sh` et documentation BASIC.
- **Dépendances :** BASIC-LOAD-005P, `fcvt.d.l`, table binary64 et source
  assembleur du moniteur.
- **Tests :** `PRINT 2+3` sous QEMU, `f1=2.0`, `f2=3.0`, `f3=5.0`, breakpoint
  et dump exact.
- **Critères de sortie :** les deux atomes sont lus dans la RAM cible et le
  calcul est exécuté par le payload, sans valeur calculée par l’hôte.
- **Cas limites :** caractère invalide, chiffre hors plage, opérande absent,
  espaces et littéraux fractionnaires restent à traiter.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, IEEE binary64, QEMU et débogueur.
- **Parallélisable :** oui avec le stockage des diagnostics ; non avec le
  déplacement du layout du corps stocké.
- **Paquet de contexte :** BASIC-LOAD-005O/P, `payload-repl.rv`, tests `X+Y`,
  `X*Y` et règles d’expression de `BASIC_LANGUAGE.md`.

#### BASIC-LOAD-005R — Longueur de record et littéraux décimaux target-side — TERMINÉE

- **Priorité :** P0, suppression de la longueur de corps codée en dur.
- **But :** stocker la longueur réelle du corps BASIC et évaluer la forme
  bornée `PRINT d.d<op>d.d` dans le payload assembleur.
- **Non-but :** exposant, signe, plusieurs chiffres, espaces, parenthèses,
  précédence générale, formatage décimal et lexer non positionnel.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 F/D, layout record et ABI `RVMPAY01`.
- **Fichiers/modules :** `payload-repl.rv`, quatre harnais de séance et
  documentation BASIC.
- **Dépendances :** BASIC-LOAD-005Q, champ longueur target-side, `fcvt.d.l` et
  opérations `fdiv.d`/`fadd.d`.
- **Tests :** `10 PRINT 2.5+3.5`, `LIST` avec longueur 13, `f1=2.5`, `f2=3.5`,
  `f3=6.0`, breakpoint et dump exact sous QEMU.
- **Critères de sortie :** les fragments décimaux et le résultat sont produits
  par le payload ; le script ne calcule aucune valeur.
- **Cas limites :** signes, plus de deux chiffres, point absent/répété,
  espaces, sous-normaux et erreur syntaxique restent à intégrer.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, IEEE binary64, QEMU et UART.
- **Parallélisable :** oui avec le lexer abstrait ; non avec un changement du
  layout de record ou de la capacité de transport source.
- **Paquet de contexte :** BASIC-LOAD-005Q, `payload-repl.rv`, tests literal et
  operators, `BASIC_LANGUAGE.md` et règles de représentation binary64.

#### BASIC-LOAD-005S — Mode direct PRINT dans le payload assembleur — TERMINÉE

- **Priorité :** P0, premier chemin d’exécution directe intégré.
- **But :** reconnaître `PRINT ...` sans numéro, conserver la même
  représentation de record et réutiliser l’évaluation target-side.
- **Non-but :** `?`, chaînes, formatage décimal, expressions générales,
  diagnostics complets et plusieurs instructions directes.
- **Entrées :** `BASIC_LANGUAGE.md`, source `payload-repl.rv`, ABI UART et
  layout du record avec longueur.
- **Fichiers/modules :** payload assembleur, test QEMU direct et documentation
  BASIC.
- **Dépendances :** BASIC-LOAD-005R/Q, boucle de commande et évaluateur des
  facteurs bornés.
- **Tests :** `PRINT 2+3` sans numéro, breakpoint, `f1=2.0`, `f2=3.0`,
  `f3=5.0` et dump exact sous QEMU.
- **Critères de sortie :** la commande et le calcul sont consommés dans la
  cible ; aucun résultat n’est injecté par l’hôte.
- **Cas limites :** `?`, chaîne directe, ligne vide, erreur de syntaxe,
  affichage décimal et retour prompt restent à intégrer.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, UART target-side, QEMU et D.
- **Parallélisable :** oui avec l’affichage binary64 ; non avec un changement
  du format de record partagé.
- **Paquet de contexte :** BASIC-LOAD-005O/R, payload REPL, tests literal et
  règles de mode direct de `BASIC_LANGUAGE.md`.

#### BASIC-LOAD-005T — Alias direct `?` target-side — TERMINÉE

- **Priorité :** P0, compatibilité d’usage immédiat inspirée de BASIC/Turbo
  BASIC XL.
- **But :** reconnaître `?expression`, construire le même record `PRINT` dans
  la mémoire cible et réutiliser exactement l’évaluateur direct.
- **Non-but :** chaînes, formatage décimal, espaces généraux, plusieurs
  instructions et parser complet.
- **Entrées :** `BASIC_LANGUAGE.md`, source `payload-repl.rv`, conventions
  historiques de `?` et ABI UART.
- **Fichiers/modules :** payload assembleur, test QEMU question et docs BASIC.
- **Dépendances :** BASIC-LOAD-005S, longueur de record et évaluateur borné.
- **Tests :** `?2+3` target-side, breakpoint, `f1=2.0`, `f2=3.0`, `f3=5.0`
  et dump exact.
- **Critères de sortie :** l’alias et la normalisation sont exécutés par le
  payload, sans transformation de l’hôte.
- **Cas limites :** `?` vide, chaîne, espaces, erreur et sortie décimale restent
  à intégrer.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, UART target-side et QEMU.
- **Parallélisable :** oui avec le formateur de sortie ; non avec le layout
  partagé du record.
- **Paquet de contexte :** BASIC-LOAD-005S/R, payload REPL, tests direct/literal
  et règles d’alias de `BASIC_LANGUAGE.md`.

#### BASIC-LOAD-005U — Affectation numérique target-side — TERMINÉE

- **Priorité :** P0, première mutation observable de l’état BASIC assembleur.
- **But :** reconnaître `X=7` ou `Y=7`, convertir le chiffre par `fcvt.d.l`,
  l’écrire dans la table binary64 cible et le relire lors d’un `PRINT`.
- **Non-but :** expressions à droite, affectations numérotées, nombres signés
  ou fractionnaires, variables longues, chaînes et diagnostics généraux.
- **Entrées :** `BASIC_LANGUAGE.md`, layout de table A..Z, R1 F/D et ABI cible.
- **Fichiers/modules :** payload assembleur, test QEMU assignment et docs BASIC.
- **Dépendances :** BASIC-LOAD-005S/T/R, table variables et capacité source
  assembleur 512 lignes.
- **Tests :** `X=7`, `PRINT X+3`, `f1=7.0`, `f2=3.0`, `f3=10.0`, dumps exacts
  des variables et du résultat sous QEMU.
- **Critères de sortie :** la mutation, la lecture et le calcul sont exécutés
  dans le payload ; l’hôte ne fournit aucune valeur.
- **Cas limites :** `Y`, affectation absente, signe, fraction, variable
  inconnue, expression RHS et retour d’erreur restent à intégrer.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, table binary64, QEMU et debugger.
- **Parallélisable :** oui avec le lexer RHS ; non avec une modification du
  layout de variables.
- **Paquet de contexte :** BASIC-LOAD-005Q/S/T, payload REPL, test assignment
  et règles d’affectation de `BASIC_LANGUAGE.md`.

#### BASIC-LOAD-005V — Scanner target-side multi-chiffres — TERMINÉE

- **Priorité :** P0, généralisation minimale de la lecture des atomes.
- **But :** parcourir les opérandes avec un pointeur cible, accumuler plusieurs
  chiffres avant/après le point et conserver l’opérateur pendant l’appel du
  parseur du second atome.
- **Non-but :** espaces, signes unaires, parenthèses, précédence générale,
  exposants, chaînes et diagnostics complets.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 D, layout du record et ABI `RVMPAY01`.
- **Fichiers/modules :** `examples/minibasic-asm/payload-repl.rv`,
  `scripts/test-guest-runtime-asm-repl-multidigit.sh` et documentation BASIC.
- **Dépendances :** BASIC-LOAD-005R/U, capacité source 512 lignes et routines
  `fcvt.d.l`/`fdiv.d` target-side.
- **Tests :** QEMU réel avec `PRINT 12.5+3.5`, breakpoint, `f1=12.5`,
  `f2=3.5`, `f3=16.0` et dump binaire exact.
- **Critères de sortie :** aucune position d’opérande ni aucun résultat n’est
  calculé par le harnais hôte ; le scanner et les opérations D s’exécutent dans
  le payload assembleur.
- **Cas limites :** texte vide, caractères invalides, espaces, signes,
  parenthèses et débordement des accumulateurs restent explicitement refusés
  ou à traiter dans la prochaine tranche.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, binary64, QEMU et débogueur.
- **Parallélisable :** oui avec la conception des diagnostics ; non avec un
  changement du layout de record.
- **Paquet de contexte :** BASIC-LOAD-005R/U, payload REPL, tests décimaux,
  `BASIC_LANGUAGE.md` et `GUEST_PAYLOAD_ABI.md`.

#### BASIC-LOAD-005W — Espaces autour des expressions — TERMINÉE

- **Priorité :** P0, compatibilité syntaxique minimale du mode direct et des
  lignes stockées.
- **But :** ignorer les espaces ASCII autour des atomes et de l’opérateur sans
  déplacer le record ni déléguer l’analyse à l’hôte.
- **Non-but :** parenthèses, signes unaires, précédence, chaînes ou lexer
  complet.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 D et layout target-side.
- **Fichiers/modules :** `payload-repl.rv`, test multi-chiffres et docs BASIC.
- **Dépendances :** BASIC-LOAD-005V et capacité source 512 lignes.
- **Tests :** `PRINT 12.5 + 3.5` sous QEMU, breakpoint, registres `f1/f2/f3`
  et dump exact `16.0`.
- **Critères de sortie :** les espaces sont consommés par le scanner cible et
  les anciennes formes sans espaces restent vertes.
- **Cas limites :** tabulations, espaces dans les chaînes, espaces multiples,
  parenthèses et signes restent à spécifier/implémenter.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur RV64, QEMU et tests E2E.
- **Parallélisable :** oui avec les diagnostics ; non avec un changement du
  layout de record.
- **Paquet de contexte :** BASIC-LOAD-005V, payload REPL et test multidigit.

#### BASIC-LOAD-005X — Signes unaires et parenthèses simples — TERMINÉE

- **Priorité :** P0, couverture des formes syntaxiques élémentaires de BASIC.
- **But :** accepter `+`/`-` devant un atome et une paire de parenthèses autour
  d’un atome, avec négation binary64 réellement exécutée dans le payload.
- **Non-but :** AST, parenthèses imbriquées, plusieurs opérateurs, précédence,
  exposants et diagnostics complets.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 D, encodages R2 des transferts F/X et
  layout du record target-side.
- **Fichiers/modules :** `payload-repl.rv`, test QEMU unary/parenthesis et docs.
- **Dépendances :** BASIC-LOAD-005W/V, `fmv.x.d`, `fmv.d.x` et opérations D.
- **Tests :** `PRINT (-2.5) + (+3.5)`, breakpoint, `f1=-2.5`, `f2=3.5`,
  `f3=1.0` et dump exact en RAM sous QEMU.
- **Critères de sortie :** le signe est appliqué dans le guest ; aucune
  valeur négative ni aucun résultat n’est injecté par le harnais.
- **Cas limites :** signe isolé, parenthèse vide/non fermée, double signe,
  imbrication, NaN et débordement syntaxique restent à diagnostiquer.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, transferts de bits FP, QEMU et
  débogueur.
- **Parallélisable :** oui avec les diagnostics ; non avec la refonte du parser
  vers une pile d’opérateurs.
- **Paquet de contexte :** BASIC-LOAD-005V/W, payload REPL, test multidigit,
  règles D et ABI cible.

#### BASIC-LOAD-005Y — Précédence target-side produit/somme — TERMINÉE

- **Priorité :** P0, passage du dispatch binaire à une expression utile.
- **But :** évaluer les produits avant les sommes dans le payload assembleur,
  tout en conservant les opérandes visibles et le résultat en `f3`.
- **Non-but :** AST général, comparaisons, fonctions, opérateurs logiques et
  diagnostics structurés.
- **Entrées :** `BASIC_LANGUAGE.md`, R1 D et contrat de registre du payload.
- **Fichiers/modules :** `payload-repl.rv`, test QEMU de précédence et docs.
- **Dépendances :** BASIC-LOAD-005X/W, `fadd.d`, `fsub.d`, `fmul.d`, `fdiv.d`.
- **Tests :** `PRINT 2+3*4` sous QEMU, breakpoint, `f1=2.0`, `f2=4.0`,
  `f3=14.0` et dump exact de `14.0` en RAM.
- **Critères de sortie :** le produit est exécuté avant la somme dans la cible,
  sans calcul ou résultat de référence fourni par le harnais.
- **Cas limites :** expression vide, opérateur manquant, chaîne d’opérateurs,
  division par zéro et parenthèses imbriquées restent à traiter.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, registres FP, QEMU et debugger.
- **Parallélisable :** oui avec les diagnostics ; non avec un changement de
  convention des registres ou du layout du record.
- **Paquet de contexte :** BASIC-LOAD-005X, payload REPL, tests opérateurs et
  `BASIC_LANGUAGE.md`.

#### BASIC-LOAD-005Z — Affichage décimal target-side borné — TERMINÉE

- **Priorité :** P0, rendre le payload observable comme un programme BASIC.
- **But :** convertir le résultat binary64 dans la cible en forme fixe à six
  décimales, construire un buffer ASCII et l’émettre par `ecall 4` avant le
  breakpoint d’inspection.
- **Non-but :** shortest-round-trip, NaN/infini complet, exponentielle,
  localisation ou formatage des chaînes.
- **Entrées :** `BASIC_LANGUAGE.md`, `GUEST_PAYLOAD_ABI.md`, R1 D et contrat
  `ecall 4`.
- **Fichiers/modules :** `payload-repl.rv`, tests QEMU d’expressions et docs.
- **Dépendances :** BASIC-LOAD-005Y, `fcvt.l.d`, `fcvt.d.l`, opérations D et
  service console target-side.
- **Tests :** sorties `8.000000`, `6.000000`, `1.000000` et `14.000000`, plus
  motifs `f3`/RAM et breakpoint conservés.
- **Critères de sortie :** le texte est produit par le buffer et l’ecall du
  payload ; aucune transcription ou conversion hôte ne participe à l’exécution.
- **Cas limites :** zéro négatif, NaN, infinis, overflow de conversion et
  valeur fractionnaire à sixième décimale restent à couvrir.
- **Taille :** 5 points / 2,5 journées-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, conversions FP, ABI console et QEMU.
- **Parallélisable :** oui avec les tests IEEE ; non avec un changement du
  buffer d’expression ou du protocole d’ecall.
- **Paquet de contexte :** BASIC-LOAD-005Y, payload REPL, `GUEST_PAYLOAD_ABI.md`
  et tests expression QEMU.

#### BASIC-LOAD-005AA — Non-régression signe/fraction du formateur — TERMINÉE

- **Priorité :** P0, verrouillage du chemin de sortie avant le contrôle de flot.
- **But :** vérifier une valeur négative avec fraction non nulle et restaurer
  le motif original de `f3` après l’émission console.
- **Non-but :** ajouter les formats IEEE spéciaux ou modifier la politique de
  précision fixe.
- **Entrées :** BASIC-LOAD-005Z, R1 D et ABI `ecall 4`.
- **Fichiers/modules :** payload assembleur, test QEMU de formatage négatif et
  documentation BASIC.
- **Dépendances :** BASIC-LOAD-005Z et `fcvt.l.d` en arrondi vers zéro.
- **Tests :** `PRINT -2.25+0`, texte `-2.250000`, `f3=0xc002...` et dump exact.
- **Critères de sortie :** la sortie est correcte et le debugger observe encore
  le résultat signé original, sans calcul hôte.
- **Cas limites :** ±0, NaN, infinis, arrondi du sixième chiffre et overflow
  restent à couvrir.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur, IEEE 754, QEMU et débogueur.
- **Parallélisable :** oui avec les cas IEEE ; non avec une refonte du buffer.
- **Paquet de contexte :** BASIC-LOAD-005Z, test négatif et ABI console.

#### BASIC-LOAD-005AB — Deux slots de lignes et LIST trié — TERMINÉE

- **Priorité :** P0, fondation du programme multi-lignes target-side.
- **But :** stocker les lignes `10` et `20` dans des enregistrements séparés,
  conserver leurs longueurs/occupations en RAM et les restituer dans l’ordre.
- **Non-but :** exécution séquentielle, `GOTO`, remplacement général, lignes
  arbitraires et suppression par numéro.
- **Entrées :** `BASIC_LANGUAGE.md`, layout du payload, ABI mémoire et règle
  d’insertion hors ordre.
- **Fichiers/modules :** `payload-repl.rv`, test QEMU two-lines et docs.
- **Dépendances :** BASIC-LOAD-005AA, capacité source et service `ecall 4`.
- **Tests :** saisie `20` puis `10`, `LIST` trié, dump du second slot et `RUN`
  réel de la ligne 10.
- **Critères de sortie :** les deux corps sont écrits et lus depuis la RAM
  cible ; l’ordre n’est pas imposé par le script hôte.
- **Cas limites :** remplacement, suppression, numéro absent, slot plein et
  exécution de la seconde ligne restent à intégrer.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** layout mémoire RV64, assembleur, UART et QEMU.
- **Parallélisable :** oui avec le design des index de lignes ; non avec une
  modification du format de record.
- **Paquet de contexte :** BASIC-LOAD-005AA, payload REPL, test two-lines et
  `BASIC_LANGUAGE.md`.

#### BASIC-LOAD-005AC — RUN séquentiel de deux slots — TERMINÉE

- **Priorité :** P0, première exécution multi-lignes entièrement target-side.
- **But :** sélectionner le slot 10, exécuter son expression, puis sélectionner
  le slot 20 et s’arrêter après son résultat, avec un breakpoint final unique.
- **Non-but :** `GOTO`, `IF`, `FOR/NEXT`, lignes arbitraires et reprise après
  breakpoint intermédiaire.
- **Entrées :** `BASIC_LANGUAGE.md`, layout des slots et contrat debugger.
- **Fichiers/modules :** `payload-repl.rv`, test two-lines, capacité assembleur
  et documentation BASIC.
- **Dépendances :** BASIC-LOAD-005AB, 768 lignes source, formatter target-side.
- **Tests :** saisie hors ordre, `LIST`, `RUN`, sorties `9.000000` et
  `5.000000`, registres finaux et breakpoint après la ligne 20.
- **Critères de sortie :** les deux évaluations et la sélection de slot sont
  exécutées dans la cible ; le harnais ne simule aucune étape.
- **Cas limites :** slot 20 seul, ligne vide, erreur au premier slot, pause
  intermédiaire et boucles restent à traiter.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, contrôle d’exécution et QEMU.
- **Parallélisable :** oui avec les comparaisons ; non avec le format de slot.
- **Paquet de contexte :** BASIC-LOAD-005AB, payload REPL et test two-lines.

#### BASIC-LOAD-005AD — Dispatcher GOTO entre slots — TERMINÉE

- **Priorité :** P0, premier saut de contrôle de flot target-side.
- **But :** reconnaître `GOTO 10`/`GOTO 20` dans le record courant et modifier
  l’état d’exécution vers le slot cible avant de reprendre `RUN`.
- **Non-but :** `IF`, expressions de comparaison, lignes arbitraires, boucle
  infinie interrompable et diagnostics complets.
- **Entrées :** `BASIC_LANGUAGE.md`, layout des records et état `RUN` cible.
- **Fichiers/modules :** `payload-repl.rv`, test QEMU GOTO et documentation.
- **Dépendances :** BASIC-LOAD-005AC et slots 10/20.
- **Tests :** `20 PRINT 2+3`, `10 GOTO 20`, `RUN`, résultat `5.000000`, état
  `f3` et breakpoint après la cible.
- **Critères de sortie :** le saut et la sélection de slot sont exécutés par le
  payload ; aucun ordre d’exécution n’est imposé par le harnais.
- **Cas limites :** cible absente, ligne 10, saut arrière, boucle et argument
  non numérique restent à traiter.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.
- **Compétences/outils :** assembleur RV64, contrôle de flot, QEMU et debugger.
- **Parallélisable :** oui avec les comparaisons ; non avec la représentation de
  l’état `RUN`.
- **Paquet de contexte :** BASIC-LOAD-005AC, payload REPL et test GOTO.

#### BASIC-LOAD-005AE — Dispatcher END target-side — TERMINÉE

- **Priorité :** P0, arrêt explicite du programme utilisateur.
- **But :** reconnaître `END`, produire le message cible et arrêter après un
  point d’observation contrôlé sans le confondre avec `NEW`.
- **Non-but :** retour prompt autonome, propagation de statut, `IF` ou reprise
  après breakpoint.
- **Entrées :** `BASIC_LANGUAGE.md`, ABI console et contrat debugger.
- **Fichiers/modules :** `payload-repl.rv`, test QEMU END et docs BASIC.
- **Dépendances :** BASIC-LOAD-005AD et service `ecall 4`.
- **Tests :** `10 END`, `RUN`, sortie `END` et breakpoint réel.
- **Critères de sortie :** l’arrêt est provoqué par l’instruction target-side
  et non par une sortie préenregistrée du test.
- **Cas limites :** END dans le second slot, END après GOTO, statut de sortie
  et retour à l’invite restent à intégrer.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** assembleur, ABI guest et QEMU.
- **Parallélisable :** oui avec les comparaisons ; non avec le protocole d’arrêt.
- **Paquet de contexte :** BASIC-LOAD-005AD, test END et `GUEST_PAYLOAD_ABI.md`.

#### BASIC-LOAD-005AF — Comparaisons FP pour IF — TERMINÉE

- **Priorité :** P0, oracle ISA préalable à `IF ... THEN`.
- **But :** accepter et exécuter `feq.d`, `flt.d` et `fle.d` dans l’assembleur
  guest, avec destination entière et résultats 0/1 target-side.
- **Non-but :** syntaxe BASIC `IF`, chaînes, NaN policy spécifique au langage
  et opérateurs logiques.
- **Entrées :** R1 chapitre D, R2 généré, `BASIC_LANGUAGE.md` et encodeur ISA.
- **Fichiers/modules :** parseur source du moniteur, fixture fcmp et test QEMU.
- **Dépendances :** tables R2 générées et capacité assembleur existante.
- **Tests :** comparaison 2.0/3.0 donnant `feq=0`, `flt=1`, `fle=1` en registres
  et en mémoire, sous exécution QEMU réelle.
- **Critères de sortie :** `rd` est traité comme registre entier et les bits
  encodés correspondent à la table générée ; aucun oracle hôte n’évalue le FP.
- **Cas limites :** NaN, ±0, égalité et `fcsr` de comparaison restent à ajouter
  au corpus BASIC.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** RISC-V D, génération R2, assembleur et QEMU.
- **Parallélisable :** oui avec le lexer IF ; non avec une modification des
  conventions de booléens.
- **Paquet de contexte :** R1 D, `riscv-opcodes`, fixture fcmp et ABI guest.

#### BASIC-LOAD-005AG — IF THEN target-side borné — TERMINÉE

- **Priorité :** P0, première décision conditionnelle BASIC.
- **But :** évaluer une comparaison entre deux atomes et transférer vers la
  ligne 10 ou 20 lorsque `THEN` est vrai, avec poursuite séquentielle lorsqu’il
  est faux.
- **Non-but :** expressions booléennes composées, `AND/OR`, parenthèses de
  condition, lignes arbitraires et diagnostics complets.
- **Entrées :** `BASIC_LANGUAGE.md`, comparaisons RISC-V D et état des slots.
- **Fichiers/modules :** `payload-repl.rv`, test QEMU IF et docs BASIC.
- **Dépendances :** BASIC-LOAD-005AF, BASIC-LOAD-005AD et slots 10/20.
- **Tests :** `10 IF 1<2 THEN 20`, `20 PRINT 7+8`, résultat `15.000000`,
  breakpoint final et exécution sans orchestration hôte.
- **Critères de sortie :** la comparaison et la branche sont calculées dans la
  cible ; le harnais ne choisit pas le chemin.
- **Cas limites :** faux sans ligne suivante, cible absente, NaN, ±0, chaîne
  de comparaison invalide et conditions composées restent à couvrir.
- **Taille :** 5 points / 2,5 journées-agent, incertitude moyenne.
- **Compétences/outils :** assembleur, IEEE 754, contrôle de flot et QEMU.
- **Parallélisable :** oui avec diagnostics et `INPUT` ; non avec la convention
  de booléens/slots.
- **Paquet de contexte :** BASIC-LOAD-005AF, payload IF et test QEMU.

#### BASIC-LOAD-005AH — Non-régression IF faux — TERMINÉE

- **Priorité :** P0, preuve du chemin conditionnel négatif.
- **But :** vérifier qu’une condition fausse n’exécute pas sa cible absente et
  s’arrête sans résultat parasite.
- **Non-but :** poursuite complexe, `ELSE`, boucles et diagnostics complets.
- **Entrées :** BASIC-LOAD-005AG, état des slots et convention booléenne.
- **Fichiers/modules :** test QEMU IF-false et documentation BASIC.
- **Dépendances :** BASIC-LOAD-005AG et dispatcher `run_stop`.
- **Tests :** `10 IF 2<1 THEN 20`, absence de `15.000000`, breakpoint réel.
- **Critères de sortie :** le chemin faux est décidé dans la cible et aucune
  sortie attendue n’est injectée par le harnais.
- **Cas limites :** false avec ligne suivante, NaN, comparateur invalide et
  `ELSE` restent à intégrer.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** QEMU, assembleur et tests de contrôle de flot.
- **Parallélisable :** oui avec `INPUT` ; non avec une refonte du dispatcher.
- **Paquet de contexte :** BASIC-LOAD-005AG, tests IF vrai/faux et payload.

#### BASIC-LOAD-005AI — INPUT numérique target-side — TERMINÉE

- **Priorité :** P0, service d’entrée nécessaire aux programmes interactifs.
- **But :** lire une ligne UART après `INPUT X` ou `INPUT Y`, convertir sa forme
  décimale dans la cible et écrire la variable binary64 avant de continuer.
- **Non-but :** chaînes, validation complète des erreurs, édition de ligne,
  séparateurs multiples et `INPUT` de plusieurs variables.
- **Entrées :** `BASIC_LANGUAGE.md`, ABI `ecall 2`, parseur binary64 et table
  des variables.
- **Fichiers/modules :** `payload-repl.rv`, test QEMU INPUT et documentation.
- **Dépendances :** BASIC-LOAD-005AG, lecture UART et `parse_atom`.
- **Tests :** `20 PRINT X*X`, `10 INPUT X`, entrée `3.5`, invite, sortie
  `12.250000`, registres `f1/f2/f3` et breakpoint final.
- **Critères de sortie :** l’octet lu, la conversion et la mutation de variable
  sont target-side ; l’hôte fournit seulement les octets UART.
- **Cas limites :** entrée vide, syntaxe invalide, overflow, Ctrl-C et INPUT Y
  restent à compléter par diagnostics et tests dédiés.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.
- **Compétences/outils :** UART guest, parseur FP, assembleur et QEMU.
- **Parallélisable :** oui avec TRACE ; non avec une refonte du buffer d’entrée.
- **Paquet de contexte :** BASIC-LOAD-005AH, ABI UART, payload IF et test INPUT.

### BASIC-LOAD-004 — Porter le magasin de lignes et le contrôle de flot

- **Priorité :** P0.
- **But :** porter lignes numérotées, LIST/RUN, variables, FOR/NEXT, IF/GOTO,
  INPUT, TRACE et diagnostics dans le payload.
- **Non-but :** augmenter encore la capacité source ; cette tâche utilise une
  capacité courte explicitement bornée.
- **Dépendances :** BASIC-LOAD-003 ; bloque BASIC-LOAD-005.
- **Tests :** FLOATLOOP, erreur GOTO absent, boucle interrompue et INPUT réel.
- **Acceptation :** les programmes de démonstration produisent leurs résultats
  à l’exécution dans U-mode et atteignent les instructions D observées.
- **Taille :** 10 points / 5 journées-agent, incertitude très élevée.

### BASIC-STR-001 — Porter les chaînes dans le payload RV

- **Priorité :** P0 produit, après le noyau lexer/expression ; différé dans la
  tranche actuelle mais conservé comme exigence.
- **But :** fournir des chaînes variables et littérales, affectation,
  affichage, limites explicites et erreurs entièrement exécutés en cible.
- **Non-but :** compatibilité Atari implicite, chaînes calculées par l’hôte ou
  allocation non bornée.
- **Entrées :** `BASIC_LANGUAGE.md`, ABI `RVMPAY01`, notes TBXL, décision de
  représentation mémoire à figer avant implémentation.
- **Fichiers/modules probables :** `payload/string_store.rv`,
  `payload/lexer.rv`, `payload/print.rv`, documentation et tests QEMU.
- **Dépendances :** BASIC-LOAD-003/004 ; bloque les démonstrations utilisant
  des variables chaînes et une partie de Hammurabi enrichi.
- **Tests :** chaîne vide, longueur maximale, copie, affichage, caractère
  spécial, dépassement du pool, snapshot et absence de sortie préenregistrée.
- **Critères de sortie :** toutes les données et opérations vivent dans la
  mémoire cible ; l’hôte ne reçoit que les octets de sortie via ecall.
- **Cas limites :** pool plein, longueur nulle, index invalide et corruption
  de descripteur diagnostiqués sans écriture partielle.
- **Taille :** 8 points / 4 journées-agent, incertitude élevée.
- **Parallélisable :** partiellement avec BASIC-ARRAY-001 après gel du layout.

### BASIC-ARRAY-001 — Porter les tableaux numériques et de chaînes

- **Priorité :** P0 produit, après BASIC-STR-001 et le contrôle de flot.
- **But :** conserver les tableaux complets dans le langage cible, avec
  dimensions, indexation, stockage row-major documenté et diagnostics de
  bornes.
- **Non-but :** tableaux calculés par l’hôte, dimensions illimitées ou
  compatibilité binaire avec les tableaux Atari.
- **Entrées :** `BASIC_LANGUAGE.md`, notes TBXL sur les tableaux, ABI payload et
  budget mémoire cible.
- **Fichiers/modules probables :** `payload/array_store.rv`, parseur
  d’indexation, `DUMP`, snapshots et corpus QEMU.
- **Dépendances :** BASIC-STR-001, BASIC-LOAD-004 et décision de layout.
- **Tests :** tableaux 1D/2D, tableaux numériques et chaînes, index minimal et
  maximal, hors bornes, dimensions invalides, pool plein et restauration.
- **Critères de sortie :** accès, mutation et résultats sont observables dans
  la RAM cible et reproductibles sous QEMU.
- **Cas limites :** produit de dimensions overflow, index négatif, tableau
  absent et alias de descripteur.
- **Taille :** 10 points / 5 journées-agent, incertitude très élevée.
- **Parallélisable :** oui avec l’outillage de DUMP après gel du layout ; non
  avec une modification concurrente du lexer.

### BASIC-LOAD-005 — Remplacer le mode résident par le payload chargé

- **Priorité :** P0, intégration.
- **But :** ajouter `assemble-load`/équivalent, validation atomique et fallback
  résident explicitement sélectionnable.
- **Dépendances :** BASIC-LOAD-004, `run-at`, snapshots guest.
- **Tests :** séance tutoriel, Hammurabi, breakpoint/watch sur `fdiv.d`,
  snapshot/reprise et faute de chargement sans mutation.
- **Acceptation :** le tutoriel indique clairement le mode chargé et aucune
  sortie de démonstration n’est préenregistrée.
- **Taille :** 5 points / 2,5 journées-agent, incertitude élevée.

### BASIC-SOURCE-001 — Étendre le stockage du programme BASIC et ses preuves de capacité

- **Priorité :** P1, après validation de P0.
- **But :** augmenter la capacité du magasin de lignes BASIC au-delà de sa
  capacité V1 actuelle, sans modifier l’ISA, et mesurer explicitement la RAM
  statique et la pile consommées.
- **Non-but :** capacité dynamique ou absence de limites.
- **Entrées :** décision de capacité du programme BASIC, pile M-mode 64 Kio,
  snapshots/projets guest.
- **Dépendances :** P0 recommandé ; bloque BASIC-SOURCE-002/003.
- **Tests :** insertion/remplacement/suppression/tri de la capacité retenue, saturation,
  mémoire pleine, snapshot et mesure de pile sous QEMU.
- **Acceptation :** la capacité retenue passe sans trap imbriqué ; la ligne suivante est refusée
  avec diagnostic stable et sans mutation.
- **Taille :** 3 points / 1,5 journée-agent, incertitude moyenne.

### BASIC-LOAD-002A — Élargir le buffer source assembleur sans croissance de pile — TERMINÉ

- **Priorité :** P0, prérequis du runtime assembleur.
- **But :** permettre au parseur guest de recevoir 256 lignes de 128 caractères
  et 64 labels, afin de charger progressivement des fragments substantiels du
  runtime MiniBASIC.
- **Non-but :** annoncer que MiniBASIC est déjà réécrit en assembleur ; cette
  tâche ne modifie pas le chemin `basic` résident.
- **Entrées :** `GUEST_PAYLOAD_ABI.md`, `SPEC.md` §§9/11/17/18, contrainte de
  pile M-mode 64 Kio.
- **Fichiers/modules :** `crates/guest-monitor/src/main.rs`,
  `scripts/test-guest-source-capacity.sh`, `docs/TUTORIAL-GUEST.md`.
- **Étapes réalisées :** buffers source, longueurs, labels, mots et adresses
  déplacés en scratch statique ; metadata `RVMETA01` porté à 32 Kio ;
  validation atomique conservée.
- **Dépendances :** BASIC-LOAD-002 ; prépare BASIC-LOAD-003/004.
- **Tests :** 256 instructions assemblées sous QEMU ; 257 lignes refusées par
  `GUEST-ASM-001`; `cargo check` cible ; tests source existants.
- **Critères de sortie :** la pile ne contient plus les tableaux de travail
  dimensionnés par `MAX_SOURCE_LINES`; l’image compile et la limite est
  observable, stable et non mutante.
- **Cas limites et échecs :** saturation, adresse hors workspace, label en
  double, metadata au-delà de 32 Kio ; le débordement de ligne reste rejeté.
- **Taille :** 3 points / 1,5 journée-agent, incertitude faible.
- **Compétences/outils :** Rust `no_std`, aliasing statique sûr, QEMU UART,
  `nm`.
- **Parallélisable :** oui avec BASIC-LOAD-003A/B ; non avec une autre
  modification des buffers source guest.
- **Paquet de contexte minimal :** `main.rs`, `GUEST_PAYLOAD_ABI.md`,
  `test-guest-source-capacity.sh`, `TUTORIAL-GUEST.md`.

### UI-GUI-001 — Stabiliser le modèle d’interface graphique

- **Priorité :** P2, après P0/P1.
- **But :** définir un modèle de panneaux, événements, commandes et snapshots
  indépendant du toolkit graphique.
- **Non-but :** choisir ou livrer immédiatement une UI spécifique à une OS.
- **Entrées :** contrats `Monitor`, `BackendConsole`, vues mémoire/registres,
  keymap terminal.
- **Dépendances :** interfaces publiques host stabilisées ; bloque UI-GUI-002.
- **Tests :** scénario scripté sans toolkit, conservation d’adresse et absence
  de mutation hors commande.
- **Acceptation :** un backend graphique pourra consommer le modèle sans accès
  direct à la RAM cible.
- **Taille :** 4 points / 2 journées-agent, incertitude moyenne.

### DBG-HISTORY-001 — Historique arrière borné et déterministe

- **Priorité :** P2, parallélisable avec UI-GUI-001 après contrat d’événement.
- **But :** enregistrer les mutations nécessaires à reverse-step/reverse-run
  dans un quota explicite, avec restauration de registres, mémoire et CSR.
- **Non-but :** historique illimité, persistance automatique ou remplacement
  de l’undo transactionnel.
- **Dépendances :** modèle d’événements, snapshots et décision de quota.
- **Tests :** pas avant/arrière, limite atteinte, instruction FP, store, trap,
  déterminisme de replay et état inchangé après refus.
- **Acceptation :** reverse-step restaure exactement l’état précédent ou
  signale `history quota exhausted` sans corruption.
- **Taille :** 6 points / 3 journées-agent, incertitude élevée.

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

#### QUAL-001A — Smoke-fuzz déterministe sans dépendance externe — TERMINÉ

- **Jalon / exigences :** M9; REQ-PROD-002/004, ISA/ASM/DIS/CMD.
- **But :** fournir un premier filet de sécurité reproductible contre les
  paniques du parseur de commandes/expressions et du désassembleur.
- **Non-but :** oracle sémantique, couverture exhaustive, libFuzzer nightly ou
  réduction automatique de crashs.
- **Entrées/sources :** SPEC §23; contrats `command::parse_expression`,
  `command::parse` et `disassemble_word`.
- **Fichiers/modules :** tests `luna-monitor`/`luna-disassembler`,
  `tools/fuzz-smoke.sh`, `tests/fuzz/README.md`.
- **Étapes réalisées :** générateur LCG fixé; 20 000 commandes/expressions et
  100 000 mots d’instruction; script CI/local borné et rejouable.
- **Dépendances et tâches bloquées :** QUAL-001; corpus métier, libFuzzer,
  reducer et budgets nightly restent à faire.
- **Tests :** `bash tools/fuzz-smoke.sh`, `cargo test --workspace`.
- **Critères de sortie :** mêmes graines et mêmes volumes à chaque exécution;
  aucune panique et aucune boucle non bornée.
- **Cas limites et échecs :** expressions trop grandes, opérateurs invalides,
  opcodes illégaux et mots arbitraires sont acceptés ou rejetés sans panic.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** Rust tests, générateur déterministe, shell CI.
- **Parallélisable :** oui avec FORMAT-001; non avec une modification des
  API parser/disassembler.
- **Paquet de contexte minimal :** `tools/fuzz-smoke.sh`, `tests/fuzz/README.md`,
  tests `fuzz_smoke_*`, SPEC §23.

#### QUAL-001B — Corpus de seeds du moniteur — TERMINÉ

- **Jalon / exigences :** M9; REQ-PROD-002/004, CMD, ASM, DIS, DBG.
- **But :** versionner des entrées normales, limites et erronées qui exercent
  réellement la surface de commande du moniteur.
- **Non-but :** considérer un diagnostic attendu comme un échec, fournir un
  oracle sémantique ou prétendre remplacer libFuzzer.
- **Entrées/sources :** SPEC §§10/13/14/16/19/23; commandes publiques host.
- **Fichiers/modules :** `tests/fuzz/seeds/`, test `luna-monitor`, README fuzz.
- **Étapes réalisées :** ajouter seeds commandes et expressions; exécuter chaque
  commande dans une cible neuve; accepter succès ou diagnostic mais interdire
  panic et blocage; conserver les cas invalides pour la non-régression.
- **Dépendances et tâches bloquées :** QUAL-001A; réduction automatique et
  libFuzzer nightly restent à faire.
- **Tests :** `bash tools/fuzz-smoke.sh`, `fuzz_seed_corpus_exercises_real_monitor_commands`.
- **Critères de sortie :** chaque seed est rejouable depuis clean checkout et
  aucun seed ne mute une cible après une erreur non prévue par le contrat.
- **Cas limites et échecs :** chemin absent, mémoire non mappée, expression
  inconnue, division par zéro et instruction invalide restent diagnostiqués.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** tests Rust, corpus texte, diagnostics CLI.
- **Parallélisable :** oui avec FORMAT-001; non avec une modification de la
  grammaire publique sans mise à jour du corpus.
- **Paquet de contexte minimal :** `tests/fuzz/seeds/*`, test seed corpus,
  `tools/fuzz-smoke.sh`, SPEC §23.

#### QUAL-001C — Réducteur de contre-exemples texte — TERMINÉ

- **Jalon / exigences :** M9; REQ-PROD-002/004, OBS-001..006.
- **But :** réduire automatiquement un cas de commandes ou diagnostics en
  conservant un prédicat externe de reproduction.
- **Non-but :** réduire des binaires, comprendre la cause sémantique ou
  remplacer un oracle indépendant.
- **Entrées/sources :** SPEC §§19/21/23; corpus `tests/fuzz/seeds`.
- **Fichiers/modules :** `tools/reduce-fuzz-case.sh`, README fuzz.
- **Étapes réalisées :** vérifier d’abord que le checker reproduit le cas;
  supprimer gloutonnement chaque ligne conservant le succès; produire un
  fichier minimal et déterministe; nettoyer les temporaires même sur erreur.
- **Dépendances et tâches bloquées :** QUAL-001A/B; réduction binaire et
  intégration libFuzzer restent différées.
- **Tests :** seed avec checker `grep`, cas non reproductible refusé et sortie
  réduite rejouable.
- **Critères de sortie :** aucun retrait accepté si le checker échoue; le
  fichier produit est lisible et le checker y reste positif.
- **Cas limites et échecs :** entrée absente, arguments incomplets, checker
  négatif, seed vide et lignes dupliquées sont traités sans perte silencieuse.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** shell POSIX, réduction delta, tests reproductibles.
- **Parallélisable :** oui avec FORMAT-001; non avec une modification du
  contrat d’artefact fuzz.
- **Paquet de contexte minimal :** `tools/reduce-fuzz-case.sh`,
  `tests/fuzz/README.md`, SPEC §23.

#### QUAL-001D — Cibles libFuzzer autonomes — TERMINÉ

- **Jalon / exigences :** M9; REQ-PROD-002/004, ISA/ASM/DIS/CMD.
- **But :** fournir des cibles libFuzzer exploitant les API publiques du
  moniteur et du désassembleur sans rendre la dépendance obligatoire au build
  principal.
- **Non-but :** exécuter libFuzzer dans la CI de chaque commit, fournir un
  oracle sémantique ou modifier le workspace de production.
- **Entrées/sources :** SPEC §23; contrat public `Monitor::execute` et
  `disassemble_word`; `libfuzzer-sys` 0.4.7.
- **Fichiers/modules :** `fuzz/Cargo.toml`, `fuzz/fuzz_targets/`, README fuzz.
- **Étapes réalisées :** créer un workspace fuzz autonome; ajouter cibles
  `commands` et `disassembler`; fixer les budgets documentés timeout 5 s/
  session 60 s; conserver smoke-fuzz comme garde-fou sans dépendance.
- **Dépendances et tâches bloquées :** QUAL-001A/B/C; installation locale de
  cargo-fuzz et exécution nightly restent dépendantes de l’outillage externe.
- **Tests :** validation statique du manifest et smoke-fuzz principal; exécution
  libFuzzer à faire dans l’environnement outillé.
- **Critères de sortie :** le package est isolé du workspace; les deux cibles
  ont des entrées bornées et une commande d’exécution reproductible.
- **Cas limites et échecs :** octets UTF-8 invalides, entrée vide, fragment
  binaire incomplet et opcode illégal ne doivent pas provoquer de panic.
- **Taille :** 2 points / 1 journée-agent, incertitude moyenne.
- **Compétences/outils :** Cargo fuzz, libFuzzer, Rust FFI, réduction de cas.
- **Parallélisable :** oui avec QUAL-002; non avec une modification de l’API
  publique du moniteur sans recompilation des cibles.
- **Paquet de contexte minimal :** `fuzz/README.md`, `fuzz/Cargo.toml`,
  `fuzz/fuzz_targets/*`, SPEC §23.

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

#### QUAL-002A — Benchmark smoke assemble/step/snapshot — TERMINÉ

- **Jalon / exigences :** M9; NFR-001..006.
- **But :** mesurer un cycle représentatif en build release avec un protocole
  stable et sans seuil hôte prématuré.
- **Non-but :** annoncer une performance garantie, comparer deux machines sans
  rapport de contexte ou optimiser avant mesure.
- **Entrées/sources :** SPEC §20; contrat public `Monitor` et snapshot v4.
- **Fichiers/modules :** `crates/monitor/examples/bench_smoke.rs`,
  `tools/bench-smoke.sh`.
- **Étapes réalisées :** exécuter 1 000 cycles `assemble addi → step → snapshot`
  sur 64 KiB; afficher durée totale, moyenne par cycle et taille snapshot;
  exiger les mêmes opérations valides à chaque itération.
- **Dépendances et tâches bloquées :** QUAL-002; matrice x86_64/ARM64,
  p95 interactif, mémoire et seuils release restent à établir.
- **Tests :** `bash tools/bench-smoke.sh`, build release propre.
- **Critères de sortie :** le benchmark termine sans erreur et expose des
  métriques identifiées; aucune mesure n’altère un artefact du dépôt.
- **Cas limites et échecs :** assemblage, step ou snapshot invalide provoque un
  échec explicite; la durée zéro reste affichée sans division par zéro.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** Rust `Instant`, Cargo release, analyse de mesures.
- **Parallélisable :** oui avec QUAL-001; non avec une modification du contrat
  snapshot ou du cycle de bootstrap.
- **Paquet de contexte minimal :** example `bench_smoke`,
  `tools/bench-smoke.sh`, SPEC §20.

#### QUAL-002B — Échantillons p50/p95 et identification hôte — TERMINÉ

- **Jalon / exigences :** M9; NFR-001..006.
- **But :** fournir des mesures comparables entre environnements sans confondre
  une valeur locale avec un seuil normatif.
- **Non-but :** imposer un p95 release avant la matrice multi-plateforme,
  corriger les variations de charge hôte ou publier une performance garantie.
- **Entrées/sources :** SPEC §20; benchmark smoke QUAL-002A.
- **Fichiers/modules :** `crates/monitor/examples/bench_smoke.rs`,
  `tools/bench-smoke.sh`, docs de tests.
- **Étapes réalisées :** 10 échantillons de 100 cycles; tri déterministe des
  mesures; affichage p50/p95, nombre total d’itérations, taille snapshot, OS et
  architecture Rust.
- **Dépendances et tâches bloquées :** QUAL-002A; collecte ARM64/x86_64,
  mémoire RSS, latence interactive et seuils release restent à faire.
- **Tests :** benchmark release local, sortie contenant `os`, `arch`, `p50_ns`
  et `p95_ns`; workspace complète.
- **Critères de sortie :** chaque mesure est contextualisée par son hôte et le
  benchmark ne dépend d’aucune horloge ou donnée persistante du dépôt.
- **Cas limites et échecs :** échantillon vide, snapshot invalide ou division
  par zéro sont impossibles ou provoquent un échec explicite.
- **Taille :** 2 points / 1 journée-agent, incertitude faible.
- **Compétences/outils :** benchmarking, statistiques p50/p95, Rust.
- **Parallélisable :** oui avec QUAL-001; non avec une modification concurrente
  du protocole de sortie benchmark.
- **Paquet de contexte minimal :** example `bench_smoke`, SPEC §20,
  `tools/bench-smoke.sh`.

#### QUAL-002C — Smoke accessibilité sortie pipe — TERMINÉ

- **Jalon / exigences :** M9; NFR-008..010, REQ-PROD-005.
- **But :** garantir qu’un script monitor produit une sortie lisible sans
  dépendre de couleur ANSI ou d’un terminal interactif.
- **Non-but :** tester toutes les tailles d’écran, contrastes matériels,
  lecteurs d’écran ou une palette graphique.
- **Entrées/sources :** SPEC §§17/20; `examples/internal-first-step.rv`;
  tests de raccourcis `luna-app`.
- **Fichiers/modules :** `tools/accessibility-smoke.sh`, docs de tests.
- **Étapes réalisées :** exécuter le chemin pipe host; refuser toute séquence
  ANSI; vérifier la présence du résultat `x1`; conserver les raccourcis
  F5/F10/F11/Ctrl+1/2/3 dans les tests unitaires.
- **Dépendances et tâches bloquées :** QUAL-002; matrice de tailles, contraste
  et TTY réel restent à compléter si l’interface terminale est étendue.
- **Tests :** `bash tools/accessibility-smoke.sh`, `cargo test -p luna-app`.
- **Critères de sortie :** sortie pipe déterministe, sans escape graphique,
  avec résultat observable du premier scénario fonctionnel.
- **Cas limites et échecs :** script absent, commande en erreur ou séquence
  ANSI inattendue provoquent un échec explicite.
- **Taille :** 1 point / 0,5 journée-agent, incertitude faible.
- **Compétences/outils :** shell, TTY/pipes, accessibilité terminale.
- **Parallélisable :** oui avec QUAL-001; non avec une modification de sortie
  publique de `luna-app`.
- **Paquet de contexte minimal :** `tools/accessibility-smoke.sh`,
  `examples/internal-first-step.rv`, SPEC §§17/20.

#### QUAL-002D — Matrice CI Linux/macOS/Windows — TERMINÉ

- **Jalon / exigences :** M9; NFR-006..010, REQ-PROD-002/005.
- **But :** exécuter formatage, workspace et validation du package fuzz sur
  trois familles d’hôte sans dépendre d’un benchmark identique.
- **Non-but :** garantir les mêmes performances, lancer le transport série,
  installer libFuzzer nightly ou remplacer les oracles ISA.
- **Entrées/sources :** SPEC §20/23; `rust-toolchain.toml`; scripts QUAL-001
  et QUAL-002C.
- **Fichiers/modules :** `.github/workflows/quality.yml`.
- **Étapes réalisées :** matrice GitHub Actions Linux/macOS/Windows; ajout
  rustfmt explicite; `cargo fmt`, `cargo test --workspace`, metadata fuzz;
  smoke scripts Unix conditionnels; permissions de contenu en lecture seule.
- **Dépendances et tâches bloquées :** QUAL-001, QUAL-002A/C; runners arm64,
  comparaison de performance et TTY réel restent à compléter.
- **Tests :** validation locale du YAML et mêmes commandes sur Linux; preuve
  distante obtenue lorsque GitHub Actions exécute le workflow.
- **Critères de sortie :** chaque runner produit un résultat Cargo/fmt lisible;
  Windows n’est pas artificiellement bloqué par les scripts Bash Unix.
- **Cas limites et échecs :** composant rustfmt absent, test workspace échoué,
  manifest fuzz invalide ou runner Unix sans shell attendu → job rouge.
- **Taille :** 2 points / 1 journée-agent, incertitude moyenne.
- **Compétences/outils :** GitHub Actions, Rust toolchains, CI matrix.
- **Parallélisable :** oui avec QUAL-001/QUAL-002; non avec une modification
  concurrente de la politique toolchain.
- **Paquet de contexte minimal :** `.github/workflows/quality.yml`,
  `rust-toolchain.toml`, `tools/*-smoke.sh`, SPEC §20.

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

### REL-001A — Audit local reproductible

- **Statut :** TERMINÉ.
- **Jalon / exigences :** M9; REQ-QUAL-001, REQ-QUAL-002, REQ-REL-001.
- **Résultat :** `tools/release-audit.sh` enchaîne les contrôles locaux de
  formatage, tests workspace, manifest fuzz, intégrité/génération R2, fuzz
  smoke, accessibilité et benchmark, avec rapport texte stable optionnel.
- **Limite explicite :** GNU/LLVM sont séparés derrière `--strict-oracles`;
  Sail/Spike et la preuve de release complète restent des contrôles de M9.
- **Critère de sortie :** l’audit local passe sur un checkout propre; les
  contrôles externes non disponibles ne sont jamais présentés comme passés.

### REL-001B — Dossier provenance, SBOM et licences

- **Statut :** TERMINÉ.
- **Jalon / exigences :** M9; REQ-REL-001, REQ-REL-002, REQ-OBS-006.
- **But :** livrer un dossier déterministe de release avec commit, hash des
  entrées, provenance R2, SBOM Cargo et statut des licences.
- **Non-but :** redistribuer les normes externes ou ajouter un format de
  migration; aucun document externe non vérifié n’est vendored.
- **Fichiers/modules :** `docs/release/`, `tools/generate-release-dossier.sh`,
  `tools/release-audit.sh`.
- **Tests :** génération puis `--check`, hash R2, metadata Cargo et audit
  local complet.
- **Critère de sortie :** artefacts générés identiques au checkout et toute
  licence externe non vérifiée explicitement signalée.
- **Limite :** signature cryptographique, archive binaire et preuve Sail/Spike
  restent des étapes M9 distinctes.

### REL-001C — Audit de politique de release et waivers

- **Statut :** TERMINÉ.
- **Jalon / exigences :** M9; REQ-REL-001, REQ-REL-003, REQ-SEC-004.
- **But :** rendre visibles les limites de publication et refuser les dossiers
  incohérents, incomplets, contenant des chemins hôte ou des questions
  bloquantes non classées.
- **Résultat :** `tools/check-release-policy.sh` contrôle le dossier, la
  politique d’archivage externe, la structure SBOM, les chemins, les waivers
  et les questions ouvertes; `docs/release/WAIVERS.md` porte les blocages
  résiduels.
- **Non-but :** approuver une release publique ou fabriquer les preuves
  externes manquantes.
- **Critère de sortie :** contrôle de politique PASS et chaque exception
  explicite, localisée et actionnable.

### REL-001D — Campagne E2E guest/QEMU unifiée

- **Statut :** TERMINÉ pour la campagne disponible; couverture SPEC complète
  encore ouverte.
- **Jalon / exigences :** M9; REQ-REL-004, REQ-DBG-003, REQ-PROD-005.
- **But :** orchestrer les quatorze scripts E2E existants et rendre leur résultat
  global lisible, reproductible et bloquant en mode `--strict`.
- **Non-but :** inventer les scénarios SPEC encore absents ou remplacer les
  tests unitaires par QEMU.
- **Fichiers/modules :** `tools/e2e-release-smoke.sh`, `docs/TESTS.md`,
  `docs/RELEASE_CHECKLIST.md`.
- **Critère de sortie :** chaque script est PASS, ou explicitement SKIP avec
  dépendance manquante; le mode strict échoue sur tout SKIP ou FAIL.
- **Résultat :** 14/14 scripts PASS sur QEMU 11.0.2, sans skip ni échec; rapport
  archivé dans `docs/release/E2E-REPORT.md`.

## Compilateur MiniBASIC natif — extension post-V1

### COMP-001 — Étudier et figer la provenance Turbo-BASIC XL — 0,5 à 1,5 j

- **Jalon / exigences :** C0; COMP-001.
- **But :** cataloguer les manuels, images 1.5, Compiler 1.1, Runtime, Linker,
  désassemblage MADS et sources/outils communautaires ; produire une note de
  correspondance entre leurs rôles et MiniBASIC-RV.
- **Non-but :** copier du code historique ou promettre la compatibilité Atari.
- **Entrées/sources :** AtariWiki Turbo-BASIC XL, manuel du compilateur,
  artefacts communautaires avec licences et `docs/BASIC_LANGUAGE.md`.
- **Fichiers/modules :** `docs/BASIC_COMPILER_ROADMAP.md`, `docs/BASIC_TBXL_NOTES.md`,
  `norms/` ou manifeste de provenance.
- **Dépendances :** aucune ; bloque COMP-002 si les licences ou artefacts ne
  sont pas identifiables.
- **Tests :** vérification des hashes, licences, URLs archivées et rapport de
  provenance sans dépendance d’exécution.
- **Acceptation :** chaque artefact est classé étude, oracle, source utilisable
  ou interdit ; aucun fichier de build ne dépend d’un binaire historique.
- **Cas limites :** source déclaré perdu, désassemblage incomplet, miroir
  indisponible ; le lot doit alors laisser une preuve et un substitut explicite.
- **Parallélisable :** oui avec COMP-003, pas avec une décision de licence.
- **Contexte minimal :** D-019 et `docs/BASIC_COMPILER_ROADMAP.md`.

### COMP-002 — Extraire l’AST/IR commun — 1 à 2 j

- **Jalon / exigences :** C1; COMP-002..003.
- **But :** faire partager au mode interprété et au futur compilateur le lexer,
  l’AST, la résolution des lignes, les noms longs, chaînes et tableaux.
- **Non-but :** générer du RV64 ou optimiser.
- **Entrées/sources :** `BASIC_LANGUAGE.md`, D-018/D-019, parser MiniBASIC actuel.
- **Fichiers/modules :** crates/modules BASIC parser, AST/IR, diagnostics et
  tests de fixtures.
- **Dépendances :** COMP-001 informatif ; bloque COMP-003/004.
- **Tests :** AST golden, erreurs avec ligne/colonne, équivalence d’analyse
  interpréteur/compilateur, expressions et `FLOATLOOP`.
- **Acceptation :** une source valide produit un AST déterministe identique dans
  les deux chemins ; une source invalide garde le même code d’erreur.
- **Cas limites :** ligne vide, commentaire, 16 caractères, parenthèses,
  chaînes contenant des séparateurs et tableau hors limite.
- **Parallélisable :** oui avec COMP-001 et l’oracle Rust ; non avec COMP-004.
- **Contexte minimal :** grammaire BASIC et tests existants du parser.

### COMP-003 — Produire du RV64 pour les expressions binary64 — 1 à 2 j

- **Jalon / exigences :** C2; COMP-004..006.
- **But :** générer un payload RV64 pour affectations et expressions, incluant
  `fadd.d`, `fsub.d`, `fmul.d`, `fdiv.d`, conversions et source mapping.
- **Non-but :** contrôle de flot complet, chaînes, tableaux et optimisations.
- **Entrées/sources :** R1 F/D, R2 généré, D-019, ABI payload guest.
- **Fichiers/modules :** backend compiler, générateur assembly/object, listing,
  `run-at` et symboles.
- **Dépendances :** COMP-002, ISA générée, runtime D existant ; bloque COMP-004.
- **Tests :** golden RV64, désassemblage, QEMU `fdiv.d`, motifs binary64 et
  `fflags`, comparaison indépendante avec interpréteur et référence IEEE.
- **Acceptation :** `X=I/3` atteint réellement `fdiv.d` dans le payload compilé,
  avec ligne source et résultat observables.
- **Cas limites :** division zéro, NaN, ±0, débordement immédiat et expression
  non compilable doivent produire un diagnostic ou trap défini.
- **Parallélisable :** oui avec COMP-001 ; non avec changement d’ABI FP.
- **Contexte minimal :** `BASIC_COMPILER_ROADMAP.md`, tests expression D/QEMU.

### COMP-004 — Compiler contrôle de flot et services cible — 1 à 2 j

- **Jalon / exigences :** C3; COMP-007..010.
- **But :** compiler `IF/GOTO`, `FOR/NEXT`, `PRINT`, `INPUT`, `END` et produire
  `COMPILE`/`RUN-COMPILED` avec runtime target-side.
- **Non-but :** optimisation globale ou compatibilité ELF externe.
- **Entrées/sources :** ABI ecall guest, BASIC_LANGUAGE, COMP-002/003.
- **Fichiers/modules :** lowering CFG, linker/runtime, commandes, loader et
  source map.
- **Dépendances :** COMP-003, payload loader, services UART/console ; bloque
  COMP-005.
- **Tests :** programmes direct, FLOATLOOP, erreur GOTO, INPUT et interruption,
  exécutés sous QEMU sans sortie codée en dur.
- **Acceptation :** le payload compilé est chargeable, exécutable et arrêtable
  comme un programme utilisateur normal.
- **Cas limites :** boucle infinie, saut absent, pile FOR pleine, entrée invalide
  et runtime incompatible.
- **Parallélisable :** partiellement avec COMP-005 sur le format de manifeste.
- **Contexte minimal :** `GUEST_PAYLOAD_ABI.md`, tests `run-at` et debugger.

### COMP-005 — Étendre au runtime chaînes/tableaux et au debug — 1 à 2 j

- **Jalon / exigences :** C4; COMP-011..014.
- **But :** compiler les constructions couvertes par D-018 et exposer variables,
  breakpoints, listing, carte source et DUMP du payload compilé.
- **Non-but :** Unicode, allocation dynamique hôte, optimisation agressive.
- **Entrées/sources :** D-018, fixtures chaînes/tableaux, format snapshot.
- **Fichiers/modules :** runtime target, descripteurs, compiler lowering,
  source map, debugger et export `.luna`.
- **Dépendances :** COMP-004, BASIC-STR/ARRAY, FORMAT-001 ; bloque COMP-006.
- **Tests :** chaînes vides/pleines, `DIM`, index 0/dernière borne, snapshots,
  breakpoint source et inspection mémoire.
- **Acceptation :** un programme compilé avec chaînes/tableaux s’exécute et ses
  objets sont inspectables sans pointeur hôte.
- **Cas limites :** pool plein, dimension invalide, index hors borne, artefact
  incompatible et restauration.
- **Parallélisable :** oui avec documentation ; non avec changement D-018.
- **Contexte minimal :** D-018, `BASIC_COMPILER_ROADMAP.md`, tests array/string.

### COMP-006 — Mesurer puis ajouter les optimisations sûres — 1 à 2 j

- **Jalon / exigences :** C5; COMP-015..018.
- **But :** mesurer le gain et ajouter uniquement les optimisations prouvées
  sûres : constantes, temporaires et sauts triviaux.
- **Non-but :** réimplémenter le compilateur historique ou sacrifier le debug.
- **Entrées/sources :** corpus MiniBASIC, COMP-004/005, politique FP.
- **Fichiers/modules :** passes d’optimisation, benchmarks, manifests et tests.
- **Dépendances :** COMP-005 ; bloque la publication du profil compiler optimisé.
- **Tests :** différentiel interprété/compilé, `fflags`, nombre d’instructions,
  breakpoints, répétabilité multi-plateforme et corpus de non-régression.
- **Acceptation :** chaque option est déterministe, documentée et désactivable ;
  aucune différence de résultat ou de flag sur le corpus couvert.
- **Cas limites :** effets d’E/S, NaN, exceptions, variables observées et trace.
- **Parallélisable :** oui avec QUAL-001 après gel des IR.
- **Contexte minimal :** D-019, plan de tests compiler et corpus E2E.
