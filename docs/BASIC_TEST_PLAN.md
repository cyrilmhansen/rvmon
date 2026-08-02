# Plan de tests MiniBASIC-RV

## Tests cible QEMU

- `scripts/test-minibasic.sh` vérifie direct mode, précédence, lignes hors
  ordre, remplacement/suppression implicite, LIST, DUMP, TRACE, FOR de dix
  itérations, binary64, erreur GOTO et INPUT avec deux valeurs positives puis
  une valeur négative.
- `scripts/test-hammurabi.sh` extrait le listing final du tutoriel, le saisit
  réellement dans le guest et vérifie cinq années de jeu sans sortie
  préenregistrée.
- Une réponse `INPUT` vide ou invalide doit produire `BASIC-INPUT-001` et
  laisser l’invite et le programme utilisables.
- Les identifiants d’une à 16 lettres/chiffres/underscore, la casse
  insensible, la création implicite à zéro, la limite de 64 noms et le rejet
  d’un 17e caractère sont vérifiés sur la cible.
- `scripts/test-guest-fdiv.sh` vérifie indépendamment l’encodage, l’exécution
  et le motif de `fdiv.d`.
- `scripts/test-guest-runtime-lexer.sh` vérifie que la lecture, le stockage et
  la reconnaissance du mot-clé `PRINT` sont exécutés par un payload assembleur
  cible, avec sortie produite par `write-buffer`.
- `scripts/test-guest-runtime-string.sh` vérifie une première primitive de
  chaîne cible : lecture UART de `Hammurabi`, stockage ASCII en RAM cible,
  descripteur `{data_addr,length,capacity}` observable et restitution par
  `write-buffer`, avec statut QEMU réel.
- `scripts/test-guest-runtime-string-lexer.sh` vérifie dans le payload la
  reconnaissance d’un identifiant chaîne de 16 caractères et de `DIM A(10)`,
  la copie target-side du littéral, le descripteur 24-octets et la sortie de
  classification du tableau.
- `scripts/test-guest-runtime-array.sh` vérifie séparément la construction et
  la relecture target-side d’un `ArrayDesc` de rang 1 pour onze éléments
  `binary64`, l’écriture/lecture de `A(10)`, le calcul row-major et le refus
  target-side de l’index 11, avec dump octet par octet sous QEMU.
- `scripts/test-guest-runtime-expression.sh` vérifie la lecture target-side de
  `2+3*4`, la précédence `*` avant `+`, `fmul.d`, `fadd.d`, le motif `14.0`,
  `fcsr` et le stockage du résultat en RAM.
- `scripts/test-guest-runtime-expression-div.sh` vérifie `22/7`, la conversion
  target-side des chiffres, `fdiv.d`, le motif exact du quotient, `fflags.NX`
  et le stockage binary64.
- `scripts/test-guest-runtime-expression-digits.sh` vérifie le parcours
  target-side d’un littéral entier multi-chiffres (`12`), la construction base
  10 dans des registres entiers, les conversions `fcvt.d.l`, puis le résultat
  `12+(3*4)=24.0` produit par `fmul.d` et `fadd.d`.
- `scripts/test-guest-runtime-number.sh` vérifie un littéral décimal signé
  (`-12.5`) : signe, partie entière, fraction, diviseur puissance de dix,
  `fdiv.d`, `fadd.d`, `fsub.d` et motif `binary64` exact dans la cible.
- `scripts/test-guest-runtime-variable.sh` vérifie une affectation numérique
  target-side `X=12.5`, la validation `A..Z`, l’offset `23*8` dans une table de
  26 `binary64`, l’écriture/relecture et le motif mémoire exact.
- `scripts/test-guest-runtime-lines.sh` vérifie l’insertion target-side de deux
  enregistrements fixes saisis hors ordre (`20` puis `10`), le déplacement du
  premier enregistrement, le compteur et les corps ASCII dans la RAM cible.
- `scripts/test-guest-runtime-line-lexer.sh` vérifie le parcours target-side de
  `20 PRINT B`, l’accumulation du numéro, la copie du corps ASCII et la longueur
  produite dans l’enregistrement cible.
- `scripts/test-guest-runtime-line-input.sh` vérifie deux lignes parsées dans
  la cible (`20 PRINT B`, puis `10 PRINT A`), l’insertion ordonnée, le déplacement
  complet du corps, les longueurs et le compteur.
- `scripts/test-guest-runtime-line-edit.sh` vérifie le remplacement du corps
  de la ligne 20, la suppression de la ligne 10 par compactage, le compteur 1
  et le record `20 PRINT C` dans la RAM cible.
- `scripts/test-guest-runtime-list.sh` vérifie le parcours target-side de deux
  records, la conversion décimale des numéros, la copie des corps et la sortie
  `10 PRINT A` / `20 PRINT B` produite par `write-buffer`.
- `scripts/test-guest-runtime-line-uart.sh` vérifie une ligne réelle fournie
  par l’UART (`20 PRINT B`), la lecture `read-char`, le stockage target-side
  jusqu’au LF et la restitution par `write-buffer`.
- `scripts/test-guest-runtime-repl-line.sh` vérifie le chemin vertical UART →
  buffer → parsing du numéro/corps → record cible, puis l’inspection de la RAM
  après retour au moniteur.
- `scripts/test-guest-runtime-repl-two-lines.sh` vérifie deux tours UART dans
  le même payload, l’insertion de `10 PRINT A` avant `20 PRINT B`, les deux
  corps intacts et le compteur target-side égal à 2.
- `scripts/test-guest-runtime-command-list.sh` vérifie la réception de `LIST`,
  sa reconnaissance target-side caractère par caractère, le dispatch vers le
  parcours des records et la sortie `10 PRINT A` / `20 PRINT B`.
- `scripts/test-guest-runtime-command-new.sh` vérifie la réception de `NEW`,
  l’effacement target-side du compteur et des records, la réponse `NEW OK` et
  les dumps mémoire nuls après la commande.
- `scripts/test-guest-runtime-command-loop.sh` vérifie qu’une commande inconnue
  produit `ERR UNKNOWN` sans tuer le payload, puis que `NEW` est reçu et exécuté
  dans le tour suivant.
- `scripts/test-guest-runtime-prompt.sh` vérifie deux émissions target-side de
  `READY> `, la continuité après `BOGUS` et l’exécution de `NEW` au tour suivant.
- `scripts/test-guest-runtime-command-run-print.sh` vérifie la réception de
  `RUN`, la lecture du record `10 PRINT 2+3`, le calcul target-side et la sortie
  `5` par `write-buffer`.
- `scripts/test-guest-runtime-command-run-fadd-d.sh` vérifie `RUN` avec le
  même record, `fcvt.d.l`, `fadd.d`, le motif `5.0` dans `f3`, le breakpoint et
  l’écriture binary64 exacte en RAM cible.
- `scripts/test-guest-runtime-command-run-variable.sh` vérifie que `RUN` lit
  `PRINT X` dans un record target-side, adresse `X` à l’offset `23*8` de la
  table numérique, charge réellement sa valeur par `fld` dans `f1` et expose
  le motif binary64 exact au breakpoint et en mémoire.
- `scripts/test-guest-runtime-command-run-variable-add.sh` vérifie `PRINT X+3`:
  lecture de `X` par `fld`, conversion target-side du littéral par `fcvt.d.l`,
  exécution `fadd.d`, motif exact `8.0` dans `f3` et en mémoire.
- `scripts/test-guest-runtime-command-run-variable-sub.sh` vérifie `PRINT X-3`,
  le chargement de `X`, `fcvt.d.l`, l’exécution `fsub.d` et le motif exact
  `2.0` dans `f3` et en mémoire.
- `scripts/test-guest-runtime-command-run-variable-mul.sh` vérifie `PRINT X*3`,
  le chargement de `X`, `fcvt.d.l`, l’exécution `fmul.d` et le motif exact
  `15.0` dans `f3` et en mémoire.
- Le breakpoint `minibasic_divide` et `disasm` permettent d’observer la
  correspondance adresse → `fdiv.d`; les registres f et `fcsr` sont affichés
  avant/après le pas.

## Matrice négative

Les cas à maintenir couvrent ligne trop longue, programme plein, division par
zéro, syntaxe, GOTO/THEN absent, STEP nul, huit frames FOR, neuvième frame,
boucle infinie et Ctrl-C. Chaque erreur conserve son code et revient à
`READY>` sans perdre le programme.

## Extensions conservées à couvrir

Les tests de release devront ajouter un corpus cible pour les chaînes et les
tableaux complets : variables chaînes, affectation, affichage, limites de
longueur, tableaux numériques et tableaux de chaînes, index hors limites,
dimensions invalides, consommation de la zone de données et restauration par
snapshot. Les résultats devront provenir du payload RV, jamais d’un
interpréteur hôte. Le layout à tester est fixé par D-018 : chaîne vide, copie
sans écriture partielle, longueur/capacité maximales, `DIM A(10)` et index 0/10,
tableaux row-major de `binary64` et de `StringDesc`, dépassement de pool,
produit de dimensions en débordement et restauration par snapshot.

## Oracle indépendant

Les valeurs d’expressions simples peuvent être comparées à une référence
IEEE 754 hôte dans les tests, mais cette référence ne participe jamais à
l’exécution cible. La preuve d’exécution est QEMU et la présence de `fdiv.d`
est contrôlée par désassemblage de l’ELF cible.

## Écart explicitement suivi

Le moteur fonctionnel est actuellement compilé depuis `minibasic.rs`. Le port
de son cœur vers une source assembleur acceptée par le parseur guest et son
chargement par `assemble-program` constituent le jalon assembleur restant.
