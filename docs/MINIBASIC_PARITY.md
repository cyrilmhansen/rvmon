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

État de l’audit au 4 août 2026 : **125 scripts QEMU assembleur MiniBASIC
recensés**. Les scénarios récents fournissent notamment la preuve nominale et
d’erreur de `ATN` après exécution QEMU verte. Ce nombre
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

Le runtime privé de TBXL est étudié comme une référence d'architecture à part
entière : tables de syntaxe, piles d'opérandes et d'exécution, évaluateur,
boucle des lignes, handlers et reprise sur erreur. Les comparaisons historiques
sont faites par familles de fonctions et structures, sans copie des adresses
6502, du format binaire des tokens ou des appels spécifiques à l'OS Atari. Les tests
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
| Programme à lignes | insertion, remplacement, suppression par numéro/plage, renumérotation bornée, alias de cibles et tri croissant | PARTIEL | `two-lines.sh`, `four-lines.sh`, `del.sh`, `renum.sh`, `renum-control.sh` |
| `NEW`, `LIST`, `RUN`, `RENUM` | commandes target-side sur le magasin de lignes | PARTIEL | `test-guest-runtime-asm-repl.sh`, `trace.sh`, `del.sh`, `renum.sh`, `renum-error.sh` |
| `TRACE ON/OFF` | affiche `[numéro]` avant l’exécution d’une ligne | VERT | `test-guest-runtime-asm-repl-trace.sh` |
| `PRINT` / `?` | expressions, chaînes, mélange et éléments multiples | VERT | `print-mixed.sh`, `question.sh`, `string.sh` |
| `INPUT` | lecture et conversion target-side de valeurs numériques et chaînes | VERT | `input.sh`, `long-input.sh`, `long-string-input.sh` |
| Arithmétique | `+`, `-`, `*`, `/` en binary64 par `fadd.d`, `fsub.d`, `fmul.d`, `fdiv.d` | VERT | `precedence.sh`, `mul.sh`, `expression-div.sh` |
| Comparaisons | `=`, `<>`, `<`, `<=`, `>`, `>=`, résultat `0.0` ou `1.0` | VERT | `if.sh`, `if-false.sh`, `precedence.sh` |
| Comparaisons chaîne dans `IF` | six opérateurs, ordre lexicographique ASCII target-side, littéraux/variables/fonctions chaîne composées | VERT | `test-guest-runtime-asm-repl-string-compare.sh` sous QEMU |
| Fonctions numériques | `ABS`, `SGN`, `INT`, `TRUNC`, `FRAC`, `MOD`, `SQR`, `SIN`, `COS`, `TAN`, `LOG`, `EXP`, appels target-side et imbrication documentée | VERT | `numeric-rounding.sh`, `numeric-rounding-error.sh`, `numeric-functions.sh`, `sqr.sh`, `sqr-error.sh`, `trig.sh`, `tan.sh`, `tan-error.sh`, `log-exp.sh`, `log-exp-error.sh` |
| `ATN` | Fonction en radians, résultat binary64 target-side et imbrication bornée | VERT | `atn.sh` et `atn-error.sh` ; cas nominaux, appel imbriqué et diagnostic QEMU |
| Conversion caractère | `ASC(string-source)` numérique et `CHR$(expression)` chaîne, exécutés dans le guest avec bornes explicites ; `ASC` accepte l'expression chaîne commune, y compris concaténation et parenthèses imbriquées | PARTIEL | `test-guest-runtime-asm-repl-string-asc-concat*.sh`, `test-guest-runtime-asm-repl-string-concat.sh`, `test-guest-runtime-asm-repl-string-concat-error.sh`; `CHR$` est disponible en affectation, concaténation et `PRINT`, tandis que les conversions implicites restent différées |
| Lecture clavier non bloquante | `INKEY$()` retourne une chaîne vide ou un octet depuis `poll-char`, entièrement dans le guest | PARTIEL | `test-guest-runtime-asm-repl-string-inkey.sh` sous QEMU ; la file vide est couverte, le cas d'un octet disponible reste à compléter |
| Conversion chaîne → nombre | `VAL(string-source)` consomme le résultat du résolveur d'expression chaîne commun dans le guest et refuse les caractères non consommés | PARTIEL | `test-guest-runtime-asm-repl-string-val-concat*.sh`; exposants et formats historiques restent hors V1 |
| Recherche chaîne | `INSTR(haystack,needle)` target-side, résultat 1-based, `0` absent et `1` pour aiguille vide ; les deux opérandes peuvent être des expressions chaîne communes, dont une découpe imbriquée | PARTIEL | `test-guest-runtime-asm-repl-string-instr-expression.sh`, `test-guest-runtime-asm-repl-string-concat.sh`, `test-guest-runtime-asm-repl-string-concat-error.sh`; les formats historiques restent hors contrat |
| Conversion nombre → chaîne | `STR$(expression)` target-side, format fixe à six décimales réutilisable en affectation, concaténation et `PRINT` | PARTIEL | `test-guest-runtime-asm-repl-string-concat.sh`, `test-guest-runtime-asm-repl-string-concat-error.sh`; formats historiques différés |
| Conversion hexadécimale | `HEX$(expression)` target-side, chiffres ASCII majuscules sans zéros de tête, borne `0..0xffffffff` | PARTIEL | `test-guest-runtime-asm-repl-string-hex.sh` sous QEMU ; `PRINT`, concaténation, affectation et erreurs de domaine sont couverts |
| Conversion hexadécimale → nombre | `DEC(string-source)` target-side, 1..8 chiffres ASCII hexadécimaux, casse mixte, résultat binary64 et rejet des formes invalides | PARTIEL | `test-guest-runtime-asm-repl-string-dec.sh` sous QEMU ; littéraux, valeurs composées et composition avec `HEX$` sont couverts |
| Aléatoire | `RND` et `RND()`, LCG target-side reproductible, graine à 1 | VERT | `rnd.sh`, `rnd-error.sh` |
| Fonctions de chaînes | `LEN`, `PRINT LEFT$`, `PRINT RIGHT$` et `PRINT MID$` avec buffer target-side ; les découpes d’affichage et les affectations de découpe acceptent l’expression chaîne commune dans les limites documentées | PARTIEL | `string-len*.sh`, `test-guest-runtime-asm-repl-string-len-concat.sh`, `test-guest-runtime-asm-repl-string-slice-expression.sh`, `test-guest-runtime-asm-repl-string-nested-expression.sh`, `string-left*.sh`, `string-right*.sh`, `string-mid*.sh`, `string-slice-assignment*.sh`, `string-slice-array-source.sh`, `string-slice-literal-source*.sh` |
| Affectation chaîne par fonction | `LET destination$=LEFT$`, `RIGHT$` ou `MID$` avec source scalaire, tableau chaîne, littéral ou expression composée et destination scalaire ou tableau, copie target-side bornée et sûre en cas de recouvrement | PARTIEL | `string-assignment.sh`, `test-guest-runtime-asm-repl-string-slice-expression.sh`, `string-slice-assignment*.sh`, `string-slice-array-source.sh`, `string-slice-array-destination*.sh`, `string-slice-literal-source*.sh`; fonctions et imbrications au-delà des cadres documentés restent différées |
| Concaténation chaîne en affectation | affectation de termes littéraux, variables, éléments de tableaux, `LEFT$`/`RIGHT$`/`MID$` ou `CHR$` avec `+`, buffer target-side de 120 octets et rejet des dépassements | PARTIEL | `test-guest-runtime-asm-repl-string-concat.sh`, `test-guest-runtime-asm-repl-string-concat-error.sh`; conversions numériques et autres opérateurs chaîne différés |
| Concaténation chaîne dans `PRINT` | `PRINT` de littéraux, variables, découpes, `CHR$` et `STR$` avec `+`, évaluation complète dans le guest et sortie sans résultat préenregistré | VERT | `test-guest-runtime-asm-repl-string-print-concat.sh` sous QEMU ; `LEFT$`, `RIGHT$`, `MID$`, `CHR$`, `STR$`, littéraux et variables sont couverts |
| Programme étalon Hammurabi | Programme à lignes numérotées, variables longues, calculs, entrées, branches, boucles et sortie interactive entièrement target-side | VERT | `test-guest-runtime-asm-repl-hammurabi.sh` sous QEMU ; saisie hors ordre, `TRACE ON`, quinze entrées, sortie finale et breakpoint vérifiés |
| Variables numériques | variables courtes historiques et identifiants ASCII de 2 à 16 caractères | VERT | `scalars.sh`, `long-names.sh`, `keyword-vars.sh` |
| Chaînes | littéraux, variables courtes/longues, affectation, affichage et entrée | VERT | `string-var.sh`, `long-string.sh` |
| Tableaux numériques | 1D/2D, noms courts/longs, index calculés et contrôle des bornes | VERT | `array*.sh`, `long-numeric-array*.sh` |
| Tableaux de chaînes | 1D/2D, noms courts/longs, stockage et contrôle des bornes | VERT | `string-array*.sh`, `long-string-array*.sh` |
| `IF ... THEN numéro` | branchement direct par numéro de ligne | VERT | `if.sh`, `if-false.sh` |
| `GOTO` | transfert par numéro de ligne et erreur de cible absente | VERT | `goto.sh`, `goto-30.sh` |
| `FOR/NEXT` | huit niveaux, `STEP` positif/négatif et noms longs | VERT | `for.sh`, `for-nested.sh`, `for-step.sh`, `for-y.sh`; profondeur temporaire target-side protégée pendant l'analyse de `TO/STEP` |
| `GOSUB/RETURN` | huit retours, cible et retour target-side | VERT | `gosub.sh` |
| `POP` | retire le cadre le plus récent de la pile unifiée | VERT | `test-guest-runtime-asm-repl-pop.sh` |
| `EXIT` | sortie typée de `FOR`, `WHILE`, `REPEAT` ou `DO` | VERT | `test-guest-runtime-asm-repl-exit.sh` |
| `WHILE/WEND` | boucles imbriquées jusqu’à huit niveaux | VERT | `while.sh`, `while-error.sh` |
| `REPEAT/UNTIL` | test terminal et boucles imbriquées jusqu’à huit niveaux | VERT | `repeat.sh`, `repeat-error.sh` |
| `DO/LOOP` | boucle inconditionnelle et sortie par `EXIT`/`POP` | VERT | `test-guest-runtime-asm-repl-do-loop.sh` |
| `IF ... ELSE ... ENDIF` | blocs structurés imbriqués jusqu’à huit niveaux, terminateurs sur lignes dédiées | VERT | `if-block.sh`, `if-block-nested.sh`, `if-block-error.sh` |
| `ON ... GOTO/GOSUB` | sélection entière 1-based sur une liste de lignes | VERT | `on.sh`, `on-error.sh` |
| `DATA/READ/RESTORE` | données numériques et chaînes, curseur target-side | VERT | `data-read.sh`, `restore.sh` |
| `REM` | commentaire BASIC jusqu’à la fin de la ligne | VERT | `rem.sh` |
| interruption | Ctrl-C consommé par le guest pendant `RUN` | VERT | `break.sh` |
| erreurs récupérables | code stable, ligne lorsque connue, retour à `READY>` | VERT | scripts `*-error.sh` |

Les motifs `*.sh` ci-dessus sont des familles ; les scripts exacts sont dans
`scripts/` et doivent être conservés comme preuves exécutables.

## Registre de couverture TBXL

Le tableau suivant est le point d’entrée de la comparaison. Il ne confond pas
une fonction historiquement connue avec une exigence MiniBASIC : une capacité
peut être **conservée**, **modernisée**, **partielle**, **planifiée** ou
**rejetée**. Toute promotion vers **VERT** doit être accompagnée d’une preuve
target-side ; toute décision de périmètre doit être reflétée dans
`BASIC_LANGUAGE.md` et dans un test négatif si la syntaxe est explicitement
refusée.

| Surface observée dans TBXL 1.5 | Décision MiniBASIC-RV | État au 2026-08-03 | Écart vérifiable |
|---|---|---:|---|
| Mode direct et invite | Conservée sous `READY>` | VERT | invite et commandes exécutées dans le guest |
| Programme à lignes, insertion, remplacement, suppression | Conservée | VERT | limites de numéros et de capacité RV explicites |
| `LIST`, `RUN`, `TRACE` | Conservée, avec diagnostics structurés | VERT | pas de compatibilité d’écran ou de tokens Atari |
| Expressions numériques | Modernisée vers binary64/RISC-V D ; littéraux décimaux, espaces, signes unaires devant littéraux, parenthèses simples et précédence | VERT | `test-guest-runtime-asm-repl-unary-paren.sh`, `precedence.sh`, `expression-tokens.sh` ; le marqueur target-side `0x82062720=1` prouve le chemin tokenisé ; pas de BCD ni de résultats numériques Atari bit à bit |
| `IF`, `GOTO`, `FOR/NEXT`, `WHILE/WEND`, `REPEAT/UNTIL`, `DO/LOOP` | Contrôle de flot conservé et borné | VERT | blocs `IF` avec `ELSE`/`ENDIF` sur lignes dédiées |
| `GOSUB/RETURN`, `POP`, `EXIT`, `ON GOTO/GOSUB` | Sous-programmes et sorties structurées conservés | VERT | piles statiques de huit niveaux, pas de layout TBXL |
| `DATA/READ/RESTORE` | Conservée dans une représentation target-side | VERT | types et stockage propres à MiniBASIC |
| Variables longues, chaînes et tableaux | Extension RV voulue, non simple compatibilité TBXL | VERT/PARTIEL | capacités fixes, 1D/2D, expressions chaîne encore bornées |
| `LEN`, `LEFT$`, `RIGHT$`, `MID$` | Conservées avec sources/destinations RV étendues et composition par expressions chaîne | PARTIEL | `LEN`, les découpes d’affichage et les affectations composées consomment le résolveur commun ; les cadres, buffers et imbrications restent bornés |
| `ASC`, `CHR$`, `VAL`, `INSTR`, `STR$` | Compatibilité d’usage modernisée | PARTIEL | bornes, formats et cas historiques non promis |
| `ABS`, `SGN`, `INT`, `TRUNC`, `FRAC`, `MOD`, `SQR`, `SIN`, `COS`, `TAN`, `LOG`, `EXP`, `RND` | Sous-ensemble numérique explicite | VERT/PARTIEL | `SQR` utilise `fsqrt.d`, les fonctions transcendantes utilisent réduction et polynômes target-side ; `LOG/EXP` sont bornées et leur approximation fixe peut perdre une unité au dernier chiffre affiché |
| `ATN` | Fonction target-side en radians avec réduction et polynôme binary64 | VERT | deux scénarios QEMU, dont `ATN(ATN(1))` et une erreur de syntaxe |
| `DEL n,m` | Extension target-side bornée, suppression inclusive | VERT | `test-guest-runtime-asm-repl-del.sh`; formes simple, plage et erreur |
| `RENUM new,old,step` | Reprise target-side bornée, avec prévalidation et réécriture des cibles de flot | PARTIEL | `test-guest-runtime-asm-repl-renum*.sh`, `renum-repeat-control.sh`; `GOTO`, `GOSUB`, `THEN` et les listes `ON` sont réécrits après plusieurs renumérotations |
| Autres fonctions mathématiques historiques non retenues dans la grammaire | Différées, pas implicitement supportées | DIFFÉRÉ | aucune syntaxe n’est promise sans décision et test dédiés |
| Chaînes complètes et tableaux complets | Conservés comme trajectoire produit | PARTIEL | concaténation, découpes, variables et tableaux sont présents ; opérations restantes et limites de composition sont encore à auditer |
| Fichiers, DOS, graphismes, sons, périphériques Atari | Rejetés | REJETÉ | ne font pas partie du modèle de services cible |
| Tokens et représentation mémoire TBXL | Rejetés | REJETÉ | le source est ASCII et le payload est relogeable RV |

Les mentions **VERT/PARTIEL** dans une même ligne signifient que le sous-ensemble
nommé est prouvé, mais que la famille historique est plus large. Elles ne
doivent pas être lues comme une promesse de compatibilité complète.

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

La forme `IF expression THEN` sans numéro ouvre un bloc MiniBASIC. Les blocs
peuvent être imbriqués jusqu’à huit niveaux ; `ELSE`/`ENDIF` doivent être les
premières instructions de lignes dédiées. La contrainte de lignes dédiées reste
**PARTIELLE** et doit rester visible dans le tutoriel.

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

1. finaliser le tutoriel progressif et son scénario de démonstration ;
2. consolider la surface chaîne déjà livrée : ajouter des cas limites pour
   les tableaux, l’auto-affectation, les buffers pleins et les compositions
   `STR$`/`CHR$`, et généraliser les fonctions imbriquées restantes sans
   élargir silencieusement la grammaire ;
3. auditer les familles TBXL non encore retenues
   (formatages et commandes d’édition), en conservant les limites explicites d’`ATN`, et
   prendre pour chacune une décision versionnée avant toute implémentation ;
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
- **Preuve automatisée :** 125 scripts QEMU assembleur recensés au moment de
  cet audit, avec des cas nominaux et négatifs dédiés aux chaînes, tableaux,
  conversions, fonctions de recherche, formatage et blocs structurés ;
- **Écart important restant :** `ELSE`/`ENDIF` doivent rester en lignes dédiées ;
  certaines fonctions historiques TBXL ne sont pas encore retenues ; les expressions
  chaîne générales et les autres opérateurs chaîne restent partiels ;
- **Démonstrateur étalon :** Hammurabi est maintenant une preuve target-side
  automatisée, mais sa présence ne transforme pas les familles de langage
  encore partielles en compatibilité TBXL complète ;
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
| 2026-08-03 | Concaténation target-side de termes chaîne avec `+` | `test-guest-runtime-asm-repl-string-concat*.sh` sous QEMU, cas nominal, capacité, fonctions de découpe, `CHR$` et formes invalides | les affectations composées sont exécutées dans la cible ; les conversions numériques et autres opérateurs chaîne restent hors de cette grammaire |
| 2026-08-03 | Conversion `VAL(string-source)` dans l’évaluateur numérique target-side | mêmes scénarios QEMU, littéraux/variables valides et sources vides ou avec traîne invalide | la conversion reste entièrement dans la cible et réutilise le parseur binary64 existant |
| 2026-08-03 | Recherche `INSTR` dans les sources chaîne target-side | mêmes scénarios QEMU, occurrences préfixe/milieu, absence, aiguille vide et erreurs | les pointeurs et longueurs restent dans la RAM cible ; aucun appel de recherche hôte n’est utilisé |
| 2026-08-03 | Formatage et impression `STR$(expression)` dans un buffer chaîne target-side | mêmes scénarios QEMU, valeurs positives/négatives, affectation, concaténation, `PRINT` et argument absent | le formateur partage la politique fixe à six décimales de `PRINT` sans délégation hôte |
| 2026-08-03 | Preuve des blocs `IF/ELSE/ENDIF` imbriqués | `test-guest-runtime-asm-repl-if-block-nested.sh` sous QEMU, branches vraie/fausse internes et externes | la profondeur de recherche et la pile unifiée sont désormais couvertes jusqu’à la limite documentée |
| 2026-08-03 | Inventaire recalculé | `find scripts -maxdepth 1 -type f -name 'test-guest-runtime-asm-repl*.sh' \| wc -l` → `99` | le nombre annoncé dans ce document est reproductible et non maintenu manuellement |
| 2026-08-03 | Registre de parité révisé après comparaison TBXL/MiniBASIC | matrice TBXL ci-dessus, `BASIC_LANGUAGE.md`, `BASIC_TBXL_NOTES.md` et scripts QEMU existants | les fonctions livrées sont séparées des extensions RV et des familles historiques différées ; la roadmap ne traite plus les découpes déjà implémentées comme une dette |
| 2026-08-03 | Ajout de `SQR(expression)` avec contrôle de domaine | `test-guest-runtime-asm-repl-sqr.sh` et `test-guest-runtime-asm-repl-sqr-error.sh` sous QEMU ; exécution `fsqrt.d` dans le payload | la fonction numérique historique est désormais VERT dans son sous-ensemble MiniBASIC ; les fonctions trigonométriques et logarithmiques restent différées |
| 2026-08-03 | Ajout de `SIN(expression)` et `COS(expression)` avec réduction d’intervalle et polynôme binary64 target-side | `test-guest-runtime-asm-repl-trig.sh` sous QEMU ; cas canoniques, valeurs générales et appels imbriqués | la première famille transcendantale TBXL est VERT dans le sous-ensemble MiniBASIC ; `ATN`, `LOG` et `EXP` restent explicitement différés |
| 2026-08-03 | Ajout de `TAN(expression)` par composition target-side de `SIN/COS` et refus des pôles | `test-guest-runtime-asm-repl-tan.sh` et `test-guest-runtime-asm-repl-tan-error.sh` sous QEMU ; valeurs générales, imbrication et `COS(pi/2)=0` | la famille trigonométrique directe est maintenant VERT dans le sous-ensemble MiniBASIC ; `ATN`, `LOG` et `EXP` restent explicitement différés |
| 2026-08-03 | Ajout de `LOG(expression)` et `EXP(expression)` avec réduction binary64, polynômes target-side et bornes de domaine | `test-guest-runtime-asm-repl-log-exp.sh` et `test-guest-runtime-asm-repl-log-exp-error.sh` sous QEMU ; valeurs usuelles, composition et `LOG(0)` | la famille logarithmique/exponentielle est disponible dans une approximation déterministe bornée ; `ATN` reste le dernier grand manque mathématique explicite |
| 2026-08-03 | Ajout et validation de `ATN(expression)` target-side avec réduction par réciproque et intervalle `pi/4` | `test-guest-runtime-asm-repl-atn.sh` et `test-guest-runtime-asm-repl-atn-error.sh` sous QEMU ; cas nominaux, imbrication et erreur | `ATN` devient VERT dans le sous-ensemble MiniBASIC ; les cadres statiques limitent l’imbrication à deux niveaux |
| 2026-08-03 | Ajout de `DEL n` et `DEL n,m` target-side | `test-guest-runtime-asm-repl-del.sh` sous QEMU ; suppression simple, plage inclusive, listing, exécution et plage inversée | l’édition de blocs TBXL est partiellement reprise ; `RENUM` et les commandes de fichiers restent différés |
| 2026-08-03 | Ajout de `RENUM new,old,step` target-side avec prévalidation | `test-guest-runtime-asm-repl-renum.sh` et `test-guest-runtime-asm-repl-renum-error.sh` sous QEMU ; corps simples, listing, exécution et erreur sans écriture partielle | la commande d’édition est PARTIELLE : les records conservent un alias de leur numéro précédent pour les résolveurs de contrôle de flot |
| 2026-08-03 | Réécriture target-side des références après plusieurs `RENUM` | `test-guest-runtime-asm-repl-renum-repeat-control.sh` sous QEMU ; deux renumérotations successives et `GOTO 30` exécutent `REN2` | les records restent en place mais les cibles `GOTO`/`GOSUB`/`THEN`/`ON` sont réémises dans un scratch cible ; la limite de record à 111 octets reste explicite |
| 2026-08-03 | Prévalidation transactionnelle de la capacité `RENUM` | `test-guest-runtime-asm-repl-renum-capacity.sh` sous QEMU ; une réécriture dépassant 111 octets est refusée, les numéros et le corps sont restaurés, sans fault | la limite de record est désormais protégée sans mutation partielle |
| 2026-08-03 | `LEN` évalue des concaténations chaîne simples dans le guest | `test-guest-runtime-asm-repl-string-len-concat.sh` et régression `string-len.sh` sous QEMU ; littéraux, variables et longueur totale après plusieurs appels `LEN` | le premier consommateur de fonctions chaîne accepte une expression composée ; fonctions imbriquées et généralisation aux autres fonctions restent partielles |
| 2026-08-04 | Validation des consommateurs numériques avec sources chaîne imbriquées | `test-guest-runtime-asm-repl-string-consumer-nested.sh` sous QEMU : `LEN(LEFT$(...))`, `ASC(RIGHT$(...))`, `VAL(LEFT$(...))` et `INSTR(RIGHT$(...),...)`, résultats `4`, `82`, `12.5` et `1` | `LEN`, `ASC`, `VAL` et `INSTR` partagent effectivement le résolveur `{adresse,longueur}` target-side ; la profondeur et la capacité des buffers restent les limites explicites |
| 2026-08-03 | `ASC` évalue des concaténations chaîne simples dans le guest | `test-guest-runtime-asm-repl-string-asc-concat.sh` et `string-asc-concat-error.sh` sous QEMU ; littéral + littéral, variable + littéral et chaîne vide | `ASC` rejoint `LEN` comme consommateur d’expression chaîne ; `VAL`, `INSTR`, `LEFT$`, `RIGHT$` et `MID$` restent à généraliser séparément |
| 2026-08-03 | Validation de la concaténation dans `PRINT` | `test-guest-runtime-asm-repl-string-print-concat.sh` sous QEMU : `"RV "+TEXT$`, `TEXT$+"!"`, `LEFT$(TEXT$,4)+"!"`, `"A"+"B"+"C"`, `CHR$(65)+"B"` et `STR$(12.5)+"!"` | la sortie concaténée, les découpes et les conversions chaîne sont calculées dans le payload ; l’affectation concaténée reste PARTIELLE pour les conversions numériques |
| 2026-08-04 | Audit de cohérence du registre | comptage reproductible : `find scripts -maxdepth 1 -type f -name 'test-guest-runtime-asm-repl*.sh' \| wc -l` → `125` | l’inventaire courant est distingué des comptes historiques ; les preuves de fonctions chaîne imbriquées, `HEX$`, `DEC` et `INKEY$` sont incluses |
| 2026-08-04 | Ajout de `HEX$(expression)` target-side | `test-guest-runtime-asm-repl-string-hex.sh` sous QEMU : `0`, `FF`, `1234ABCD`, concaténation/affectation et trois erreurs de domaine | une fonction historique de conversion est reprise sans hôte, avec format ASCII majuscule déterministe et borne RV explicite |
| 2026-08-04 | Ajout des comparaisons chaîne dans `IF` | `test-guest-runtime-asm-repl-string-compare.sh` sous QEMU : les six opérateurs, variables/littéraux, résultat de branche et absence de `BAD-*`/fault | l’ordre lexicographique ASCII et les longueurs sont comparés dans la cible ; la forme reste limitée à `IF` et n’élargit pas silencieusement toutes les expressions |
| 2026-08-04 | Ajout de `DEC(string-source)` target-side | `test-guest-runtime-asm-repl-string-dec.sh` sous QEMU : `0`, `FF`, `1a2b`, `D`, `DE`, `DEA`, `DEAD`, `HEX$(DEC("DEAD"))` et absence de fault | la conversion inverse de `HEX$` est exécutée par l’assembleur cible ; la syntaxe V1 est limitée aux chiffres hexadécimaux sans préfixe et la table du lexer passe à 19 entrées |
| 2026-08-04 | Ajout de `INKEY$()` target-side | `test-guest-runtime-asm-repl-string-inkey.sh` sous QEMU : expression composée avec `"["+INKEY$()+"]"`, file UART vide et absence de fault | une fonction TBXL de lecture clavier est modernisée vers `poll-char`, sans horloge ni état hôte caché ; la disponibilité d'un octet est conservée comme cas à renforcer |
| 2026-08-03 | Intégration de l’évaluateur tokenisé pour signes unaires devant littéraux | `test-guest-runtime-asm-repl-unary-paren.sh` sous QEMU : `(-2.5)+(+3.5)`, sortie `1.000000`, motifs binaires finaux et marqueur `0x82062720=1` | les signes sont traités dans le guest sans écraser la profondeur de pile ; un signe devant un groupe parenthésé revient volontairement au parseur historique |
| 2026-08-04 | Les découpes d’affichage et leurs affectations consomment une expression chaîne composée | `test-guest-runtime-asm-repl-string-slice-expression.sh` sous QEMU : affichages `LEFT$(TEXT$+"X",5)`, `RIGHT$("0"+TEXT$,4)`, `MID$(TEXT$+"X",2,4)` et variables `LEFTOUT$`, `RIGHTOUT$`, `MIDOUT$` | le résolveur target-side commun s’arrête à la virgule de niveau zéro ; le probe lexical choisit le concaténateur pour les affectations composées |
| 2026-08-04 | Sélection target-side du concaténateur pour les affectations de découpes composées | même test QEMU, avec `LEFTOUT$`, `RIGHTOUT$` et `MIDOUT$` composés, plus `string-right-assignment-concat.sh` et les scénarios d’erreur | un probe lexical ignore les `+` entre guillemets ou parenthèses imbriquées ; les sources simples gardent le handler spécialisé et les expressions composées utilisent le résolveur commun |
| 2026-08-03 | Correction de la résolution des tableaux courts nommés `C` | `test-guest-runtime-asm-repl-array-table.sh` sous QEMU : `DIM B(3)`, `DIM C(2)`, affectations et `PRINT B(1)+C(2)` produisent `16.000000` | `C(...)` ne tombe plus dans le résolveur de tableaux longs ; le test de la fonction `COS(...)` doit rester vert |
| 2026-08-03 | `LEN` accepte les littéraux ASCII | `test-guest-runtime-asm-repl-string-len.sh` sous QEMU, avec `LEN("RV64")` en plus des variables et tableaux | la résolution reste target-side ; les expressions chaîne générales restent différées |
| 2026-08-03 | Correction de `RIGHT$` concaténé dans une affectation et preuve d’une destination tableau | `test-guest-runtime-asm-repl-string-right-assignment-concat.sh` et `test-guest-runtime-asm-repl-string-array-assignment-concat.sh` sous QEMU ; `RABI<` et `RABI!` sans fault | les trois découpes d’affectation rejoignent leur concaténateur target-side propre, y compris pour un élément de tableau ; la composition générale reste PARTIELLE |
| 2026-08-03 | Généralisation du résolveur d'expressions chaîne pour `LEN`, `ASC` et `VAL` | `test-guest-runtime-asm-repl-string-len-concat.sh`, `test-guest-runtime-asm-repl-string-asc-concat.sh`, `test-guest-runtime-asm-repl-string-val-concat.sh` et leurs régressions QEMU ; parenthèses imbriquées validées par `LEN(A$(1))` | les trois consommateurs partagent le contrat target-side `{adresse,longueur}` ; la profondeur est bornée à 8 et les buffers sont empilés par contexte |
| 2026-08-03 | `INSTR` consomme deux expressions chaîne séparées par une virgule de niveau zéro, dont `LEFT$` imbriqué | `test-guest-runtime-asm-repl-string-instr-expression.sh` sous QEMU : concaténation dans les deux opérandes, virgule littérale, `INSTR(LEFT$(...),...)` et résultats 4, 3, 2, 2 | la séparation syntaxique et la composition sont génériques dans les bornes documentées ; le contrat préserve pointeur, curseur et `x31` de l'évaluateur appelant |
| 2026-08-03 | Entrée de tous les noms alphabétiques dans le reconnaisseur table-driven | `test-guest-runtime-asm-repl-numeric-functions.sh` et `test-guest-runtime-asm-repl-array-table.sh` sous QEMU ; `ABS`, `TRUNC`, `FRAC`, `MOD` et repli `B(1)+C(2)` | les noms majuscules et minuscules suivent le même parcours ; le repli restaure l'état nécessaire aux variables/tableaux et chaque entrée est recalculée depuis `base + index*16` |
| 2026-08-03 | Préreconnaissance table-driven des mots-clés statements | `test-guest-runtime-asm-repl.sh`, `numeric-functions.sh`, `if.sh` et `data-read.sh` sous QEMU ; mots-clés en casse mixte, normalisation target-side et repli legacy | la table borne les noms, longueurs et IDs sans créer un second exécuteur ; les handlers `FOR/NEXT` et autres familles conservent encore leurs contrats historiques jusqu’à migration dédiée |
| 2026-08-03 | Migration directe du handler `PRINT` via l'ID 8 | `test-guest-runtime-asm-repl.sh`, `test-guest-runtime-asm-repl-numeric-functions.sh` et `test-guest-runtime-asm-repl-string-print-concat.sh` sous QEMU | `PRINT` restaure le contexte attendu puis appelle `print_dispatch` depuis la table ; les autres statements conservent le repli legacy jusqu'à validation de leur contrat |
| 2026-08-03 | Routage de `END` conservé sur le fallback legacy | régressions QEMU contenant `END` | le comportement reste target-side et inchangé ; son veneer direct est différé pour rester sous la limite de labels du mini-assembleur |
| 2026-08-03 | Migration directe du handler `REM` via l'ID 23 | `test-guest-runtime-asm-repl-rem.sh` sous QEMU, avec ligne `REM` suivie d'affectations et de `PRINT` | `REM` restaure le contexte attendu puis appelle `rem_statement`; le reste de la ligne est ignoré par le chemin target-side historique |
| 2026-08-03 | Migration directe du handler `GOTO` via l'ID 1 | `test-guest-runtime-asm-repl-goto.sh`, `goto-30.sh` et `break.sh` sous QEMU | `GOTO` restaure le contexte puis appelle `goto_dispatch`, qui résout les numéros de ligne génériques ; `GOSUB` reste sur son contrat legacy jusqu'à migration dédiée |
| 2026-08-03 | Migration directe du handler `IF` via l'ID 15 | `test-guest-runtime-asm-repl-if.sh` sous QEMU ; comparaison, `THEN` et saut vers une ligne | `IF` restaure le contexte puis appelle `if_statement`, qui conserve l'évaluation target-side et les branches historiques |
| 2026-08-03 | Migration directe des handlers `ELSE` et `ENDIF` via les IDs 16/17 | `test-guest-runtime-asm-repl-if-block.sh`, `if-block-nested.sh` et `if-block-error.sh` sous QEMU | les deux terminators restaurent le contexte puis appellent leurs handlers target-side ; les blocs conditionnels n'utilisent plus le dispatch de préfixe pour ces mots-clés |
| 2026-08-03 | Correction du scan des `ENDIF` imbriqués | `test-guest-runtime-asm-repl-if-block-nested.sh` sous QEMU ; `INNER-ELSE-OK` et `OUTER-ELSE-OK` sans `ERR` | le scanner décrémente sa profondeur avant de traiter l'`ENDIF` d'un bloc imbriqué ; seul le terminator au niveau recherché dépile ou reprend l'exécution |
| 2026-08-03 | Routage de `INPUT` conservé sur le fallback legacy | régressions `test-guest-runtime-asm-repl-input.sh`, `long-input.sh` et `long-string-input.sh` | le comportement target-side reste inchangé ; son veneer direct est différé afin de libérer une étiquette pour `ON` sous la limite du mini-assembleur |
| 2026-08-03 | Migration directe du handler `ON` via l'ID 24 | `test-guest-runtime-asm-repl-on.sh` sous QEMU ; sélections `GOTO` et `GOSUB` | `ON` restaure le contexte puis appelle `on_statement`; la résolution des cibles et la pile d'appel restent target-side |
| 2026-08-03 | Migration directe des handlers `FOR` et `NEXT` via les IDs 6/7 | `test-guest-runtime-asm-repl-for.sh`, `for-y.sh`, `for-step.sh`, `for-long-name.sh` et `for-nested.sh` sous QEMU | les deux handlers restaurent le contexte avant d'utiliser les frames flottantes target-side ; la profondeur et les bornes `STEP` restent centralisées dans le contrat historique |
| 2026-08-03 | Migration directe des handlers `WHILE` et `WEND` via les IDs 3/14 | `test-guest-runtime-asm-repl-while.sh` et `while-error.sh` sous QEMU ; boucles simples, imbriquées et `WEND` orphelin | les handlers restaurent le contexte avant d'utiliser la frame de continuation et les comparaisons binary64 target-side |
| 2026-08-03 | Migration directe des handlers `REPEAT` et `UNTIL` via les IDs 13/4 | `test-guest-runtime-asm-repl-repeat.sh` et `repeat-error.sh` sous QEMU ; reprise, imbrication et `UNTIL` orphelin | les handlers restaurent le contexte avant d'utiliser la pile REPEAT target-side et l'évaluation binary64 de la condition |
| 2026-08-03 | Migration directe des handlers `DO` et `LOOP` via les IDs 19/20 | `test-guest-runtime-asm-repl-do-loop.sh` sous QEMU ; paire inconditionnelle et pile de contrôle | les handlers restaurent le contexte puis utilisent la frame DO target-side ; les variantes conditionnelles restent explicitement hors du sous-ensemble V1 |
| 2026-08-03 | Migration directe des handlers `GOSUB` et `RETURN` via les IDs 2/10 | `test-guest-runtime-asm-repl-gosub.sh`, `on.sh`, `pop.sh` et `renum-control.sh` sous QEMU | les deux IDs partagent le veneer `statement_table_goto` ; `x8+2648` sélectionne `goto_dispatch` ou `return_statement` après restauration du contexte, sans augmenter la limite de labels |
| 2026-08-03 | Correction de la profondeur `FOR/NEXT` pendant l'analyse des bornes | `test-guest-runtime-asm-repl-for-y.sh` et `test-guest-runtime-asm-repl-for-nested.sh` sous QEMU ; un et deux niveaux | la profondeur est sauvegardée en `x8+2608` avant `TO/STEP`, puis rechargée avant publication ; les boucles imbriquées produisent les résultats attendus |

Les affectations ne constituent pas encore une parité chaîne complète : le RHS
est soit une forme `LEFT$`/`RIGHT$`/`MID$`, soit une concaténation de termes
littéraux, variables, éléments de tableau chaîne, fonctions de découpe ou
`CHR$` ; les conversions numériques et autres opérateurs chaîne sont
explicitement hors de cette tranche. Toute extension doit ajouter
un test QEMU target-side et une entrée à cette matrice avant d’être marquée
VERT.
