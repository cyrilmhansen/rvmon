# État des tests RVMonitor

Ce document décrit le périmètre effectivement couvert par les tests présents dans le dépôt sur la branche courante. Il complète TEST_PLAN.md, qui décrit la stratégie cible et contient des fonctions encore non implémentées.

## Commandes de validation

Validation locale complète :

    cargo fmt --all
    bash tools/check-r2.sh
    bash tools/check-oracles.sh
    cargo test --workspace
    cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf
    bash scripts/test-guest-monitor.sh
    bash scripts/test-qemu-gdb-backend.sh
    git diff --check

La suite actuelle exécute 87 tests unitaires/intégration répartis dans les crates. Les doc-tests compilent mais ne contiennent actuellement aucun cas. Le script QEMU ouvre en plus une session GDB RSP réelle, hors comptage Cargo.

Démonstration M-mode/U-mode sous QEMU :

    timeout 4s qemu-system-riscv64 -M virt -bios none \
      -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
      -nographic

La commande doit afficher le passage en U-mode, puis `trap: breakpoint` avec
l’adresse du `ebreak`. La terminaison par timeout est
attendue car le prompt de trap est volontairement bloquant dans cette tranche.

Démonstration minimale :

    cargo run -p luna-app

Moniteur texte interactif :

    cargo run -p luna-app -- --interactive

## Inventaire par crate

| Crate | Tests | Périmètre |
|---|---:|---|
| luna-abi | 2 | Extension de signe des pointeurs 32 bits et idempotence. |
| luna-memory | 2 | Little-endian, transactions atomiques et rollback après erreur. |
| luna-asm-lexer | 5 | Registres numériques/ABI, commentaires, décalages, chaînes UTF-8 et positions d’erreur. |
| luna-assembler | 17 | AST, alias ABI, expressions, symboles, directives, alignement, fadd.s et fadd.d. |
| luna-isa-core | 3 | Encodeurs `addi`, branches, sauts et `fadd.s`/`fadd.d` sans allocation, depuis les tables R2 partagées avec le guest ; commit R2 et champs générés validés. |
| luna-isa | 6 | Tables générées depuis R2, encodage/décodage entier et flottant via `luna-isa-core`. |
| luna-machine | 12 | Exécution entière, branches, mémoire, tables de pointeurs ILP32, fadd.s, fadd.d, NaN-boxing, flags, contrat backend et snapshot cible. |
| luna-disassembler | 7 | Format canonique, symboles, opcodes illégaux, C rejeté et round-trip. |
| luna-floatfmt | 3 | Bits hex exacts, décimal court, classes IEEE et NaN-box invalide. |
| luna-monitor | 19 | assemble → step → regs, affichage flottant, run borné, vues mémoire hex/ASCII, édition/undo, console backend-générique, marques QuickJump, breakpoints, watchpoints, symboles, pile, historique et persistance. |
| luna-target-api | 4 | Contexte de trap, capacités explicites RV64 bare-metal, codes `mcause`, contrat de layout, résultats et accès mémoire du backend commun. |
| luna-qemu-backend | 7 | Framing GDB RSP, checksum, lecture mémoire, layouts RV64 entier et F/D, stop reply, initialisation `?`, pas et budget nul. |
| luna-guest-monitor | 0 | Image bare-metal, boot QEMU, PMP, transition M→U, lecture/édition mémoire transactionnelle, `undo`, directives exactes `.word`/`.float`/`.binary128`, assemblage invité entier/flottant, `setf`, NaN-boxing et traps; vérifié par smoke test QEMU. |
| luna-app | 0 | Compilation du binaire et démonstration ; pas encore d’E2E terminal. |
| luna-diag | 0 | Types utilisés par les autres crates ; pas de test dédié. |

## Périmètre détaillé

### ABI et mémoire

Les tests ABI verrouillent les frontières de pointeurs RV64ILP32 déjà modélisées, notamment la représentation sign-étendue dans un registre RV64. Ils ne valident pas encore une convention d’appel complète.

Les tests mémoire couvrent les transactions de bytes et le little-endian 32 bits. MMIO, quotas, pages, alignement généralisé, snapshots et mémoire distante ne sont pas couverts.

### Lexer, parser et assembleur

Le lexer et le parser testent les labels avec ou sans instruction, les registres entiers/flottants, les alias ABI, les opérandes imm(registre), les expressions et les diagnostics syntaxiques. Les positions du lexer sont 1-based et comptées en scalaires Unicode ; les diagnostics lexicaux peuvent en outre fournir une longueur de surlignage.

Les formes assembleur testées sont :

    addi, add, sub, lui, lw, sw, beq, bne, jal, jalr
    fadd.s, fadd.d
    .byte, .half, .word, .dword, .ascii, .asciz, .align

Les expressions couvrent la précédence, les bases décimale/hexadécimale/binaire, les opérateurs unaires, les décalages, les symboles en avant et les débordements. Un test vérifie qu’un offset numérique de branche reste un offset après pc = 0.

### ISA et encodages

Le registre d’opcodes est produit depuis les extraits R2 épinglés. `bash
tools/check-r2.sh` vérifie le SHA complet du commit, l’ensemble exact des
fichiers attendus, leurs empreintes SHA-256 et déclenche la validation du
générateur. Il vérifie aussi le SHA-256 de la table générée effectivement
produite dans `OUT_DIR`. Les tests vérifient ensuite la présence des données générées et
les round-trips des formes entières, de fadd.s et de fadd.d. Le générateur
signale les recouvrements `mask/match` compressés dont la distinction dépend
de contraintes d’opérandes R2 (`rd_n0`, etc.) ; un doublon exact reste une
erreur bloquante.

Cette vérification ne remplace pas encore une comparaison indépendante avec GNU, LLVM, Sail, Spike ou SoftFloat.

Le contrôle `bash tools/check-oracles.sh` constitue la première preuve externe
active. Il exige les versions déclarées dans `norms/oracles/manifest.toml` et
compare GNU Binutils et LLVM MC à sept encodages du corpus R1 v20260120. Il ne
prouve pas encore la couverture complète de R1, la sémantique d’exécution ni
la conformité ABI RV64ILP32.

### Machine et flottants

Les tests machine vérifient :

- écriture de x1 par addi ;
- règles de pc des branches et jumps ;
- lw/sw et extension de signe ;
- motifs exacts de 1.5 + 2.25 en binary32 et binary64 ;
- NX sur une perturbation subnormale ;
- NV pour +infini + -infini ;
- NaN-box invalide converti en NaN silencieux canonique ;
- flags sticky dans fcsr.

Seul RNE est exécuté. Les autres modes d’arrondi sont rejetés explicitement.

### Formatage flottant

luna-floatfmt sépare :

- le motif binaire exact en hexadécimal ;
- la classe IEEE ;
- le décimal court pour les valeurs finies ;
- les affichages de ±0, infinis et NaN ;
- la détection d’un NaN-box binary32 invalide.

Les tests vérifient le round-trip du décimal court pour des valeurs finies représentatives. Les payloads NaN sont garantis par le champ hexadécimal, pas par une chaîne décimale.

### Désassembleur

Les tests couvrent le format canonique en xN/fN, la symbolisation PC-relative, la représentation d’un opcode illégal en .word, les unités tronquées, le rejet explicite de C non supporté et assembleur → désassembleur → assembleur.

### Moniteur et application

Les tests du moniteur utilisent son API déterministe, pas un terminal réel. Ils vérifient :

1. assemble addi x1,x0,1, step, puis x1 ;
2. assemble fadd.s, step, puis le motif flottant et fcsr ;
3. run 3 sur une boucle et respect de la borne.

Les commandes couvertes sont help, assemble, step, run, disasm, regs, reset, memory/hex, view, edit, undo et quit dans le moniteur hôte. Le backend QEMU couvre en plus `break`, `delete`, `info break` et `continue`, avec validation UART de l’arrêt sur breakpoint permanent et de son réarmement après un pas de franchissement. L’entrée/sortie interactive complète, les couleurs, le clavier, les marques et l’édition mémoire QEMU restent à tester.

La vue mémoire utilise 16 octets par ligne, affiche les octets exacts et
remplace les caractères non imprimables par `.` dans la colonne ASCII. `edit`
effectue une lecture de sauvegarde puis une écriture via `TargetBackend`; une
erreur de plage ne modifie donc pas la mémoire. `undo` restaure au maximum les
64 dernières éditions, avec une limite de 4096 octets par opération.

Les marques sont des noms ASCII stables de 32 octets maximum. `mark name`
capture l’adresse de la vue courante, tandis que `mark name address` l’associe
à une adresse explicite. La notation `@name` est acceptée par les commandes de
navigation et de vue, et `reset` supprime les marques.

Le moniteur hôte implémente des breakpoints logiques et des watchpoints sur
lectures, écritures ou les deux. `supports_watchpoints` indique que le
backend expose des événements d’accès utilisables par le moniteur, par unité
de debug native ou par trace d’exécution. Un breakpoint arrête avant l’instruction ;
`continue` franchit celui qui se trouve au PC courant une seule fois. Un
watchpoint arrête après l’instruction et rapporte l’accès mémoire observé. La
machine expose ces accès dans `ExecutionOutcome`; un backend qui ne peut pas
les fournir doit déclarer cette capacité comme non supportée au lieu de les
inventer.

Le chargement `assemble-program` accepte un texte multi-lignes, charge les
symboles et réinitialise l’état d’exécution. `where` affiche le PC et le
symbole le plus proche, `symbols` liste la table, `stack` expose la pile
d’appels inférée pour `jal`/`jalr`, et `history` conserve au plus 4096
instructions. L’inférence de pile n’est pas un unwind ABI complet et ne doit
pas être utilisée comme preuve de validité d’une pile corrompue.

Les snapshots utilisent le format canonique versionné RVSNAP01 et les projets
RVPROJ01, tous deux en little-endian avec taille maximale de 64 MiB. La
restauration décode et vérifie l’intégralité du fichier avant de remplacer
l’état courant. L’historique d’exécution, la pile inférée et l’historique undo
sont volontairement vidés après restauration ; ils ne constituent pas encore
des éléments persistés du replay.

### Backend cible 4B

Le contrat `luna_target_api::TargetBackend` est la frontière commune entre le
moniteur et une cible. Il expose les capacités, un `TargetContext`, des accès
octet par octet, `step` et `run`, avec des résultats indépendants du transport.
`luna-machine` et le backend QEMU l’implémentent.

Le crate `luna-qemu-backend` fournit un adaptateur GDB Remote Protocol générique
sur `Read + Write` et un constructeur TCP. La première tranche couvre les
paquets `?`, `m`, `M`, `s` et `g`, les accusés de réception, l’échappement, les
checksums et les stop replies. Le layout RV64 est explicite et injectable. Le
backend tente d’abord le layout x0..x31, f0..f31, pc, puis reconnaît le layout
observé avec QEMU 11.0.2 dans cette image (x0..x31, pc) et expose alors
honnêtement des capacités F/D désactivées. `bash
scripts/test-qemu-gdb-backend.sh` valide ce chemin sur un QEMU live, avec
lecture de la RAM à `0x80000000` puis un pas.

`luna-monitor::BackendConsole<B>` utilise le même contrat pour les commandes
communes `assemble`, `assemble-program`, `step`, `run`, `continue`, `regs`,
`disasm`, `symbols`, `where`, `memory`, `view`, `edit`, `undo`, `break`,
`delete` et `info break`. Ses breakpoints sont
logiques côté moniteur : ils arrêtent avant l’appel au backend et ne modifient
pas la mémoire cible. Les commandes `watch`, `rwatch`, `awatch`, `info watch`
et `history` utilisent les `MemoryAccess` et l’historique borné retournés par
le backend ; elles sont refusées proprement lorsque ces événements ne sont
pas disponibles.

Le contrat optionnel `TargetBackend::snapshot` permet une restauration de
l’état cible complet lorsque le backend l’autorise. `luna-machine` sérialise
registres, `fcsr`, PC, compteur d’instructions et mémoire dans un format
déterministe ; QEMU retourne un diagnostic d’indisponibilité tant que le
transport ne sait pas restaurer les registres.
`luna-app --qemu-port PORT` sélectionne ce chemin et l’intègre à la boucle
interactive ; la sonde live vérifie les registres, la RAM QEMU et le pas depuis
le PC de reset. Les symboles multi-lignes et snapshots restent dans le profil
`Monitor` historique jusqu’à leur migration explicite vers le contrat backend.

Le crate `luna-isa-core` est `no_std` et produit les artefacts d’opcodes à
partir des mêmes sources R2 que `luna-isa`; il expose les encodeurs entiers,
de contrôle et `fadd.s`/`fadd.d` sans allocation pour le guest. Le crate
`luna-guest-monitor` est une première
tranche d’intégration hors
`cargo test` : il est compilé pour `riscv64gc-unknown-none-elf`, mais les
options Cargo désactivent `c` et `zca` afin de respecter le profil V1 C=off.
Le linker place l’image en RAM QEMU à partir de `0x80000000`. Le code M-mode
configure `mtvec`, `mscratch`, les registres flottants et une entrée PMP TOR
permettant l’accès U-mode à la fenêtre basse contenant le MMIO UART et la RAM.
Le trap capture les registres entiers, flottants, `fcsr`, `mstatus`, `mepc`,
`mcause` et `mtval`, puis s’arrête sur le prompt monitor.

Le linker réserve une fenêtre `.target_workspace` distincte de l’image et de
la pile du moniteur. Les commandes guest `assemble` et `assemble-program` ne
peuvent écrire que dans cette fenêtre ; le smoke test récupère son adresse
avec `_target_workspace_start` et `nm` pour éviter toute dépendance à une
adresse fixe.

Le smoke test résout `target_entry` dans l’image avec `riscv64-linux-gnu-nm`,
pose un breakpoint permanent sur le `beq`, vérifie `info break`, exécute
`continue` jusqu’à ce breakpoint, puis exécute un second `continue` pour
franchir l’instruction originale et vérifier le réarmement. Il supprime ensuite
le breakpoint et reprend deux pas temporaires. La séquence vérifie la
restauration des mots, les instructions séquentielles, `beq`/`bne`, `jal` et
`jalr` du profil actuellement émis. Il assemble enfin six lignes, dont une
branche avec labels, une instruction ignorée et `fadd.s`/`fadd.d`, dans la
fenêtre de travail réservée. Il édite quatre octets, vérifie la vue mémoire et
restaure la transaction par `undo`, écrit `.word`, `.float` et `.binary128` en
motifs exacts puis annule chaque écriture. Il vérifie ensuite `symbols`,
`disasm`, `setf`, les motifs NaN-boxés et binary64, pose un breakpoint par
label, exécute cinq pas et vérifie `x1=3`, `f3`, `f6` et `fcsr`.

Le backend QEMU limite volontairement la table à quatre breakpoints permanents
numérotés de 1 à 4. Une adresse doit être un mot aligné de la fenêtre RAM cible.
Un breakpoint permanent conserve son instruction originale et est réinstallé
après le franchissement logiciel d’une instruction. Une collision entre un
breakpoint permanent et le breakpoint temporaire du pas-à-pas est refusée.

## Pyramide actuelle

| Niveau | État | Commentaire |
|---|---|---|
| Unitaire | Présent | ABI, mémoire, lexer, expressions, ISA, flottants, machine et contrat de cible. |
| Composant | Partiel | Assembleur, désassembleur et moniteur testés par API. |
| Intégration interne | Présent | Round-trips et chaîne monitor/machine. |
| Différentiel externe | Partiel | GNU/LLVM sont branchés sur sept encodages R1 ; Sail, Spike et SoftFloat restent à intégrer. |
| Génératif/fuzzing | Absent | Aucun budget de fuzzing installé. |
| E2E terminal | Partiel | Smoke test UART/QEMU et session TCP GDB RSP automatisés ; protocole interactif complet encore absent. |
| Multi-plateforme | Absent | Pas encore de matrice Linux/macOS/Windows et x86_64/arm64. |

## Limites actuelles

Les tests ne prouvent pas encore :

- la conformité complète RV64I/M/F/D ;
- binary16, binary128, bfloat16, conversions ou autres opérations flottantes ;
- tous les modes d’arrondi IEEE ;
- toutes les règles de payload NaN ;
- les oracles indépendants GNU/LLVM/Sail/Spike/SoftFloat ;
- C, A, V et les CSR/privilèges complets ;
- breakpoints permanents, watchpoints, snapshots, édition mémoire et annulation ;
- la reproductibilité cross-platform ;
- les performances, quotas, fuzzing et corpus de non-régression à grande échelle.

## Prochains tests prioritaires

1. Différentiel GNU/LLVM sur fadd.s, fadd.d et les formes entières.
2. Oracle indépendant des résultats et flags flottants.
3. Corpus IEEE binary32/binary64 : limites, overflow, underflow, exact/inexact, ±0, infinis, qNaN/sNaN et payloads.
4. Test E2E stdin/replay du moniteur.
5. Fuzz targets lexer, parser, désassembleur et commandes.

Un test est considéré comme présent uniquement s’il est exécuté par `cargo test --workspace` dans la validation courante. Les éléments de TEST_PLAN.md non reflétés ici sont des objectifs, pas des garanties actuelles.
