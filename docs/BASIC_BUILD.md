# Construire et lancer MiniBASIC-RV

Depuis un checkout propre :

```text
rustup target add riscv64gc-unknown-none-elf
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf
qemu-system-riscv64 -M virt -m 64M -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor -nographic
```

À `rvmonitor>`, entrer `basic`. Le programme cible affiche `READY>`. Le smoke
réel complet est :

```text
bash scripts/test-minibasic.sh
```

Pour produire une transcription issue de QEMU :

```text
MINIBASIC_TRANSCRIPT=docs/BASIC_DEMO_TRANSCRIPT.txt bash scripts/test-minibasic.sh
```

Le build utilise le guest QEMU et les services `ecall` du moniteur ; aucun
interpréteur BASIC hôte n’est invoqué.

Pour inspecter le contrat du futur payload depuis la cible :

```text
rvmonitor> info payload
```

Cette commande est informative et ne modifie ni les registres ni la mémoire.

## Statut du chargement

Dans cette version, MiniBASIC est lié dans l’ELF guest et `basic` saute vers
`minibasic_entry`. Il s’exécute bien en U-mode, mais n’est pas encore chargé
depuis le workspace par l’assembleur du moniteur.

La première brique du futur chemin utilisateur est maintenant disponible :

```text
rvmonitor> assemble-program 0x81000100
source> addi x10,x0,65
source> addi x17,x0,1
source> ecall
source> addi x10,x0,0
source> addi x17,x0,3
source> ecall
source> end
rvmonitor> run-at 0x81000100
```

`run-at` lance ce payload U-mode déjà assemblé. Le contrat est décrit dans
[`GUEST_PAYLOAD_ABI.md`](GUEST_PAYLOAD_ABI.md) ; le remplacement de MiniBASIC
résident par un payload BASIC assembleur est une étape ultérieure.

Le squelette assembleur utilisateur, indépendant du runtime Rust résident,
peut être rejoué ainsi :

```text
bash scripts/test-guest-payload-skeleton.sh
```

La première primitive arithmétique du futur runtime assembleur est testée par
`bash scripts/test-guest-expression-d.sh`. Elle charge trois `binary64`,
exécute `fmul.d`, `fadd.d`, `fsub.d`, puis `fsd` et s’arrête sur `ebreak` pour
inspecter les bits exacts. Cette étape ne fournit pas encore le lexer BASIC ni
la conversion décimale générale ; elle prouve seulement que l’évaluation D
pourra être faite dans le payload cible.

La tranche suivante démontre une conversion target-side bornée pour les
valeurs positives finies, en six décimales fixes :

```text
bash scripts/test-guest-decimal-print.sh
```

Le payload calcule `22/7`, sépare partie entière et fractionnaire avec
`fcvt.l.d`/`fcvt.d.l`, remplit l’ASCII et l’envoie par `write-buffer`, ce qui
produit `3.142857` sans interpréteur ni conversion hôte. Les signes négatifs,
les valeurs particulières et le raccord au lexer BASIC restent à couvrir.

La fixture [`minibasic-runtime-isa.rv`](../examples/minibasic-runtime-isa.rv)
illustre la tranche assembleur suivante : lecture de bytes, opérations
bit-à-bit, accès mémoire et branchements exécutés dans le payload. Son
organisation est inspirée des modules séparés observés dans le désassemblage
de Turbo-BASIC XL, documenté dans [`BASIC_TBXL_NOTES.md`](BASIC_TBXL_NOTES.md),
mais le code et l’ABI restent propres à RVMonitor.

La fixture [`minibasic-runtime-lexer.rv`](../examples/minibasic-runtime-lexer.rv)
va un cran plus loin : elle lit réellement `PRINT` par `read_char`, range les
octets en RAM cible, reconnaît le mot-clé et écrit `OK` par `write-buffer`.
Elle est vérifiée par :

```text
bash scripts/test-guest-runtime-lexer.sh
```

La fixture [`minibasic-runtime-string.rv`](../examples/minibasic-runtime-string.rv)
ajoute la première primitive de chaînes du futur payload : elle lit une ligne
réelle, la stocke dans la RAM cible, écrit un descripteur de chaîne à une
adresse distincte et restitue le buffer par `write-buffer`.

```text
bash scripts/test-guest-runtime-string.sh
```

Elle ne contient ni résultat ni interpréteur hôte. Le layout destiné aux
variables chaînes et aux tableaux complets est fixé dans D-018 ; cette fixture
est une preuve de contrat mémoire et d’E/S, pas encore l’implémentation de
S$, DIM ou de l’accès indexé.

La fixture [`minibasic-runtime-string-lexer.rv`](../examples/minibasic-runtime-string-lexer.rv)
valide ensuite deux formes syntaxiques dans le payload : une affectation avec
un identifiant de 16 caractères et `DIM A(10)`. Le payload copie le littéral
`Hammurabi` vers un objet chaîne cible,
écrit son descripteur, puis reconnaît la forme de déclaration du tableau :

```text
bash scripts/test-guest-runtime-string-lexer.sh
```

Le test vérifie aussi le descripteur en RAM et le statut de sortie QEMU.

La fixture [`minibasic-runtime-array.rv`](../examples/minibasic-runtime-array.rv)
isole la construction d’un `ArrayDesc` pour `DIM A(10)` : pointeur de base,
11 éléments `binary64`, rang 1 et borne inclusive 10. Elle est vérifiée par :

```text
bash scripts/test-guest-runtime-array.sh
```

La même fixture vérifie également `A(10)` : elle calcule `base + 10 * 8`,
écrit puis relit le motif binary64 de `42.0`, et refuse l’index 11 avant tout
accès.

Le premier chemin d’expression target-side est disponible dans
[`minibasic-runtime-expression.rv`](../examples/minibasic-runtime-expression.rv).
Il reconnaît `2+3*4`, charge les opérandes dans les registres flottants, exécute
`fmul.d` puis `fadd.d`, et s’arrête sur `ebreak` pour rendre les bits et `fcsr`
inspectables :

```text
bash scripts/test-guest-runtime-expression.sh
```

Cette fixture prouve la précédence et l’exécution D, mais ne constitue pas
encore le parser général de MiniBASIC.

Le chemin de division target-side est vérifié séparément par
[`minibasic-runtime-expression-div.rv`](../examples/minibasic-runtime-expression-div.rv).
Le payload lit `22/7`, convertit les chiffres en binary64 avec `fcvt.d.l`,
exécute `fdiv.d` et vérifie le motif exact de `3.142857...` ainsi que
`fflags.NX` :

```text
bash scripts/test-guest-runtime-expression-div.sh
```

La fixture [`minibasic-runtime-expression-digits.rv`](../examples/minibasic-runtime-expression-digits.rv)
ajoute la lecture target-side d’un littéral entier à plusieurs chiffres. Elle
parcourt les caractères `12+3*4`, construit `12` dans un accumulateur entier
avec `valeur = valeur * 10 + chiffre`, puis convertit les trois opérandes en
`binary64` avant d’exécuter `fmul.d` et `fadd.d`. Le résultat attendu est
`12 + (3 * 4) = 24.0`, observable dans `f5` et en mémoire :

```text
bash scripts/test-guest-runtime-expression-digits.sh
```

Cette étape ne couvre pas encore les fractions décimales, les parenthèses, les
variables ni le parser général ; elle isole la conversion des chiffres et la
précédence déjà démontrée par la fixture précédente.

La fixture [`minibasic-runtime-number.rv`](../examples/minibasic-runtime-number.rv)
valide ensuite le lexer numérique target-side sur `-12.5` : signe, accumulation
de la partie entière, accumulation de la fraction, division par une puissance
de dix et application du signe par `fsub.d`. Le motif `binary64` négatif
`0xc029000000000000` est écrit en mémoire cible :

```text
bash scripts/test-guest-runtime-number.sh
```

Le test reste volontairement limité à un nombre décimal borné ; les exposants,
les débordements d’accumulateur, les parenthèses et les variables appartiennent
au lexer/parser général.

La fixture [`minibasic-runtime-variable.rv`](../examples/minibasic-runtime-variable.rv)
ajoute la première table de variables numériques en RAM cible. Elle lit
`X=12.5`, valide `X` dans `A..Z`, calcule l’offset `23 * 8`, écrit le
`binary64` dans la table de 26 cases puis le relit dans `f2` :

```text
bash scripts/test-guest-runtime-variable.sh
```

Cette preuve couvre la représentation mémoire et l’accès indexé d’une
variable ; elle ne fournit pas encore le magasin de lignes, les noms longs,
`LET` général ni le dispatch des instructions BASIC.

La fixture [`minibasic-runtime-lines.rv`](../examples/minibasic-runtime-lines.rv)
ouvre le magasin de lignes target-side. Elle utilise des enregistrements fixes
de 32 octets, insère d’abord la ligne 20 puis la ligne 10, déplace la première
et laisse la table dans l’ordre `10, 20` :

```text
bash scripts/test-guest-runtime-lines.sh
```

Le compteur et les corps ASCII sont observés directement dans la RAM cible.
`LIST`, suppression, remplacement et exécution séquentielle seront ajoutés
sur ce layout lors de la tranche de contrôle de flot.

La fixture [`minibasic-runtime-line-lexer.rv`](../examples/minibasic-runtime-line-lexer.rv)
relie maintenant le texte source au magasin : le payload parcourt les octets de
`20 PRINT B`, accumule le numéro 20, copie `PRINT B` dans le corps et renseigne
la longueur 7 dans le même enregistrement cible :

```text
bash scripts/test-guest-runtime-line-lexer.sh
```

Cette étape ne lit pas encore l’UART et ne gère qu’une ligne bornée ; elle isole
le contrat lexer → record avant l’insertion de plusieurs lignes.

La fixture [`minibasic-runtime-line-input.rv`](../examples/minibasic-runtime-line-input.rv)
enchaîne maintenant deux parsings target-side, `20 PRINT B` puis `10 PRINT A`.
Elle déplace le record complet, y compris le corps de 7 octets, et vérifie une
table ordonnée `10, 20` :

```text
bash scripts/test-guest-runtime-line-input.sh
```

Le compteur, les longueurs et les deux corps sont observables en RAM. La
lecture UART, `LIST`, suppression et remplacement restent à raccorder.

La fixture [`minibasic-runtime-line-edit.rv`](../examples/minibasic-runtime-line-edit.rv)
valide ensuite l’édition du magasin : elle remplace `PRINT B` par `PRINT C`,
supprime la ligne 10 par compactage, puis conserve une table d’une ligne 20 et
son compteur target-side :

```text
bash scripts/test-guest-runtime-line-edit.sh
```

La commande `LIST` et la capacité générale restent volontairement séparées de
ce test mémoire.

La fixture [`minibasic-runtime-list.rv`](../examples/minibasic-runtime-list.rv)
ajoute `LIST` target-side : elle parcourt les records ordonnés, convertit les
numéros en ASCII, copie les corps dans un buffer cible et appelle
`write-buffer`. Elle produit réellement :

```text
10 PRINT A
20 PRINT B
```

```text
bash scripts/test-guest-runtime-list.sh
```

La commande interactive `LIST`, les bornes de capacité et les lignes reçues par
UART seront raccordées au même parcours.

La fixture [`minibasic-runtime-line-uart.rv`](../examples/minibasic-runtime-line-uart.rv)
valide le premier raccord UART : le payload appelle `read-char` jusqu’au LF,
stocke `20 PRINT B` dans sa RAM, puis restitue exactement les octets par
`write-buffer` :

```text
bash scripts/test-guest-runtime-line-uart.sh
```

Cette primitive ne parse pas encore le numéro et ne l’insère pas dans la table ;
elle ferme d’abord le contrat transport → buffer target-side.

La fixture [`minibasic-runtime-repl-line.rv`](../examples/minibasic-runtime-repl-line.rv)
fusionne les étapes : elle reçoit `20 PRINT B` par UART, parse le numéro et le
corps dans la cible, remplit le premier record et rend ensuite la mémoire
inspectable par le moniteur :

```text
bash scripts/test-guest-runtime-repl-line.sh
```

La boucle REPL ne traite encore qu’une ligne par lancement ; la répétition,
`LIST` interactif et les commandes `NEW`/`RUN` restent à construire.

La fixture [`minibasic-runtime-repl-two-lines.rv`](../examples/minibasic-runtime-repl-two-lines.rv)
effectue deux tours UART dans le même payload : `20 PRINT B`, puis `10 PRINT A`.
Le compteur est conservé dans un registre dédié, la seconde ligne est insérée
avant la première et les deux records restent inspectables :

```text
bash scripts/test-guest-runtime-repl-two-lines.sh
```

La borne de deux tours est volontaire pour cette preuve ; la boucle persistante
et le dispatch des commandes seront ajoutés ensuite.

La fixture [`minibasic-runtime-command-list.rv`](../examples/minibasic-runtime-command-list.rv)
ajoute le premier dispatch textuel target-side : elle lit `LIST`, vérifie ses
quatre caractères dans la cible, puis appelle le parcours de listing sur la
table déjà présente. La sortie est produite par `write-buffer` :

```text
bash scripts/test-guest-runtime-command-list.sh
```

Les commandes invalides et `NEW` restent à traiter dans la boucle de commandes.

La fixture [`minibasic-runtime-command-new.rv`](../examples/minibasic-runtime-command-new.rv)
ajoute `NEW` target-side : elle valide la commande, efface le compteur et les
records dans la RAM cible, puis produit `NEW OK` par `write-buffer` :

```text
bash scripts/test-guest-runtime-command-new.sh
```

La commande ne persiste encore qu’un état borné à deux records ; la capacité
finale, les erreurs structurées et le retour à une invite persistante restent à
intégrer.

La fixture [`minibasic-runtime-command-loop.rv`](../examples/minibasic-runtime-command-loop.rv)
vérifie ensuite la persistance minimale de la boucle de commandes : `BOGUS`
produit `ERR UNKNOWN`, puis `NEW` est encore lu et exécuté, avec `NEW OK` et
compteur nul :

```text
bash scripts/test-guest-runtime-command-loop.sh
```

La boucle est encore bornée par le scénario de test ; l’invite `READY>`, le
dispatch de `RUN` et la reprise après erreur complète restent à intégrer.

La fixture [`minibasic-runtime-prompt.rv`](../examples/minibasic-runtime-prompt.rv)
ajoute l’invite target-side persistante : chaque tour produit `READY> `, puis
lit une commande. Le scénario vérifie deux invites, `ERR UNKNOWN`, puis `NEW OK` :

```text
bash scripts/test-guest-runtime-prompt.sh
```

L’invite n’est pas fournie par le moniteur hôte ; elle est écrite par le payload
par `write-buffer`.

La fixture [`minibasic-runtime-command-run-print.rv`](../examples/minibasic-runtime-command-run-print.rv)
ajoute le premier `RUN` target-side. Elle reçoit `RUN`, vérifie le record
`10 PRINT 2+3`, calcule `2+3` dans les registres entiers et produit `5` suivi
d’un LF depuis la cible :

```text
bash scripts/test-guest-runtime-command-run-print.sh
```

Cette étape est volontairement entière et bornée ; les expressions binary64,
les variables et le contrôle de flot seront raccordés ensuite.

La fixture [`minibasic-runtime-command-run-fadd-d.rv`](../examples/minibasic-runtime-command-run-fadd-d.rv)
évalue le même record avec l’extension D : `2` et `3` sont convertis par
`fcvt.d.l`, additionnés par `fadd.d`, puis le motif exact de `5.0` est écrit en
RAM cible et observé au breakpoint :

```text
bash scripts/test-guest-runtime-command-run-fadd-d.sh
```

Le formatage décimal de sortie et les variables restent distincts de cette
preuve arithmétique.
