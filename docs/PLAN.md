# Plan de développement — moniteur-assembleur RV64ILP32

## 1. Résumé exécutif

Le projet sera livré comme un workspace Rust multi-crates, avec un cœur sans UI et un frontend terminal initial. Le premier incrément démontrable est volontairement étroit : générer l’encodage de `addi`, assembler `addi x1,x0,1`, charger l’image dans une RAM virtuelle, exécuter exactement un pas et montrer `x1=1`. Le deuxième incrément à risque est `fadd.s`, avec motif de résultat et `fflags` comparés à un oracle externe.

Le profil hérité est `rv64imafd_zicsr_zifencei`, little-endian, U mono-hart, `XLEN=64`, `FLEN=64`, ABI locale `RV64ILP32D-MON-1`, C optionnel, A non exécuté par défaut, Zfh/Q data/decode-only. La génération des encodages depuis `riscv-opcodes` est une condition de conception, pas une tâche de confort.

### Effort et capacité

Estimation essentielle LE : 55–75 journées-agent, hors frontend graphique,
export ELF complet et multi-hart. Marge de risque LE : 20–30
journées-agent, principalement flottants, C, ILP32 et oracles. Le profil BE
complet est un lot post-V1 de 20–35 journées-agent essentielles, plus 10–20
de marge toolchain/QEMU. Optionnel V1 : 8–15 journées-agent pour ELF contrôlé,
historique arrière et polish terminal. Hypothèse : 2 agents de code à temps
plein, 1 agent validation/revue à temps partiel ; les fourchettes ne sont pas
des dates calendaires.

Le guest QEMU de référence utilise une RAM physique de 64 MiB avec des régions
distinctes pour image fixe, code cible et données cible. Cette extension ne
change pas le budget de la machine hôte sparse ; elle prépare les tests de
pointeurs, pile et données éloignées.

### Conversion indicative en tokens GPT-5.6 Luna High

Une journée-agent n’est pas une quantité de tokens fixe : elle inclut les
attentes d’outils, les lectures de contexte, les erreurs, les reprises et la
revue locale. Pour budgéter sans fausse précision, ce plan utilise **40 000 à
80 000 tokens de travail total par journée-agent**, entrée + sortie +
raisonnement du modèle, avec une valeur centrale de 60 000.

| Unité | Budget indicatif |
|---|---:|
| 0,5 journée-agent / 1 point | 20k–40k tokens |
| 1 journée-agent | 40k–80k tokens |
| 2 journées-agent | 80k–160k tokens |
| LE essentiel, 55–75 j | 2,2M–6,0M tokens |
| marge LE, 20–30 j | 0,8M–2,4M tokens |
| BE essentiel, 20–35 j | 0,8M–2,8M tokens |
| marge BE/toolchain, 10–20 j | 0,4M–1,6M tokens |

Le cœur LE + risques LE + BE + marge BE représente donc environ **4,2M à
12,8M tokens**, soit 6,3M–9,6M au point central. Le parallélisme réduit la
durée calendaire, pas le volume total de tokens. Une tâche d’oracle, de
flottants ou d’intégration QEMU peut dépasser cette fourchette de 30–100 % ;
elle doit alors être découpée ou marquée à risque. Ces nombres servent au
capacity planning et ne prédisent ni une facture ni une limite de contexte.

## 2. Contrôle de cohérence

### Écarts et décisions nécessaires

1. **R2 n’est pas réellement figé** : `c6edca7` est un SHA court. Le premier code dépend d’un SHA complet et d’un artefact hashé.
2. **R3 pointe vers `master`** : impossible de reproduire la validation ABI. À figer avant import/export ELF et avant toute convention ABI testée.
3. **Carte pointeur haute vs RAM/MMIO** : la spécification définit la traduction `sign_extend_32`, mais pas si une allocation dans la fenêtre haute est physiquement aliasée vers une RAM basse. Défaut : deux fenêtres logiques indépendantes ; aucune aliasing ; les allocations hautes sont explicites et bornées.
4. **ELF32/flags** : `ELFCLASS32` ne suffit pas à reconnaître RV64ILP32. Défaut : import rejeté si machine, classe, flags float, ABI manifest ou relocation ne concordent pas ; l’ELF externe, en particulier BE, est explicitement hors chemin critique M2–M9.
5. **Outils GNU/LLVM** : leur acceptation de RV64ILP32 varie. Défaut : les tests différentiels marquent `unsupported-by-oracle` avec preuve de version et ne deviennent jamais des tests auto-comparatifs.
6. **IEEE 754 déterministe** : la spécification exige un résultat indépendant de l’hôte, mais ne choisit pas de bibliothèque. Défaut : prototype contre Berkeley SoftFloat ou équivalent logiciel audité ; aucun fallback implicite vers les opérations hôte.
7. **C** : C est optionnel et son émission automatique est désactivée. Défaut : C explicitement activable dès que le décodeur 16/32 est stable, sans relaxation automatique.
8. **Historique arrière** : le journal inverse est retenu mais son coût n’est pas borné en octets. Défaut : quota configurable et test de pression ; la fonctionnalité peut être désactivée sans casser undo transactionnel.
9. **Big-endian RISC-V** : l’ISA autorise des accès données big-endian mais le
   profil et l’ELF/ABI doivent être cohérents. Défaut : profil BE distinct,
   instructions toujours little-endian, `MBE=1`/`UBE=1`, aucune bascule mixte en
   session ; QEMU et la toolchain BE sont validés comme dépendances séparées.

Ces points ne sont pas corrigés silencieusement ; ils deviennent ADR et tests de contrat.

## 3. Étape 0 — décisions bloquantes avant le premier code

| Décision | Défaut obligatoire | Condition de sortie |
|---|---|---|
| SHA R2 | commit complet correspondant à `c6edca7`, avec archive des fichiers générés | manifest R2, SHA vérifié deux fois, licence enregistrée |
| SHA R3 | commit complet du snapshot observé le 31-07-2026 | manifest R3, HTML/AsciiDoc hashé, `CONFLICT-ABI-001` accepté |
| R2↔R1 | script de contrôle qui compare au moins les instructions du profil | rapport zéro divergence non expliquée |
| pointeurs hauts | fenêtres basse/haute non aliasées, `sign_extend_32`, mapping explicite par segment | table de carte mémoire et tests 0x7fffffff/0x80000000/0xffffffff |
| ELF | import strict, refus des combinaisons ambiguës, format `.luna` canonique | corpus ELF accepté/refusé et codes stables |
| langage | Rust stable piné par `rust-toolchain.toml`, cœur sans `unsafe` par défaut | build reproductible et audit des dépendances |
| oracle FP | SoftFloat logiciel audité comme oracle d’exécution, Sail/Spike pour sémantique globale | prototype `fadd.s` comparé sur 1000 motifs |

Étape 0 ne bloque pas l’écriture de fixtures et de scripts de vérification indépendants, mais bloque les crates de production qui consomment les encodages ou l’ABI.

## 4. Architecture de dépôt proposée

```text
/
  Cargo.toml                         # workspace, versions communes
  rust-toolchain.toml                # toolchain pinée
  norms/                              # snapshots R1–R5, manifests, licences
  generated/opcodes/<r2-sha>/        # tables dérivées, manifest, hash, pas de saisie manuelle
  crates/
    diag/                             # Diagnostic, codes, Location, Severity
    profile/                          # ISA/ABI/environment manifests et capabilities
    bits/                             # bitvectors, endian, widths, bit patterns
    opcode-gen/                       # import R2, validation R1, génération
    isa/                              # encode/decode, pseudos, C 16/32
    float/                            # formats, SoftFloat adapter, fcsr, display
    abi/                              # types, sign extension, ELF flags/validation
    asm-lexer/                        # tokens, Unicode policy, comments
    asm-parser/                       # AST, expressions, directives, macros
    assembler/                        # passes, symbols, relocations, listing
    memory/                           # sparse RAM, MMIO contract, transactions
    target-api/                       # TargetBackend, context, capabilities, outcomes
    machine/                          # state, hart, CSR, traps, deterministic backend
    debugger/                         # break/watch, stepping, source mapping, history
    formats/                          # project, image, snapshot, symbols, ELF limited
    command/                          # EBNF command parser/evaluator
    monitor-model/                    # views, marks, QuickJump, selection
    frontend-terminal/                # ratatui/crossterm adapter, keymap
    app/                              # composition, persistence, CLI entrypoint
  tests/
    golden/                           # fixtures R2, GNU, LLVM, Sail/Spike
    e2e/                              # scénarios SPEC 1–14
    fuzz/                             # targets, seeds, reducers
  tools/                              # scripts oracle, lock, corpus, benchmarks
  docs/                               # guide développeur/utilisateur
```

Frontières : `target-api` ne dépend d’aucun transport et ne contient pas la sémantique machine ; `machine` implémente `TargetBackend` sans connaître AST ni UI ; `assembler` ne dépend pas de `machine` ; `isa` dépend des tables générées, jamais de texte saisi ; `formats` sérialise des contrats versionnés ; le frontend ne manipule jamais directement la RAM.

## 5. Langage, bibliothèques et coût de verrouillage

Rust stable est recommandé pour les invariants de largeur, ownership des snapshots, portabilité et fuzzing. Alternatives : C++ (meilleur accès à certains oracles mais coût mémoire/ownership plus élevé), Go (ergonomie mais moins adapté au bit-level et au frontend terminal riche). Le choix Rust verrouille l’écosystème Cargo et impose des wrappers FFI audités si SoftFloat est retenu.

Bibliothèques candidates à figer après M0 : `thiserror`/`miette` ou équivalent interne pour diagnostics ; `serde` + format JSON canonique ; `proptest` et `cargo-fuzz` ; `ratatui`/`crossterm` pour le terminal. Les versions exactes et licences sont enregistrées dans `norms/dependencies.lock`; aucune bibliothèque ne fournit la sémantique ISA sans comparaison externe.

Pour FP, le défaut est un backend logiciel dérivé/vendored d’un SoftFloat audité ; alternative : `rustc_apfloat` si sa couverture et sa licence conviennent. Le prototype M4 doit mesurer payload NaN, sous-normaux, `frm` et flags avant verrouillage.

## 6. Politique de normes et données générées

Chaque build porte les identifiants R1–R5 et SHA. Le générateur R2 produit `mask`, `match`, champs, contraintes, pseudos, imports et métadonnées d’extension. Les artefacts générés sont committés pour revue mais ne sont jamais édités manuellement ; CI les régénère et compare sans tolérance. Un changement de SHA ouvre un ADR, reconstruit les fixtures et exige une migration de profil.

R1 reste l’autorité sémantique ; le contrôle R2 détecte une divergence mais ne la résout pas automatiquement. Les oracles GNU/LLVM/Sail/Spike sont des dépendances de test, pas de runtime V1.

## 7. Tranches verticales et jalons

| Jalon | Démonstration | Scénarios SPEC | Sortie minimale |
|---|---|---|---|
| M0 | dépôt reproductible et sources gelées | prérequis 1, 6, 10, 12 | CI, manifests, licences, ADR ouverts |
| M1 | `addi` encode/decode avec tables générées | E2E 1 partiel, 10 | golden R2, encode↔decode, illegal opcode |
| M2 | source→RAM→step→`x1=1`, loads/stores RV64 de base | E2E 1 | tranche entière sans UI obligatoire |
| M3 | symboles/directives/diagnostics/disassemble | E2E 3, 9, 10, 14 | listing, C mixte, round-trip |
| M4 | `fadd.s`, résultat bit-exact, `fflags` | E2E 5, 6, 13 | oracle FP indépendant |
| M5 | D, données binary16/128, Zfh/Q decode-only | E2E 7 | profil matrix et refus d’exécution |
| M6 | commandes, mémoire hex/ASCII/dis, marks | E2E 2, 3 | terminal utilisable |
| M7 | break/watch/step/source/history | E2E 4, 11 | debugger contractuel |
| M8 | projets, snapshots, export/reprise/migration | E2E 12, 14 | replay hash-identique |
| M9 | fuzz, perf, accessibilité, release candidate | tous | rapport RC signé |
| M10-BE | contrat d’endianess, profils LE/BE, sérialisation et ELF MSB | BE-001 à BE-002 | simulateur LE inchangé, matrice endian complète |
| M11-BE | simulateur interne BE : loads/stores, pile, CSR, traps, flottants | BE-003 | exécution bit-exacte LE/BE sur le même corpus |
| M12-BE | image guest BE, contexte de trap et toolchain/ELF qualifiés | BE-004 à BE-005 | boot bare-metal BE ou blocage toolchain documenté |
| M13-BE | QEMU `big-endian=on`, smoke test et mode pédagogique final | BE-006 à BE-007 | démonstration QEMU BE reproductible, sinon waiver explicite |
| C0 | étude Turbo-BASIC XL 1.5, provenance et architecture compiler/runtime/linker | COMP-001 | corpus et note historique licenciés |
| C1 | AST/IR partagé interpréteur-compilateur | COMP-002 | même source analysée, diagnostics identiques |
| C2 | backend RV64 des expressions binary64 | COMP-003 | payload compilé avec `fdiv.d` observable |
| C3 | contrôle de flot, E/S et payload compilé chargeable | COMP-004 | `COMPILE`/`RUN-COMPILED` reproductibles |
| C4 | chaînes, tableaux, debug source et artefacts | COMP-005 | sous-ensemble étendu compilable et inspectable |
| C5 | optimisation mesurée et release compiler optionnelle | COMP-006 | équivalence et gains mesurés, sans régression |

Chaque jalon conserve un build vert et une démonstration scriptée. M1–M4 sont prioritaires sur le polish UI.

### Priorisation révisée

À la demande du projet, les travaux post-M9 sont réordonnés ainsi :

1. **P0 — MiniBASIC chargé comme payload assembleur** : cette priorité est
   décomposée en trois preuves qui ne doivent pas être confondues :
   l’assembleur guest Rust embarqué existe déjà et couvre les programmes
   bornés, mais doit encore accepter le source complet du runtime MiniBASIC
   (sections, directives de données, symboles/relocations et image multi-
   section) ; le transfert binaire actuel doit ensuite devenir sparse (`clear`
   de plage puis blocs non nuls) ; enfin `assemble-load` devra assembler et
   charger atomiquement ce payload depuis le moniteur guest. Le générateur GNU
   `as` sur l’hôte reste un oracle/build intermédiaire explicite, jamais la
   preuve de l’assemblage guest. Le MiniBASIC résident reste un mode de secours
   jusqu’à validation du payload assemblé et chargé dans la cible.
2. **P1 — capacité du magasin BASIC** : augmenter la capacité du programme
   BASIC, avec limites centralisées, diagnostic de saturation, tests de
   consommation de pile M-mode et compatibilité snapshot/projet. La capacité
   du source assembleur guest distingue maintenant 4096 lignes persistantes
   et 9216 lignes pour `assemble-program`, avec scratch statique ; elle ne
   préjuge pas de la capacité du programme
   BASIC. Aucune extension ISA n’est ajoutée.
3. **P2 — interface graphique et historique arrière** : stabiliser d’abord les
   contrats de modèle et d’événements, puis ajouter un frontend graphique sans
   exposer directement la RAM cible. L’historique arrière doit rester borné,
   déterministe et indépendant de l’undo transactionnel existant.
4. **P3 — compilateur MiniBASIC natif** : étudier d’abord les artefacts
   Turbo-BASIC XL 1.5, puis construire un backend RV64 explicite partageant
   l’analyse syntaxique avec l’interpréteur. Cette extension reste séparée du
   jalon de chargement du payload interprété et ne doit pas retarder le support
   des chaînes et tableaux dans `RUN`.

Cette priorité repousse la clôture de release publique complète après P0–P2,
mais ne modifie pas le profil ISA/ABI ni les preuves déjà acquises.

## 8. Migration du profil minimal

Le profil de bootstrap est `rv64i_zicsr_zifencei` uniquement, utilisé pour valider pipeline et traps. M2 ajoute M et les loads/stores nécessaires. M4 active F en conservant les mêmes interfaces d’état ; M5 active D et les formats data sans activer Q/Zfh en exécution. C est ajouté par capability et ne modifie pas le curseur mémoire. A reçoit un profil séparé `A-MH1`, jamais un booléen global. Toute extension future doit fournir parse/assemble/decode/execute/debug/test/export séparément et un corpus externe.

### Migration vers le profil big-endian

Le profil BE ne réutilise pas implicitement les fixtures LE. M10 introduit
`Endian::Little`/`Endian::Big`, un `ProfileId` distinct et des serializers
paramétrés ; l’ISA générée et les fetchs d’instructions restent communs. M11
porte les accès multi-octets de la machine interne, le contexte de trap, les
registres flottants et les pointeurs ILP32 dans les deux endianness. M12 vérifie
qu’une chaîne de compilation peut produire un ELF RISC-V BE cohérent
(`EI_DATA=ELFDATA2MSB`) ; à défaut, le profil reste exécutable uniquement par
fixtures/assembleur contrôlés et n’est pas annoncé compatible GNU/LLVM. M13
branche QEMU avec `-cpu <model>,big-endian=on`, vérifie `MBE`/`UBE`, l’UART et
les instructions little-endian, puis publie un corpus de replay LE/BE. Les
tests de pile vérifient séparément la décroissance de `sp` et l’ordre des
octets des frames ; aucune vue mémoire ne doit simuler le BE en parcourant les
adresses à rebours.

## 9. Parallélisation et intégration

Parallélisables après M0 : générateur R2, diagnostics, FP oracle adapter, lexer/parser, memory transactions, fixtures d’oracle. À intégrer tôt : `ProfileId`, `Diagnostic`, `DecodedInstruction`, `MachineState`, `ObjectImage`, `SnapshotManifest`. Un agent ne modifie pas une interface publique sans ADR ou contrat de test. Les branches fusionnent par tranche, avec rebase limité et test de compatibilité des artefacts générés.

## 10. Risques et réduction précoce

* **ILP32/ELF** : tests de frontières et corpus refusé en Étape 0 ; import ELF repoussé hors chemin critique.
* **Encodages** : générateur + contrôle R1 dès M1 ; aucune table manuelle.
* **FP** : prototype SoftFloat contre GNU/Sail/Spike avant généralisation ; commencer par `fadd.s`.
* **C** : décodeur longueur variable testé avant relaxation ; émission automatique désactivée.
* **Pas-à-pas** : machine synchrone et compteur d’instructions dès M2 ; UI n’est pas l’horloge.
* **Fuzzing** : targets parser/decoder/command dès M3, budgets CI séparés.
* **Reproductibilité** : manifests et hash de chaque entrée dès M0, snapshots sans horodatage.
* **Compiler historique** : les sources originales n’étant pas garanties,
  verrouiller d’abord provenance/licence et utiliser le désassemblage comme
  référence de comportement, jamais comme code généré automatiquement.
* **Divergence interprété/compilé** : commencer par un corpus différentiel
  target-side et refuser toute optimisation avant l’équivalence bit/flags.
