# MiniBASIC-RV — suivi de parité Turbo BASIC XL 1.5

## Objet

Ce document est le registre de suivi de la parité fonctionnelle entre
MiniBASIC-RV et l’expérience de programmation de Turbo BASIC XL 1.5 (TBXL).
Il sert à distinguer précisément :

1. les fonctions présentes et prouvées dans le programme assembleur cible ;
2. les fonctions présentes mais limitées par une décision MiniBASIC ;
3. les fonctions planifiées ;
4. les fonctions volontairement hors périmètre.

« Parité » signifie ici une proximité d’usage et de structures de langage :
mode direct, lignes numérotées, édition immédiate, LIST/RUN, contrôle de flot,
expressions, données, chaînes, tableaux et diagnostics. Elle ne signifie ni
compatibilité Atari, ni compatibilité binaire, ni compatibilité des tokens,
des nombres historiques, des périphériques, du DOS ou des graphismes.

État de l’audit au 3 août 2026 : **89 scripts QEMU assembleur MiniBASIC
recensés**. Le nombre
est contrôlable par :

```text
find scripts -maxdepth 1 -type f -name 'test-guest-runtime-asm-repl*.sh' | wc -l
```

La preuve retenue est une exécution du payload assembleur dans la machine RV64
du moniteur. L’interpréteur Rust résident peut servir à développer ou comparer
des idées, mais il n’est pas l’oracle des résultats target-side.

## Sources et méthode

- [Turbo-BASIC XL — Expanded Documentation](https://ftp.pigwa.net/stuff/collections/Atari%20books/Turbo-BASIC%20XL%20-%20Expanded%20Documentation.pdf)
  : référence d’expérience utilisateur et de langage historique, non normative
  pour RVMonitor ;
- [`docs/BASIC_TBXL_NOTES.md`](BASIC_TBXL_NOTES.md) : observations issues de
  l’étude du désassemblage et des artefacts TBXL 1.5 ;
- [désassemblage `dmsc/turbo-dis`](https://github.com/dmsc/turbo-dis) :
  référence d’architecture interne, non dépendance de construction ;
- [`docs/BASIC_LANGUAGE.md`](BASIC_LANGUAGE.md) : contrat MiniBASIC effectif ;
- [`docs/BASIC_TEST_PLAN.md`](BASIC_TEST_PLAN.md) : stratégie de vérification ;
- les documents ASM-One sous `docs/dontcommit/` : ergonomie du moniteur
  uniquement, sans autorité sur le langage BASIC.

Les comparaisons historiques sont faites par familles de fonctions et non par
copie des adresses 6502, des tables de tokens ou de l’OS Atari. Les tests
différentiels éventuels vérifient des résultats, mais l’exécution doit rester
entièrement dans le guest.

## Légende des états

| État | Signification |
|---|---|
| **VERT** | Fonction disponible dans le payload assembleur et couverte par au moins un test QEMU ciblé. |
| **PARTIEL** | Fonction réellement disponible, mais avec une limite syntaxique, structurelle ou sémantique explicitement documentée. |
| **PLANIFIÉ** | Extension compatible avec la trajectoire, non acceptée dans l’état courant. |
| **DIFFÉRÉ** | Décision ou chantier conservé, mais non prioritaire pour la parité actuelle. |
| **REJETÉ** | Fonction hors périmètre MiniBASIC-RV ; elle ne doit pas être comptée comme dette cachée. |

## Matrice de parité fonctionnelle

| Famille TBXL / MiniBASIC | Contrat MiniBASIC-RV | État | Preuve actuelle |
|---|---|---:|---|
| Invite directe | `READY>`, commande immédiate et retour après erreur | VERT | `test-guest-runtime-asm-repl-direct.sh` |
| Programme à lignes | insertion, remplacement, suppression par numéro et tri croissant | VERT | `two-lines.sh`, `four-lines.sh` |
| `NEW`, `LIST`, `RUN` | commandes target-side sur le magasin de lignes | VERT | `test-guest-runtime-asm-repl.sh`, `trace.sh` |
| `TRACE ON/OFF` | affiche `[numéro]` avant l’exécution d’une ligne | VERT | `test-guest-runtime-asm-repl-trace.sh` |
| `PRINT` / `?` | expressions, chaînes, mélange et éléments multiples | VERT | `print-mixed.sh`, `question.sh`, `string.sh` |
| `INPUT` | lecture et conversion target-side de valeurs numériques et chaînes | VERT | `input.sh`, `long-input.sh`, `long-string-input.sh` |
| Arithmétique | `+`, `-`, `*`, `/` en binary64 par `fadd.d`, `fsub.d`, `fmul.d`, `fdiv.d` | VERT | `precedence.sh`, `mul.sh`, `expression-div.sh` |
| Comparaisons | `=`, `<>`, `<`, `<=`, `>`, `>=`, résultat `0.0` ou `1.0` | VERT | `if.sh`, `if-false.sh`, `precedence.sh` |
| Fonctions numériques | `ABS`, `SGN`, `INT`, `TRUNC`, `FRAC`, `MOD`, appels target-side et imbrication documentée | VERT | `numeric-rounding.sh`, `numeric-rounding-error.sh`, `numeric-functions.sh` |
| Aléatoire | `RND` et `RND()`, LCG target-side reproductible, graine à 1 | VERT | `rnd.sh`, `rnd-error.sh` |
| Fonctions de chaînes | `LEN`, `PRINT LEFT$`, `PRINT RIGHT$` et `PRINT MID$` avec buffer target-side, sources scalaires, tableaux chaîne ou littéraux | PARTIEL | `string-len*.sh`, `string-left*.sh`, `string-right*.sh`, `string-mid*.sh`, `string-slice-array-source.sh`, `string-slice-literal-source*.sh`; expressions chaîne différées |
| Affectation chaîne par fonction | `LET destination$=LEFT$`, `RIGHT$` ou `MID$` avec source scalaire, tableau chaîne ou littéral et destination scalaire ou tableau, copie target-side bornée et sûre en cas de recouvrement | PARTIEL | `string-assignment.sh`, `string-slice-assignment*.sh`, `string-slice-array-source.sh`, `string-slice-array-destination*.sh`, `string-slice-literal-source*.sh`; expressions chaîne générales différées |
| Variables numériques | variables courtes historiques et identifiants ASCII de 2 à 16 caractères | VERT | `scalars.sh`, `long-names.sh`, `keyword-vars.sh` |
| Chaînes | littéraux, variables courtes/longues, affectation, affichage et entrée | VERT | `string-var.sh`, `long-string.sh` |
| Tableaux numériques | 1D/2D, noms courts/longs, index calculés et contrôle des bornes | VERT | `array*.sh`, `long-numeric-array*.sh` |
| Tableaux de chaînes | 1D/2D, noms courts/longs, stockage et contrôle des bornes | VERT | `string-array*.sh`, `long-string-array*.sh` |
| `IF ... THEN numéro` | branchement direct par numéro de ligne | VERT | `if.sh`, `if-false.sh` |
| `GOTO` | transfert par numéro de ligne et erreur de cible absente | VERT | `goto.sh`, `goto-30.sh` |
| `FOR/NEXT` | huit niveaux, `STEP` positif/négatif et noms longs | VERT | `for.sh`, `for-nested.sh`, `for-step.sh` |
| `GOSUB/RETURN` | huit retours, cible et retour target-side | VERT | `gosub.sh` |
| `POP` | retire le cadre le plus récent de la pile unifiée | VERT | `test-guest-runtime-asm-repl-pop.sh` |
| `EXIT` | sortie typée de `FOR`, `WHILE`, `REPEAT` ou `DO` | VERT | `test-guest-runtime-asm-repl-exit.sh` |
| `WHILE/WEND` | boucles imbriquées jusqu’à huit niveaux | VERT | `while.sh`, `while-error.sh` |
| `REPEAT/UNTIL` | test terminal et boucles imbriquées jusqu’à huit niveaux | VERT | `repeat.sh`, `repeat-error.sh` |
| `DO/LOOP` | boucle inconditionnelle et sortie par `EXIT`/`POP` | VERT | `test-guest-runtime-asm-repl-do-loop.sh` |
| `IF ... ELSE ... ENDIF` | bloc structuré non imbriqué, terminateurs sur lignes dédiées | PARTIEL | `if-block.sh`, `if-block-error.sh` |
| `ON ... GOTO/GOSUB` | sélection entière 1-based sur une liste de lignes | VERT | `on.sh`, `on-error.sh` |
| `DATA/READ/RESTORE` | données numériques et chaînes, curseur target-side | VERT | `data-read.sh`, `restore.sh` |
| `REM` | commentaire BASIC jusqu’à la fin de la ligne | VERT | `rem.sh` |
| interruption | Ctrl-C consommé par le guest pendant `RUN` | VERT | `break.sh` |
| erreurs récupérables | code stable, ligne lorsque connue, retour à `READY>` | VERT | scripts `*-error.sh` |

Les motifs `*.sh` ci-dessus sont des familles ; les scripts exacts sont dans
`scripts/` et doivent être conservés comme preuves exécutables.

## Divergences sémantiques assumées

### Nombres

TBXL historique repose sur l’environnement Atari et ses conventions numériques.
MiniBASIC-RV utilise IEEE 754 binary64 dans le guest, avec l’extension RISC-V D.
Les résultats ne sont donc pas comparés bit à bit à une sortie Atari. Le format
d’affichage V1 est fixe à six décimales ; `INF`, `-INF` et `NAN` ont des formes
stables documentées dans `BASIC_LANGUAGE.md`.

`RND` est volontairement reproductible : LCG 32 bits, multiplicateur `1664525`,
incrément `1013904223`, graine `1` au chargement et après `NEW`. Cela facilite
l’audit et diffère d’un générateur TBXL destiné à un usage interactif.

### Structures de contrôle

Les structures utilisent des zones statiques target-side et une pile unifiée
portant au minimum le type de cadre et la ligne source. Cette représentation
permet de rendre `POP` et `EXIT` typés ; elle ne reproduit pas les adresses ou
les tables internes de TBXL.

La forme `IF expression THEN` sans numéro ouvre un bloc MiniBASIC. En V1, les
blocs sont non imbriqués et `ELSE`/`ENDIF` doivent être les premières
instructions de lignes dédiées. Cette contrainte est **PARTIELLE** et doit
rester visible dans le tutoriel.

Les recherches de terminateurs sont bornées au magasin de lignes et portent
sur le premier statement de chaque ligne. Les structures ouvertes et fermées
dans une même ligne séparée par `:` ne doivent pas être considérées comme
équivalentes aux formes TBXL plus permissives.

### Identifiants, chaînes et tableaux

MiniBASIC conserve la lisibilité des variables BASIC traditionnelles mais
autorise des identifiants ASCII jusqu’à 16 caractères, ainsi que des variables
chaînes et des tableaux numériques ou de chaînes 1D/2D. Ces capacités sont des
extensions RV target-side ; elles ne prétendent pas reproduire les pointeurs,
tokens ou layouts Atari.

## Fonctions conservées, modernisées et rejetées

### Conservées comme expérience

- invite immédiate et programme à lignes numérotées ;
- `LIST`, `RUN`, `TRACE`, `PRINT`, `INPUT` ;
- contrôle de flot par lignes, boucles, sous-programmes et données ;
- édition par remplacement/suppression et diagnostic lisible ;
- programmes pédagogiques interactifs, notamment Hammurabi-RV.

### Modernisées pour RVMonitor

- exécution entièrement target-side dans RV64, sans interprétation hôte ;
- binary64 matériel et observation de `fcsr`, `frm`, `fflags` au débogueur ;
- bornes explicites, stockage statique, erreurs récupérables et interruption ;
- noms longs, chaînes et tableaux inspectables ;
- assemblage du payload par le moniteur et chargement relogeable ;
- reproductibilité QEMU et tests automatisés indépendants du chemin hôte.

### Rejetées du périmètre MiniBASIC

Compatibilité Atari, BCD historique, DOS, fichiers Atari, graphismes, sons,
périphériques Atari, tokens binaires TBXL, `PEEK/POKE` Atari et reproduction
des contraintes mémoire 8-bit. Ces éléments ne sont pas des tâches de parité
à cacher dans le backlog.

## Écarts et roadmap

Priorité suivante, après stabilisation du socle actuel :

1. produire une démonstration Hammurabi complète et auditée, incluant les
   variables longues et les tableaux visibles dans le moniteur ;
2. finaliser le tutoriel progressif et son scénario de démonstration ;
3. étendre prudemment les fonctions chaîne aux expressions générales ; `LEN`,
   `PRINT LEFT$`, `PRINT RIGHT$`,
   `PRINT MID$` et les affectations scalaires `LEFT$`/`RIGHT$`/`MID$` sont
   désormais implémentés avec les limites documentées ;
4. traiter les limites documentées des blocs structurés et des expressions
   d’index uniquement si une compatibilité supplémentaire est prioritaire ;
5. conserver la compilation native BASIC comme chantier séparé et différé,
   conformément à [`docs/BASIC_COMPILER_ROADMAP.md`](BASIC_COMPILER_ROADMAP.md).

Ne pas annoncer une parité complète TBXL avant d’avoir soit implémenté, soit
explicitement rejeté dans une décision versionnée chaque famille jugée
nécessaire au produit.

## Reproduction de l’audit

Construction du moniteur et des tests :

```text
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf
bash scripts/test-guest-runtime-asm-repl-hammurabi.sh
bash scripts/test-guest-runtime-asm-repl-rnd.sh
bash scripts/test-guest-runtime-asm-repl-if-block.sh
```

Pour la matrice complète, exécuter les scripts
`test-guest-runtime-asm-repl*.sh` individuellement ou via le harnais de test
documenté dans `docs/BASIC_TEST_PLAN.md`. Un arrêt `trap: breakpoint` à la fin
d’un test est le comportement de terminaison attendu du payload, pas une
sortie BASIC préenregistrée.

## Audit de couverture

- **Décisions de parité figées :** mode direct, lignes, contrôle de flot,
  chaînes, tableaux, binary64 target-side, interruption et périmètre Atari
  rejeté ;
- **Preuve automatisée :** 87 tests QEMU assembleur recensés au moment de cet
  audit, dont six scénarios dédiés aux affectations chaîne et à leurs sources
  ou destinations tableau ;
- **Écart important restant :** `IF/ELSE/ENDIF` est non imbriqué ; certaines
  fonctions historiques TBXL ne sont pas encore retenues ; les expressions
  chaîne générales et les affectations sur tableaux ou littéraux restent
  partielles ;
- **Conclusion :** la parité d’expérience du démonstrateur est solide, mais la
  parité de langage TBXL 1.5 n’est pas complète et ne doit pas être présentée
  comme telle.

## Journal de mise à jour

| Date | Évolution | Preuve | Impact de parité |
|---|---|---|---|
| 2026-08-03 | Ajout des affectations scalaires `LEFT$`, `RIGHT$` et `MID$` dans le payload assembleur | quatre tests QEMU target-side : cas nominaux, auto-affectation et erreurs de forme/bornes | les fonctions de découpe sont maintenant réutilisables par les programmes, tout en restant PARTIEL pour les tableaux et expressions chaîne |
| 2026-08-03 | Résolution de sources scalaires et d’éléments de tableaux chaîne dans les fonctions de découpe | `test-guest-runtime-asm-repl-string-slice-array-source.sh` sous QEMU, incluant tableau court et long | les fonctions peuvent consommer les données de tableaux sans délégation hôte |
| 2026-08-03 | Écriture dans des destinations scalaires et éléments de tableaux chaîne | `test-guest-runtime-asm-repl-string-slice-array-destination*.sh` sous QEMU, cas nominal et erreurs | les trois fonctions de découpe peuvent alimenter les tableaux target-side, sans écriture partielle lors d’une erreur |
| 2026-08-03 | Littéraux ASCII comme sources des fonctions de découpe | `test-guest-runtime-asm-repl-string-slice-literal-source*.sh` sous QEMU, cas nominal et erreurs de forme/bornes | les littéraux sont copiés dans un buffer cible distinct ; aucune évaluation de chaîne n’est déléguée à l’hôte |
| 2026-08-03 | Inventaire recalculé | `find scripts -maxdepth 1 -type f -name 'test-guest-runtime-asm-repl*.sh' \| wc -l` → `89` | le nombre annoncé dans ce document est reproductible et non maintenu manuellement |

Les affectations ne constituent pas encore une parité chaîne complète : le RHS
doit être exactement une forme `LEFT$`, `RIGHT$` ou `MID$` avec littéral ASCII,
variable scalaire ou élément de tableau chaîne source, et la destination doit
être une variable scalaire ou un élément de tableau chaîne ; les expressions
chaîne générales sont explicitement hors de cette tranche. Toute extension doit ajouter
un test QEMU target-side et une entrée à cette matrice avant d’être marquée
VERT.
