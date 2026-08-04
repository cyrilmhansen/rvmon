# Roadmap du compilateur MiniBASIC-RV

## Positionnement

Le compilateur est une extension postérieure au démonstrateur interprété. Il
produit un payload RV64 exécutable dans la machine cible, accompagné de son
runtime cible, de symboles et d’une carte source. Il ne remplace pas
l’interpréteur : `RUN` conserve le chemin interprété et `COMPILE` choisit
explicitement le chemin natif.

L’objectif est comparable à l’expérience Turbo-BASIC XL 1.5 : transformer un
programme BASIC en exécutable plus rapide avec un runtime et un linker séparés.
Ce n’est pas une compatibilité binaire Atari et ses optimisations internes ne
seront pas reproduites sans preuve indépendante.

AtariWiki référence le compilateur 1.5, Compiler 1.1, Runtime, Linker, leurs
manuels, images et un désassemblage MADS, mais précise que le source original
est perdu. L’étude utilisera donc les artefacts disponibles et leurs licences.
Aucun code historique ne sera copié dans RVMonitor sans provenance et droit
d’utilisation explicites.

## Contrat fonctionnel proposé

| Commande | Effet |
|---|---|
| `COMPILE [options]` | analyse le programme et produit un artefact natif dans le workspace |
| `COMPILE LIST` | affiche le plan, les routines runtime requises et les diagnostics |
| `RUN-COMPILED` | charge puis exécute le dernier artefact accepté |
| `COMPILE-CLEAR` | supprime l’artefact compilé sans modifier le programme BASIC |

Le programme source, le profil ISA, le hash du runtime et les options sont
intégrés au manifeste de l’artefact. Un artefact incompatible est refusé.

## Architecture

1. **Analyse partagée** : lexer, parseur, noms longs, chaînes, tableaux et
   expressions produisent un AST commun à l’interpréteur et au compilateur.
2. **Vérification statique** : résolution des lignes, variables, dimensions,
   bornes configurées et instructions supportées.
3. **IR MiniBASIC** : contrôle de flot explicite, temporaires binary64,
   indexation entière, accès variables, appels runtime et positions source.
4. **Backend RV64** : génération d’assembleur accepté par le moniteur ou d’un
   objet interne équivalent ; les encodages proviennent toujours de R2 généré.
5. **Linker/runtime cible** : fusion du code avec les routines d’E/S,
   conversion numérique, chaînes, tableaux, boucles et diagnostics.
6. **Loader/debug** : image, symboles, lignes source et variables sont chargés
   comme un payload normal.

Le premier backend est un générateur RV64 direct et simple. LLVM, GCC et une VM
intermédiaire restent des prototypes ou oracles, pas des dépendances runtime.

## Sous-ensemble compilable initial

Le premier jalon couvre les variables numériques et les noms jusqu’à 16
caractères, constantes, opérateurs et parenthèses, `LET`, `PRINT`, `IF`,
`GOTO`, `FOR/NEXT`, `INPUT` et `END`. Les chaînes et tableaux suivent après
validation de leur runtime cible.

Les constructions non compilables produisent un diagnostic stable avec ligne,
colonne, cause et alternative (`RUN` interprété). Une instruction ne peut pas
être compilée silencieusement en no-op.

## Jalons

- **COMP-0 — étude et provenance** : cataloguer manuels, images, désassemblage
  MADS, sources communautaires et licences ; documenter compiler/runtime/linker.
- **COMP-1 — AST/IR partagé** : mêmes sources et diagnostics que l’interpréteur.
- **COMP-2 — backend expressions** : `fadd.d`, `fsub.d`, `fmul.d`, `fdiv.d`,
  symboles et source mapping.
- **COMP-3 — contrôle de flot et E/S** : `IF/GOTO`, `FOR/NEXT`, `PRINT`,
  `INPUT`, `END`, payload chargeable par `run-at`.
- **COMP-4 — chaînes, tableaux et debug** : D-018, bornes, variables et
  breakpoints source.
- **COMP-5 — optimisation mesurée** : constantes, temporaires, boucles puis
  benchmarks reproductibles, sans modifier les résultats ou `fflags`.

## Critères d’acceptation

Le programme compilé doit s’exécuter dans le guest RV64, sans interpréteur BASIC
hôte ni sortie préenregistrée. Pour le sous-ensemble couvert, le chemin
interprété et le chemin compilé doivent produire les mêmes sorties, mémoire,
résultats binary64 et `fflags`, contrôlés par QEMU et des motifs de bits
indépendants.

Le premier démonstrateur sera un calcul `FOR` contenant `X=I/3` : il devra
montrer `fdiv.d`, la ligne BASIC correspondante, le résultat binary64 et une
mesure séparée du nombre d’instructions. Hammurabi sera compilé après validation
des chaînes, tableaux et `INPUT`.

## Risques

- Le désassemblage historique peut être incomplet ou soumis à une autre
  licence : il sert d’étude, pas de dépendance de build.
- Le format `.luna` ou payload brut contrôlé est prioritaire sur ELF externe.
- Le runtime compilé et interprété partagent des primitives testées, mais leurs
  chemins restent distincts pour conserver un oracle différentiel.
