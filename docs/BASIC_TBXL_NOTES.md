# Notes d’étude Turbo-BASIC XL

Cette note consigne l’usage de Turbo-BASIC XL comme référence d’architecture
et d’expérience, sans en faire une spécification de compatibilité de
MiniBASIC-RV.

## Sources examinées

- [dmsc/turbo-dis](https://github.com/dmsc/turbo-dis), désassemblage MADS de
  Turbo-BASIC XL 1.5. Le dépôt est organisé en modules assembleur et annonce
  une reconstruction relogeable testée à plusieurs adresses.
- [page de préservation AtariWiki](https://www.atariwiki.org/wiki/Wiki.jsp?page=Turbo-BASIC+XL),
  qui référence les images ATR contenant des sources et les variantes 1.5/2.0.
- [image ATR source DOS XE](https://www.atariwiki.org/wiki/attach/Turbo-BASIC%20XL/TBXL_DSDD_DOS_XE.atr)
  et [image ATR complète 1.5](https://www.atariwiki.org/wiki/attach/Turbo-BASIC%20XL/TURBO-BASIC_XL_1.5_full.atr),
  inspectées temporairement hors dépôt. Les chaînes de l’image source DOS XE
  confirment une organisation en parties `TURBO.001` à `TURBO.015`.

L’image disque n’est pas commitée : elle reste une source externe de référence
et son extraction n’est pas une dépendance de construction de RVMonitor.

La référence de travail du désassemblage est figée au commit
`c97e1b2132793cae583fa4819c31cf6465b645f5` (branche `HEAD` observée le
2026-08-03). Une évolution du dépôt TBXL doit produire une nouvelle note de
comparaison ; elle ne doit pas modifier silencieusement les conclusions
ci-dessous.

## Valeur de l'étude du runtime privé

Le runtime TBXL n'est pas une dépendance de l'OS Atari : c'est une
implémentation privée de l'interpréteur et de son exécution, écrite pour la
machine 6502. Cette distinction est essentielle. Nous n'importons pas une
API Atari abstraite ; nous étudions une architecture complète de langage
exécuté dans une cible contrainte.

Le désassemblage est donc utilisé comme une source de décisions structurelles
et de questions de vérification : où sont séparés les tokens et les lignes,
comment les tables associent syntaxe et handlers, quels états sont conservés
par l'évaluateur, comment les erreurs reprennent l'exécution et comment les
piles d'opérandes et de contrôle bornent l'imbrication. Une décision RV qui
s'écarte de TBXL doit être motivée par l'ISA, l'ABI, la mémoire ou le modèle
numérique, et non par une méconnaissance du runtime historique.

## Constats vérifiés dans le désassemblage 1.5

Ces constats sont des faits d’implémentation du runtime privé TBXL, pas des
promesses de compatibilité MiniBASIC-RV :

- `stmttab.asm` contient une table d’adresses de handlers indexée par le token
  de statement. Les entrées non exécutables et les mots réservés peuvent
  pointer vers un même retour : le dispatch est une table de données, non une
  chaîne de tests indépendante pour chaque mot-clé.
- `syntable.asm` encode une grammaire compacte par sous-tables : alternatives
  (`SOR`), retours (`SRTN`), transformations de tokens (`SCHNG`) et appels de
  sous-règles. Les appels imbriqués de fonctions ne sont donc pas une liste de
  combinaisons câblées ; ils sont bornés par l’état des piles et la grammaire.
- `exexpr.asm` sépare la lecture des éléments (`GETTOK`), la pile d’opérateurs
  (`OPSTK`) et l’exécution des opérations (`EXOP`). La précédence est une
  table (`OPRTAB`/`OPLTAB`) et les réductions précèdent l’empilement de
  l’opérateur entrant lorsque nécessaire.
- `execnl.asm` sépare la progression dans une ligne, l’identification de la
  prochaine instruction et le saut indirect dans `STMT_X_TAB`. Le point de
  reprise de `TRACE` est ainsi au début du dispatch, pas dans chaque handler.
- `argstack.asm` réserve séparément les zones de variables et d’arguments ;
  `opstack.asm` réserve 64 octets pour les opérateurs. Le stockage des lignes
  peut partager une région temporaire pendant l’initialisation, mais cette
  réutilisation est explicite et bornée.
- `errors.asm` centralise le numéro d’erreur, la ligne courante, le traitement
  d’un `TRAP` et le retour à l’invite. `errortab.asm` sépare les codes des
  chaînes affichées. Ce découplage est directement pertinent pour les
  diagnostics stables de MiniBASIC-RV.

## Conséquences obligatoires pour le portage RV

1. Le lexer RV doit seulement reconnaître et publier des tokens ; il ne doit
   pas connaître toutes les combinaisons d’instructions valides.
2. Le parseur d’expressions doit consommer une séquence générique de tokens
   avec une table de précédence et des piles bornées. Parenthèses, appels et
   futurs arguments doivent être ajoutés par règles, pas par chemins spéciaux
   dans chaque handler.
3. Le dispatch des statements doit être une table `{token_id, handler,
   flags}` inspectable dans la mémoire cible. Les handlers peuvent rester
   différents, mais la sélection doit être uniforme.
4. L’état d’exécution doit distinguer le curseur de source, la pile
   d’opérandes, la pile de contrôle (`FOR`, `GOSUB`, boucles) et le contexte de
   diagnostic. Une seule pile U-mode pour ces rôles serait contraire à
   l’enseignement tiré de TBXL et rendrait les bornes difficiles à vérifier.
5. Une erreur de syntaxe ou d’exécution doit passer par un point de reprise
   commun, avec conservation du numéro de ligne et abandon contrôlé de la
   transaction d’exécution en cours.

Ces points deviennent des critères de revue pour toute intégration du parseur
tokenisé dans `payload-repl.rv` et pour le futur portage des chaînes et
tableaux.

## Découpage retenu pour RV

Le désassemblage montre une séparation utile entre pile d’opérandes, pile
d’exécution, tables de syntaxe, recherche d’instructions, évaluateur
d’expressions, exécution des lignes et gestionnaires de mots-clés. Nous
reprenons ce principe sous une forme adaptée à RV64 :

| TBXL | MiniBASIC-RV chargé |
|---|---|
| `argstack` / `opstack` | buffers statiques de lexer, parseur et évaluateur |
| `stmttab` / `syntable` | tables de mots-clés et dispatch borné |
| `exexpr` | parseur d’expressions binary64 avec `fadd.d`, `fmul.d`, `fdiv.d` |
| `execnl` | boucle ligne par ligne et contrôle de flot |
| `x-for*`, `x-next`, `x-ifthen` | modules `FOR/NEXT`, `IF/THEN`, `GOTO` |
| `x-print`, `prt*` | sortie texte et conversion binary64 cible |
| `errors` | codes, ligne, colonne et retour à l’invite |

Le portage ne traduira pas les adresses 6502, les appels spécifiques à l'OS
Atari, ni la représentation binaire historique des tokens ou la BCD. Les
nombres restent binary64 IEEE exécutés par D et
les services d’E/S passent par l’ABI `RVMPAY01`.

## Décisions d’implémentation

1. Le stockage source reste distinct du code : il sera tokenisé ou conservé
   dans des records fixes selon le coût mesuré, mais ne dépendra pas d’une
   adresse absolue.
2. Les tables de dispatch seront des données du payload, avec labels et
   bornes vérifiables par `DUMP`; elles ne seront pas une copie du bytecode
   6502.
3. Les piles temporaires seront des zones statiques dans la région de données
   du payload. La pile U-mode servira aux appels et aux retours, avec une
   limite explicite ; le moniteur ne déplacera pas silencieusement ces zones.
4. La relocalisation sera traitée par le chargeur RV avant l’exécution. Une
   source assembleur guest ne pourra pas supposer que son adresse d’image est
   fixe.

## Ce qui est repris, modernisé ou rejeté

- Repris : invite immédiate, lignes numérotées, LIST/RUN, tables de dispatch,
  contrôle de flot, TRACE et erreurs courtes.
- Modernisé : séparation lexer/parser/exécuteur inspirée de la séparation
  observée dans le runtime TBXL, binary64 matériel, ABI ecall,
  bornes mémoire, diagnostics stables, snapshots et débogage M-mode.
- Rejeté en V1 : compatibilité Atari, BCD, graphismes, DOS, mémoire relogeable
  sous ROM et commandes non prévues par `BASIC_LANGUAGE.md`. Les chaînes et
  tableaux complets ne sont pas rejetés : ils sont conservés comme extensions
  obligatoires de MiniBASIC-RV et seront implémentés dans des modules cibles
  dédiés.
