# État des tests RVMonitor

Ce document décrit le périmètre effectivement couvert par les tests présents dans le dépôt sur la branche courante. Il complète TEST_PLAN.md, qui décrit la stratégie cible et contient des fonctions encore non implémentées.

## Commandes de validation

Validation locale complète :

    cargo fmt --all
    cargo test --workspace
    cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf
    bash scripts/test-guest-monitor.sh
    git diff --check

La suite actuelle exécute 62 tests unitaires/intégration répartis dans les crates. Les doc-tests compilent mais ne contiennent actuellement aucun cas.

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
| luna-asm-lexer | 2 | Labels, ponctuation, commentaires, chaînes et positions d’erreur. |
| luna-assembler | 16 | AST, alias ABI, expressions, symboles, directives, alignement, fadd.s et fadd.d. |
| luna-isa | 6 | Tables générées depuis R2, encodage/décodage entier et flottant. |
| luna-machine | 11 | Exécution entière, branches, mémoire, tables de pointeurs ILP32, fadd.s, fadd.d, NaN-boxing, flags et contrat backend. |
| luna-disassembler | 7 | Format canonique, symboles, opcodes illégaux, C rejeté et round-trip. |
| luna-floatfmt | 3 | Bits hex exacts, décimal court, classes IEEE et NaN-box invalide. |
| luna-monitor | 9 | assemble → step → regs, affichage flottant, run borné, vues mémoire hex/ASCII, édition/undo et marques QuickJump. |
| luna-target-api | 4 | Contexte de trap, capacités explicites RV64 bare-metal, codes `mcause`, contrat de layout et types du backend commun. |
| luna-guest-monitor | 0 | Image bare-metal, boot QEMU, PMP, transition M→U, trap `ebreak` et boucle UART; vérifié par smoke test QEMU. |
| luna-app | 0 | Compilation du binaire et démonstration ; pas encore d’E2E terminal. |
| luna-diag | 0 | Types utilisés par les autres crates ; pas de test dédié. |

## Périmètre détaillé

### ABI et mémoire

Les tests ABI verrouillent les frontières de pointeurs RV64ILP32 déjà modélisées, notamment la représentation sign-étendue dans un registre RV64. Ils ne valident pas encore une convention d’appel complète.

Les tests mémoire couvrent les transactions de bytes et le little-endian 32 bits. MMIO, quotas, pages, alignement généralisé, snapshots et mémoire distante ne sont pas couverts.

### Lexer, parser et assembleur

Le lexer et le parser testent les labels avec ou sans instruction, les registres entiers/flottants, les alias ABI, les opérandes imm(registre), les expressions et les diagnostics syntaxiques.

Les formes assembleur testées sont :

    addi, add, sub, lui, lw, sw, beq, bne, jal, jalr
    fadd.s, fadd.d
    .byte, .half, .word, .dword, .ascii, .asciz, .align

Les expressions couvrent la précédence, les bases décimale/hexadécimale/binaire, les opérateurs unaires, les décalages, les symboles en avant et les débordements. Un test vérifie qu’un offset numérique de branche reste un offset après pc = 0.

### ISA et encodages

Le registre d’opcodes est produit depuis l’extrait R2 épinglé. Les tests vérifient la présence des données générées et les round-trips des formes entières, de fadd.s et de fadd.d.

Cette vérification ne remplace pas encore une comparaison indépendante avec GNU, LLVM, Sail, Spike ou SoftFloat.

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

### Backend cible 4B

Le contrat `luna_target_api::TargetBackend` est la frontière commune entre le
moniteur et une cible. Il expose les capacités, un `TargetContext`, des accès
octet par octet, `step` et `run`, avec des résultats indépendants du transport.
`luna-machine` l’implémente actuellement ; le backend QEMU bare-metal conserve
son protocole UART local et sera adapté à ce contrat dans une tranche
ultérieure.

Le crate `luna-guest-monitor` est une première tranche d’intégration hors
`cargo test` : il est compilé pour `riscv64gc-unknown-none-elf`, mais les
options Cargo désactivent `c` et `zca` afin de respecter le profil V1 C=off.
Le linker place l’image en RAM QEMU à partir de `0x80000000`. Le code M-mode
configure `mtvec`, `mscratch`, les registres flottants et une entrée PMP TOR
permettant l’accès U-mode à la fenêtre basse contenant le MMIO UART et la RAM.
Le trap capture les registres entiers, flottants, `fcsr`, `mstatus`, `mepc`,
`mcause` et `mtval`, puis s’arrête sur le prompt monitor.

Le smoke test résout `target_entry` dans l’image avec `riscv64-linux-gnu-nm`,
pose un breakpoint permanent sur le `beq`, vérifie `info break`, exécute
`continue` jusqu’à ce breakpoint, puis exécute un second `continue` pour
franchir l’instruction originale et vérifier le réarmement. Il supprime ensuite
le breakpoint et reprend deux pas temporaires. La séquence vérifie la
restauration des mots, les instructions séquentielles, `beq`/`bne`, `jal` et
`jalr` du profil actuellement émis.

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
| Différentiel externe | Absent | GNU, LLVM, Sail, Spike et SoftFloat ne sont pas branchés dans CI. |
| Génératif/fuzzing | Absent | Aucun budget de fuzzing installé. |
| E2E terminal | Partiel | Smoke test UART/QEMU automatisé ; protocole interactif complet encore absent. |
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
