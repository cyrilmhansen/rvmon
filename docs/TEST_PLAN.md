# Plan de tests et stratégie d’oracles

## 1. Pyramide

1. **Unitaires** : bits, endian, registres, diagnostics, expressions, formats FP, transactions.
2. **Propriétés/génératifs** : encode↔decode, sign extension, alignements, parser AST, snapshots.
3. **Composants** : opcode generator, assembler, memory, machine, debugger, format readers.
4. **Différentiels externes** : GNU, LLVM, Sail, Spike, SoftFloat selon capacité et version.
5. **E2E interactifs** : scénarios SPEC 1–14, terminal scripté, replay.
6. **Release** : cross-platform, fuzz nightly, performances, accessibilité et provenance.

Un test auto-comparant assembleur et désassembleur est une propriété utile, mais ne compte pas comme preuve externe de l’encodage ou de la sémantique.

## 2. Matrice de conformité

| Extension | Parse | Assemble | Decode | Execute | Debug | Export | Oracle principal |
|---|---:|---:|---:|---:|---:|---:|---|
| I | oui | oui | oui | oui | oui | brut/.luna | GNU/LLVM/R1 |
| M | oui | oui | oui | oui | oui | brut/.luna | Sail/Spike |
| F | oui | oui | oui | oui | oui | brut/.luna | SoftFloat/Sail/Spike |
| D | oui | oui | oui | oui | oui | brut/.luna | SoftFloat/Sail/Spike |
| Zicsr/Zifencei | oui | oui | oui | oui | oui | brut/.luna | Sail/Spike |
| C | oui | oui explicite | oui | profil C | oui | brut | GNU/LLVM/R1 |
| A | oui | oui | oui | profil A-MH1 seulement | oui | brut | Sail/Spike |
| Zfh/Zfhmin | oui | oui | oui | non V1 | affichage | données | R1/R2 encode |
| Q | oui | data only | oui | non V1 | affichage | données | R1/R2 encode |
| B/V/Zfa/S/M/crypto | rejet ou expérimental isolé | non V1 | non V1 | non | non | non | negative corpus |

## 3. Golden et génération R2

`tools/opcode-gen` est la seule source de tables. Pour chaque commit R2 figé, CI :

* régénère `mask`, `match`, champs, pseudos/imports et métadonnées ;
* compare les artefacts au commit ;
* vérifie largeur, overlap, valeurs constantes et champs complets ;
* compare les instructions du profil à R1 ;
* exécute des fixtures d’encodage connus et encode↔decode.

Un diff de génération est bloquant sauf ADR de mise à jour de profil. Une table éditée à la main est une violation de release.

## 4. Tests assembler/désassembleur

Chaque fixture assemble→bytes→désassemble→forme canonique→réassemble. Les pseudos doivent conserver l’expansion et les bytes. Les tests couvrent : immédiats positifs/négatifs, `hi20/lo12`, relocations, labels globaux/locaux, directives, alignement, macros, strings, erreurs de largeur, C 16/32 mêlé, données mêlées au code et opcode illégal.

Les différentiels GNU `as/objdump` et LLVM `llvm-mc` sont exécutés uniquement avec versions déclarées et seulement pour les sous-ensembles acceptés par ces outils. Un refus d’un outil est enregistré comme limitation d’oracle ; il ne transforme pas le test en comparaison avec le propre assembleur.

## 5. Tests sémantiques externes

Pour I/M/F/D/Zicsr/Zifencei et C couvert, générer des programmes courts avec états initiaux contrôlés, exécuter le backend, Sail et Spike lorsque chacun supporte le profil. Comparer PC, x/f/CSR, mémoire, flags et cause de trap. Les différences sont triées `backend`, `oracle unsupported`, `norm conflict` ou `test defect`.

Sail est l’oracle sémantique formel principal lorsque sa configuration couvre l’extension ; Spike est un second oracle pratique. Aucun oracle n’est embarqué dans le runtime V1.

## 6. Propriétés

* `decode(encode(i)) == canonical(i)` pour toutes instructions générées valides ;
* `encode(decode(bytes))` conserve les bytes pour les instructions non ambiguës ;
* sign-extension d’un pointeur : `sext32(sext32(v)) == sext32(v)` et valeurs frontière exactes ;
* les écritures x0 restent zéro ;
* les accès width/alignement respectent little-endian et ne mutent pas après erreur ;
* une plage inversée ou un overflow d’expression est rejeté ;
* un snapshot restore rend un hash d’état identique ;
* un `run` borné termine toujours dans la limite ;
* une condition de breakpoint/watchpoint ne peut muter l’état ;
* les offsets C/32-bit font progresser le PC de 2/4 octets selon décodage ;
* une génération R2 répétée est idempotente.

## 7. Flottants

Corpus par format binary16/32/64/128 : `+0`, `-0`, `+inf`, `-inf`, min/max normal, min/max subnormal, overflow, underflow, exact/inexact, sNaN/qNaN avec plusieurs payloads. Tester décimaux, hex flottants et `bitsN`; `bfloat16` est un corpus de rejet pour `.binary16`.

Pour F/D : tester `fadd.s`, `fadd.d`, conversions retenues, rm statique/dynamique, `frm` réservé, `fflags` sticky, NaN-boxing valide/invalide, entier↔flottant et format↔format. Comparer motif de résultat et `NV/DZ/OF/UF/NX` à SoftFloat puis Sail/Spike quand applicable. L’affichage décimal doit être shortest-round-trip et l’hex toujours exact.

## 8. ILP32 et mémoire

Fixtures d’appels couvrant `a0..a7`, `fa0..fa7`, variadiques, structures ≤/> XLEN, retours indirects, pile alignée 16, pointeurs 32 bits signés. Cas obligatoires : `0x7fffffff`, `0x80000000 → 0xffffffff80000000`, `0xffffffff → 0xffffffffffffffff`, addition overflow et adresse RV64 explicite distincte d’un pointeur.

Tester fenêtres basse/haute non aliasées, RAM, MMIO, unmapped, cross-page, misalignment, quota et transaction rollback. ELF fixtures : classe/machine/flags compatibles, `ELFCLASS32` seul, flags flottants contradictoires, relocation non supportée.

## 8-BE. Matrice little-endian / big-endian

Le profil LE reste la référence V1. Le profil BE expérimental réutilise les
encodages et le fetch d’instructions, mais exécute un corpus parallèle pour
les accès données. Chaque cas ci-dessous doit être exécuté avec les deux
profils, et les bytes attendus sont distincts uniquement lorsque la largeur
de l’accès est supérieure à un octet :

| Cas | LE attendu | BE attendu | Contrôle |
|---|---|---|---|
| `.byte 0x12` | `12` | `12` | invariance par octet |
| `.half 0x1234` | `34 12` | `12 34` | store/load 16 bits |
| `.word 0x11223344` | `44 33 22 11` | `11 22 33 44` | store/load 32 bits |
| `.dword 0x0102030405060708` | `08..01` | `01..08` | store/load 64 bits |
| binary128 motif | octet faible en premier | octet fort en premier | 16 bytes exacts |
| instruction `addi` | parcels LE | parcels LE | fetch identique |
| `fadd.s`/`fadd.d` | résultat identique | résultat identique | registres/flags |

Tests spécifiques BE : vérifier `MBE=1` en M-mode, `UBE=1` en U-mode,
l’invariance des adresses d’octets, le contexte de trap, la pile, les accès
UART byte-wide, les pointeurs ILP32 et `ELFDATA2MSB`. Les tests de pile doivent
prouver séparément que `sp_new < sp_old` après allocation et que les octets
d’un champ multi-octets sont rangés aux adresses `A`, `A+1`, etc. selon le
profil ; ils ne doivent jamais utiliser une adresse décroissante pour
représenter le BE. Aucun test ne doit
autoriser une bascule endianess implicite pendant une instruction ou une
transaction. Le smoke QEMU utilise la propriété CPU `big-endian=on`, jamais
une inversion opérée par le moniteur.

## 9. Instructions illégales et données mêlées

Corpus de bytes sans match, match d’extension désactivée, longueur 16 tronquée, 32 tronquée, CSR non implémenté, instruction privilégiée et C off. Le désassembleur émet un item illégal ; le moteur trap avec PC/bytes/cause. Les marks de données empêchent une exécution implicite mais ne masquent pas l’octet.

## 10. Fuzzing

Targets : lexer/parser, expression evaluator, opcode decoder, assembler, command parser, snapshot reader, path validator. Budget PR : 60 secondes par target avec seeds fixes ; nightly : 30 minutes par target ; pré-release : 2 heures par target. Chaque crash conserve seed, version des normes, profil, input réduit et commande de replay. Le reducer doit produire un cas plus court sans perdre le défaut. Timeout et OOM sont des résultats à classer, pas à ignorer.

## 11. Interactif et ASM-One modernisé

Scripts terminal couvrant : assemble/load/run/step, edit memory/undo, code↔hex↔ASCII, QuickJump, marks, regs changes, breakpoint/watch, diagnostics source, snapshot/restore et crash recovery. Les scénarios SPEC 1–14 sont des tests de release, avec au moins un test négatif par commande et une vérification de non-effet de bord.

## 12. Reproductibilité et seuils

Exécuter sur Linux/macOS/Windows x86_64 et arm64 quand disponibles. Les mêmes manifest, source, profile et initial state doivent produire les mêmes hashes d’image, snapshot et trace ; les chemins hôte sont normalisés ou anonymisés explicitement. Seuils : couverture lignes cœur ≥90 %, branches critiques (encode/decode, traps, FP flags, ABI) ≥85 %, 100 % des codes diagnostics documentés testés, zéro flaky test accepté, zéro violation de génération R2.

Les seuils de performance SPEC sont bloquants pour release ou exigent un waiver signé : assemblage p95 <1 s/64 KiB, latence commande p95 <50 ms hors run, snapshot sparse <200 ms, UI mémoire au repos <256 MiB.

## 13. Tests du compilateur MiniBASIC (post-V1)

Le compilateur possède une matrice distincte de l’interpréteur :

- tests AST/IR comparant l’analyse commune, sans comparer deux exécutions du
  même backend ;
- golden RV64 générés à partir de l’IR et vérifiés par désassemblage et
  encodages R2 générés ; aucune table d’opcodes compilateur saisie à la main ;
- exécution QEMU du payload compilé pour expressions, `fdiv.d`, `PRINT`,
  `INPUT`, `IF/GOTO`, `FOR/NEXT`, puis chaînes et tableaux ;
- tests différentiels interprété contre compilé sur sorties, mémoire, résultats
  binary64 et `fflags`, avec un contrôle indépendant par motifs de bits ;
- vérification de la carte source : breakpoint sur une ligne BASIC, symbole
  compilé et instruction RV64 correspondante ;
- tests de refus : instruction non compilable, dimension invalide, runtime ou
  profil incompatibles, artefact corrompu et option inconnue ;
- benchmarks séparant temps interprété, temps compilé et coût de compilation,
  sans transformer un gain historique Atari en exigence RV64.

Le corpus historique AtariWiki est conservé comme entrée d’étude et de
provenance, pas comme oracle binaire : les artefacts sont utilisés selon leur
licence et aucune sortie historique ne prouve la conformité MiniBASIC-RV.
