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
- Le breakpoint `minibasic_divide` et `disasm` permettent d’observer la
  correspondance adresse → `fdiv.d`; les registres f et `fcsr` sont affichés
  avant/après le pas.

## Matrice négative

Les cas à maintenir couvrent ligne trop longue, programme plein, division par
zéro, syntaxe, GOTO/THEN absent, STEP nul, huit frames FOR, neuvième frame,
boucle infinie et Ctrl-C. Chaque erreur conserve son code et revient à
`READY>` sans perdre le programme.

## Oracle indépendant

Les valeurs d’expressions simples peuvent être comparées à une référence
IEEE 754 hôte dans les tests, mais cette référence ne participe jamais à
l’exécution cible. La preuve d’exécution est QEMU et la présence de `fdiv.d`
est contrôlée par désassemblage de l’ELF cible.

## Écart explicitement suivi

Le moteur fonctionnel est actuellement compilé depuis `minibasic.rs`. Le port
de son cœur vers une source assembleur acceptée par le parseur guest et son
chargement par `assemble-program` constituent le jalon assembleur restant.
