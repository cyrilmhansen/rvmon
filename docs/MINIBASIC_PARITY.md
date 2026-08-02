# MiniBASIC-RV / Turbo BASIC XL 1.5 — suivi de parité

## Objet et limites

Ce document suit la parité fonctionnelle entre MiniBASIC-RV et les idées
structurantes de Turbo BASIC XL 1.5. Il ne promet ni compatibilité Atari, ni
compatibilité binaire, ni reproduction des nombres, tokens, périphériques,
DOS ou graphismes Atari.

Les références d’étude sont :

- [Turbo-BASIC XL — Expanded Documentation](https://ftp.pigwa.net/stuff/collections/Atari%20books/Turbo-BASIC%20XL%20-%20Expanded%20Documentation.pdf), référence d’expérience et de langage historique ;
- [`docs/BASIC_TBXL_NOTES.md`](BASIC_TBXL_NOTES.md), synthèse du désassemblage et des artefacts étudiés ;
- les sources ASM-One v1.48 sous `docs/dontcommit/`, utilisées uniquement pour
  l’ergonomie du moniteur.

La preuve de MiniBASIC est toujours une exécution QEMU du payload assembleur
chargé par le moniteur. L’interpréteur Rust résident (`basic-rust`) n’est pas
utilisé comme oracle des résultats target-side.

## Légende

- **VERT** : implémenté dans l’assembleur cible et couvert par un test QEMU.
- **PARTIEL** : comportement présent mais limité par une décision documentée.
- **PLANIFIÉ** : compatible avec la trajectoire, pas encore accepté.
- **REJETÉ V1** : hors périmètre local, même si TBXL le propose.

## Matrice de suivi

| Domaine TBXL / MiniBASIC | MiniBASIC-RV actuel | État | Preuve ou remarque |
|---|---|---:|---|
| Invite directe et programme à lignes | `READY>`, insertion/remplacement/suppression, tri, `LIST` | VERT | `test-guest-runtime-asm-repl-two-lines.sh`, `four-lines.sh` |
| `NEW`, `RUN`, `TRACE` | exécution et trace target-side | VERT | `test-guest-runtime-asm-repl-trace.sh` |
| `PRINT` / `?` | nombres, chaînes, expressions et mélange | VERT | `print-mixed.sh`, `question.sh`, `string.sh` |
| `INPUT` | numérique et chaînes, y compris noms longs | VERT | `input.sh`, `long-input.sh`, `long-string-input.sh` |
| Expressions | parenthèses, signes, précédence, comparaisons, binary64 D | VERT | `precedence.sh`, `unary-paren.sh`, `format-negative-fraction.sh` |
| Variables numériques | `A..Z` et noms ASCII jusqu’à 16 caractères | VERT | `scalars.sh`, `long-names.sh`, `keyword-vars.sh` |
| Chaînes | cellules target-side, noms courts/longs, affectation/affichage | VERT | `string-var.sh`, `long-string.sh` |
| Tableaux numériques | 1D/2D, noms courts/longs, index calculés et bornes | VERT | `array*.sh`, `long-numeric-array*.sh` |
| Tableaux de chaînes | 1D/2D, noms courts/longs, stockage et bornes target-side | VERT | `string-array*.sh`, `long-string-array*.sh` |
| `IF ... THEN` / `GOTO` | comparaisons et cibles par numéro de ligne | VERT | `if.sh`, `if-false.sh`, `goto*.sh` |
| `FOR ... NEXT` | huit niveaux, `STEP` positif/négatif, noms longs | VERT | `for.sh`, `for-nested.sh`, `for-step.sh` |
| `GOSUB ... RETURN` | huit retours, cible et retour target-side | VERT | `gosub.sh` |
| `DATA`, `READ`, `RESTORE` | valeurs numériques et chaînes, curseur target-side | VERT | `data-read.sh`, `restore.sh` |
| `REM` | commentaire BASIC | VERT | `rem.sh` |
| `WHILE ... WEND` | huit niveaux, comparaisons et recherche de terminator | VERT | `while.sh`, `while-error.sh` |
| `REPEAT ... UNTIL` | huit niveaux, test terminal et comparaisons | VERT | `repeat.sh`, `repeat-error.sh` |
| `POP` | retire le cadre le plus récent de `FOR/GOSUB/WHILE/REPEAT` | VERT | `pop.sh`, forme `POP:GOTO` |
| `EXIT` | sortie structurée de `FOR/WHILE/REPEAT` avec scan typé | VERT | `test-guest-runtime-asm-repl-exit.sh` |
| `DO ... LOOP` | boucle infinie structurée | PLANIFIÉ | à traiter avec `EXIT` et la pile unifiée |
| `IF ... ELSE ... ENDIF` | non disponible | PLANIFIÉ | extension structurée TBXL, distincte du `IF ... THEN` V1 |
| `ON ... GOTO/GOSUB` | non disponible | PLANIFIÉ | sélection entière et liste de cibles |
| `PROC/EXEC`, fonctions utilisateur | non disponible | REJETÉ V1 | hors démonstrateur MiniBASIC actuel |
| fichiers, DOS, graphismes, sons, périphériques Atari | non disponible | REJETÉ V1 | remplacés par les services RVMonitor documentés |
| compilation native BASIC | roadmap séparée | DIFFÉRÉ | `BASIC_COMPILER_ROADMAP.md`, interpréteur prioritaire |

## Modèle d’exécution et divergences assumées

Turbo BASIC XL utilise une représentation et des conventions 8-bit Atari ;
MiniBASIC-RV utilise exclusivement des structures dans la RAM cible et
`binary64` exécuté par `fadd.d`, `fsub.d`, `fmul.d` et `fdiv.d`. Les résultats
ne doivent donc pas être comparés bit à bit avec les valeurs Atari historiques.

Les piles spécialisées de MiniBASIC restent utiles aux algorithmes existants,
mais une pile unifiée `{kind, line}` est maintenant maintenue en parallèle pour
les opérations de nesting et `POP`. Une instruction de contrôle qui ne trouve
pas son cadre attendu produit une erreur cible ; elle ne vide jamais
silencieusement une pile étrangère.

Le format d’affichage numérique est fixe à six décimales pour V1. Les sorties
de Hammurabi sont ainsi déterministes entre plateformes, mais elles ne
reproduisent pas nécessairement le rendu de TBXL.

## Fonctionnalités historiques à ne pas confondre

Le mot « parité » signifie ici parité de l’expérience de programmation et des
structures utiles au démonstrateur : REPL, lignes numérotées, édition rapide,
contrôle de flot, boucles, sous-programmes, données, chaînes, tableaux,
diagnostics et observation au débogueur. Il ne signifie pas que les commandes
suivantes doivent être ajoutées : `DIR`, `DELETE`, `RENAME`, `LOCK`, `UNLOCK`,
`GRAPHICS`, `CIRCLE`, `PAINT`, `SOUND`, `PEEK/POKE` Atari, `USR`, ou les
opérations DOS propriétaires.

## Prochaine séquence recommandée

1. Ajouter `DO/LOOP` avec un cadre dédié dans la pile unifiée et diagnostics
   d’imbrication.
2. Décider explicitement si `IF/ELSE/ENDIF` et `ON GOTO/GOSUB` appartiennent au
   profil MiniBASIC-RV étendu ; ne pas les introduire comme compatibilité
   implicite.
3. Ajouter ensuite les fonctions numériques génériques (`RND`, `TRUNC`,
   `FRAC`, `MOD`) si elles sont nécessaires à des programmes pédagogiques,
   avec motifs et résultats calculés dans la cible.

## Audit courant

La conversion assembleur couvre actuellement le chemin utile de bout en bout :
source assembleur accepté par le moniteur, chargement U-mode, lexing et
évaluation BASIC dans la cible, registres flottants observables, mémoire cible,
breakpoints, interruption et reprise. La matrice assembleur compte maintenant
60 tests QEMU ; après durcissement du harnais de tableau de chaînes 2D, le cas
qui échouait sporadiquement passe cinq fois consécutives. La parité TBXL n’est
pas déclarée complète tant que `DO/LOOP` et les décisions sur les
extensions restantes ne sont pas résolus.
