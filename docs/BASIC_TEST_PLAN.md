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
- `scripts/test-guest-runtime-command-run-variable-div.sh` vérifie `PRINT X/2`,
  le chargement de `X`, `fcvt.d.l`, l’exécution `fdiv.d` et le motif exact
  `2.5` dans `f3` et en mémoire.
- `scripts/test-guest-runtime-command-run-variable-divzero.sh` vérifie
  `PRINT X/0` dans la cible : `+inf`, `fcsr=0x8` (`fflags.DZ`) et le motif
  binary64 exact en mémoire, avant l’ajout du diagnostic BASIC.
- `scripts/test-guest-runtime-command-run-variable-divzero-diagnostic.sh`
  vérifie l’émission target-side de `ERR DIV0` par `write-buffer` et le retour
  propre avec statut 0 après `fdiv.d` par zéro.
- `scripts/test-guest-runtime-command-run-variable-zerozero.sh` vérifie
  `0.0/0.0` : NaN quiet canonique `0x7ff8000000000000` et `fcsr=0x10`, donc
  `fflags.NV` au bit 4 selon RISC-V.
- `scripts/test-guest-runtime-command-run-variable-negative-div.sh` vérifie la
  conservation du signe avec `X=-5.0`, `PRINT X/2` et le motif exact `-2.5`
  (`0xc004000000000000`) dans `f3` et en mémoire.
- `scripts/test-guest-runtime-command-run-variable-negative-zero-div.sh`
  vérifie `X=-0.0`, `PRINT X/2`, le motif exact `0x8000000000000000` et
  l’absence de flag flottant (`fcsr=0`).
- `scripts/test-guest-runtime-command-run-variable-negative-zero-denominator.sh`
  vérifie `+5.0/-0.0`, `-inf` (`0xfff0000000000000`) et `fcsr=0x8`
  (`fflags.DZ`).
- `scripts/test-guest-runtime-command-run-two-variables-add.sh` vérifie deux
  lectures de table (`X` dans `f1`, `Y` dans `f2`), `fadd.d` et le résultat
  exact `8.0` (`f3=0x4020000000000000`) dans la cible.
- `scripts/test-guest-runtime-asm-repl.sh` vérifie la première tranche intégrée
  assemblée par le moniteur : invite, saisie target-side de `10 PRINT X+Y`,
  `LIST`, `RUN`, deux variables dans `f1/f2`, `fadd.d` et résultat en RAM.
- `scripts/test-guest-runtime-asm-repl-mul.sh` vérifie le dispatch de l’opérateur
  stocké vers `fmul.d`, avec `X=5.0`, `Y=3.0` et `15.0` dans `f3`/RAM.
- `scripts/test-guest-runtime-asm-repl-literal.sh` vérifie le décodage de deux
  chiffres target-side (`2+3`), leurs conversions `fcvt.d.l`, `fadd.d` et le
  motif exact `5.0` dans `f3`/RAM.
- `scripts/test-guest-runtime-asm-repl-decimal.sh` vérifie le stockage de la
  longueur réelle du corps, `2.5+3.5`, les conversions/fragments décimaux
  target-side et le résultat exact `6.0` dans `f3`/RAM.
- `scripts/test-guest-runtime-asm-repl-direct.sh` vérifie `PRINT 2+3` sans
  numéro de ligne et la réutilisation target-side du chemin d’évaluation.
- `scripts/test-guest-runtime-asm-repl-question.sh` vérifie l’alias `?2+3`,
  sa normalisation target-side et le même résultat binary64 exact `5.0`.
- `scripts/test-guest-runtime-asm-repl-assignment.sh` vérifie `X=7`, la
  mutation target-side de la table, puis `PRINT X+3` avec `10.0` dans `f3` et
  la valeur `7.0` conservée en mémoire.
- `scripts/test-guest-runtime-asm-repl-multidigit.sh` vérifie le scanner
  target-side multi-chiffres et les espaces avec `PRINT 12.5 + 3.5`, les
  motifs exacts des deux opérandes et le résultat `16.0` dans `f3` et en RAM.
- `scripts/test-guest-runtime-asm-repl-unary-paren.sh` vérifie les signes
  unaires et les parenthèses simples avec `PRINT (-2.5) + (+3.5)`, dont les
  motifs exacts `f1=-2.5`, `f2=3.5` et `f3=1.0` sont produits dans la cible.
- `scripts/test-guest-runtime-asm-repl-precedence.sh` vérifie les deux niveaux
  target-side avec `PRINT 2+3*4`, `fmul.d` avant `fadd.d`, `f1=2.0`, `f2=4.0`
  et `f3=14.0` dans la cible.
- Les tests intégrés vérifient également l’émission target-side des résultats
  fixes (`8.000000`, `6.000000`, `1.000000`, `14.000000`) par le service
  `ecall 4`, indépendamment de l’inspection des motifs binaires.
- `scripts/test-guest-runtime-asm-repl-format-negative-fraction.sh` vérifie
  `-2.25`, la sortie `-2.250000`, la restauration du motif signé de `f3` au
  breakpoint et l’octetage exact en RAM.
- `scripts/test-guest-runtime-asm-repl-two-lines.sh` vérifie l’insertion hors
  ordre de `20` puis `10`, le tri observable de `LIST`, les deux slots mémoire
  et l’exécution séquentielle target-side des lignes `10` puis `20`.
- `scripts/test-guest-runtime-asm-repl-goto.sh` vérifie `10 GOTO 20`, le
  transfert target-side vers le slot 20, `5.000000` et le breakpoint final.
- `scripts/test-guest-runtime-asm-repl-end.sh` vérifie le dispatch target-side
  de `10 END`, la sortie `END` et l’arrêt contrôlé au breakpoint.
- `scripts/test-guest-runtime-fcmp-d.sh` vérifie l’assemblage généré et
  l’exécution QEMU de `feq.d`, `flt.d` et `fle.d`, avec `rd=xN` et les résultats
  booléens exacts en registres et en RAM.
- `scripts/test-guest-runtime-asm-repl-if.sh` vérifie `10 IF 1<2 THEN 20`, la
  comparaison dans le guest, le saut vers le slot 20, `15.000000` et le
  breakpoint final.
- `scripts/test-guest-runtime-asm-repl-if-false.sh` vérifie `2<1`, l’absence
  de sortie du slot inexistant et l’arrêt target-side sans branche forcée.
- `scripts/test-guest-runtime-asm-repl-input.sh` vérifie l’invite, la lecture
  UART de `3.5`, la conversion target-side vers `X`, puis `PRINT X*X` avec
  `12.250000` et les motifs FP exacts.
- `scripts/test-guest-runtime-asm-repl-trace.sh` vérifie `[10]` et `[20]` avant
  deux lignes exécutées ; `scripts/test-guest-runtime-asm-repl-break.sh`
  vérifie Ctrl-C sur `10 GOTO 10`, `BREAK` et le retour au moniteur.
- `scripts/test-guest-runtime-asm-repl-string.sh` vérifie deux littéraux ASCII
  `PRINT "FIRST"`/`PRINT "SECOND"`, lus en RAM cible et émis par `ecall 1`.
- `scripts/test-guest-runtime-asm-repl-for.sh` vérifie la première boucle de
  contrôle itérative entièrement target-side : `FOR X=1 TO 3` initialise la
  variable, `NEXT X` incrémente par `fadd.d`, compare par `fle.d` et produit
  `1.000000`, `2.000000`, puis `3.000000` avant l'arrêt.
- `scripts/test-guest-runtime-asm-repl-for-y.sh` vérifie le même chemin avec
  `Y`, afin de prouver que l'emplacement de la variable de contrôle n'est pas
  implicite dans le dispatcher.
- `scripts/test-guest-runtime-asm-repl-for-step.sh` vérifie `STEP 2`, un pas
  négatif (`STEP -1`) et le refus de `STEP 0`; les trois cas sont exécutés dans
  la cible et le dernier doit produire `ERR`.
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
