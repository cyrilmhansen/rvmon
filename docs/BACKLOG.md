# Backlog atomique

Convention : 1 point ≈ 0,5 journée-agent ; les estimations incluent tests locaux mais pas revue finale. Pour un agent GPT-5.6 Luna High, on budgète
indicativement 20k–40k tokens de travail total par point, ou 40k–80k par
journée-agent. Ce sont des tokens de contexte + sortie + raisonnement ; ils ne
mesurent pas la taille du diff et ne remplacent pas la condition de sortie.
Une tâche est terminée seulement si sa condition de sortie est satisfaite.

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
- **Étapes réalisées :** épingler QEMU user-mode 11.0.2 dans le manifeste; construire un probe RISC-V autonome pour quatre cas F et quatre cas D; comparer motifs de résultat et fflags au moteur; vérifier qu’une mutation du candidat est détectée; enregistrer la décision D-016.
- **Dépendances/bloqués :** BOOT-003; BOOT-001 pour versions; bloque FP-002.
- **Tests :** bash tools/check-fp-oracle.sh; corpus fixe; comparaison QEMU/machine; mutation de la chaîne candidate.
- **Acceptation :** QEMU 11.0.2 et le moteur concordent sur huit cas; la mutation contrôlée diverge; choix et licence sont enregistrés.
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
