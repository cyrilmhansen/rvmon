# Tutoriel : RVMonitor exécuté dans QEMU

Ce parcours est le parcours prioritaire du projet. `luna-guest-monitor` est
cross-compilé sur l’hôte, chargé par `qemu-system-riscv64`, puis exécuté dans
QEMU en M-mode. Il supervise sur le même hart un petit programme U-mode. La
conversation passe par l’UART virtuelle `virt` et non par GDB.

```text
machine hôte                         machine QEMU
-------------------------------      -------------------------------
cargo build --target ...         -->  RVMonitor M-mode
qemu-system-riscv64 <UART>           └─ programme cible U-mode
terminal                           <-- commandes et diagnostics UART
```

Ce binaire invité est actuellement un moniteur de démarrage et de débogage
minimal. Il fournit déjà l’inspection des registres, les vues mémoire hex/ASCII,
les directives de données exactes et des commandes `assemble` et
`assemble-program` limitées à `addi`, `lui`, `beq`, `bne`, `jal`, `jalr`, `ld`,
`sd`, `fadd.s`, `fadd.d`, `fmv.w.x` et `fmv.x.w`. Les vues avancées, les
watchpoints, l’historique et les snapshots restent à porter. Le programme U-mode de
démonstration est lié dans l’image et sert à valider les traps, les
breakpoints logiciels et le pas-à-pas.

## 1. Préparer les outils

Depuis la racine du dépôt, vérifier les outils nécessaires :

```text
$ rustup target add riscv64gc-unknown-none-elf
$ riscv64-linux-gnu-nm --version
$ qemu-system-riscv64 --version
```

Le backend invité ne dépend pas d’un service QEMU externe ni d’un port TCP.
L’option `-nographic` relie la console UART à la sortie standard du terminal.

## 2. Construire et démarrer le moniteur dans QEMU

Construire l’ELF bare-metal :

```text
$ cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf
```

Démarrer QEMU sans BIOS, avec l’ELF comme noyau :

```text
$ qemu-system-riscv64 \
    -M virt \
    -m 64M \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -nographic
```

La sortie initiale ressemble à ceci :

```text
RVMonitor 4B M-mode
target: RV64 ILP32D U-mode, hart=1, C=off
capabilities: I M F D Zicsr Zifencei
target workspace: 0x0000000081000000..0x0000000081010000
target data: 0x0000000082000000..0x0000000082100000
target: entering U-mode
trap: breakpoint pc=0x000000008000....
rvmonitor>
```

Au démarrage, le moniteur active le FIFO RX/TX du NS16550 virtuel avec le
seuil minimal d'un octet. Le pilote reste volontairement simple et lit les
octets par polling, mais les caractères reçus peuvent désormais être
tamponnés par le périphérique pendant que le moniteur traite une commande.
Le moniteur les regroupe ensuite dans un tampon logiciel de 64 octets, utilisé
par la console et par les payloads binaires. Les snapshots peuvent employer
`snapshot patchrle <region> <offset> <taille-brute> <taille-compressee>` ; le
format est une suite de paires `(longueur, octet)`, avec longueurs sur un octet.
Le chemin `patchbin` reste disponible pour les blocs non compressibles.
Pour les scripts ou un autre hôte, attendre l'invite `rvmonitor> ` avant
d'envoyer la première commande : envoyer des octets dès le lancement de QEMU
peut sinon les soumettre avant l'initialisation du périphérique.

Le premier `ebreak` du programme U-mode arrête volontairement la cible. Le
PC exact dépend du placement final de l’image ; il faut utiliser l’adresse
affichée par QEMU ou celle calculée avec `nm`, jamais supposer une adresse
fixe dans un tutoriel automatisable.

## 3. Commandes disponibles dans le guest

La commande `help` affiche la grammaire actuellement implémentée :

```text
rvmonitor> help
help/? regs/registers set <xreg> <hex64> setf <freg> <hex64> memory <addr> <length> edit <addr> <hex-bytes> data <addr> <directive> <bits> undo assemble <addr> <instruction> assemble-program <addr> ... end symbols disasm <addr|label> <count> step/s run <count> continue/c break <addr|label> delete <n> info break quit/q
```

### Lire les registres

```text
rvmonitor> regs
pc=0x000000008000.... mepc=0x000000008000.... mcause=0x0000000000000003 mtval=0x000000008000....
mstatus=0x........ fcsr=0x0000000000000000
integer registers:
x0=0x0000000000000000  x1=0x0000000000000001  ... x31=0x0000000000000000
floating registers (raw bits):
f0=0x0000000000000000  ... f31=0x0000000000000000
```

Les 32 registres entiers et les 32 registres flottants sont affichés sous
forme hexadécimale exacte. Les CSR de trap (`mepc`, `mcause`, `mtval`,
`mstatus`) et `fcsr` sont également visibles. Les registres flottants sont
présentés comme motifs binaires, sans conversion décimale dépendante de
l’hôte.

### Lire la mémoire

```text
rvmonitor> memory 0x80000000 16
0x0000000080000000: 17 .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. |................|
```

La longueur est décimale et limitée à 128 octets par commande. La lecture
doit rester dans la RAM cible `[0x80000000, 0x84000000)`. Les adresses MMIO,
les dépassements et les longueurs nulles sont refusés.

### Modifier et annuler la mémoire

`edit` reçoit une suite d’octets hexadécimaux, au maximum 32 octets. Toute la
plage est validée avant écriture ; le guest conserve une seule transaction
annulable. `undo` vérifie que les octets n’ont pas changé entre-temps avant de
restaurer l’original :

```text
rvmonitor> edit 0x80010040 deadbeef
edited 4 byte(s) at 0x0000000080010040
rvmonitor> memory 0x80010040 4
0x0000000080010040: de ad be ef                                     |....|
rvmonitor> undo
undone 4 byte(s) at 0x0000000080010040
```

Une nouvelle édition remplace l’annulation précédente. Les assemblages et les
breakpoints invalident également cette annulation afin de ne jamais restaurer
silencieusement des octets qui ne sont plus ceux de la transaction.

### Écrire des directives de données exactes

`data` écrit une valeur unique dans la zone de données cible
`[0x82000000,0x82100000)`, en little-endian. `.float` et
`.double` prennent les bits binary32/binary64 ; `.binary16` et `.binary128`
prennent respectivement 4 et 32 chiffres hexadécimaux représentant le motif
IEEE dans l’ordre numérique, puis l’écrivent en little-endian :

```text
rvmonitor> data 0x82000060 .float 0x3f800000
stored .float at 0x0000000082000060 (4 byte(s))
rvmonitor> memory 0x82000060 4
0x0000000082000060: 00 00 80 3f                         |...|
rvmonitor> data 0x82000060 .binary128 000102030405060708090a0b0c0d0e0f
stored .binary128 at 0x0000000082000060 (16 byte(s))
```

Les directives disponibles dans cette tranche sont `.byte`, `.half`, `.word`,
`.dword`, `.binary16`, `.float`, `.double` et `.binary128`. Les valeurs
décimales sont acceptées pour les entiers ; les formats flottants exigent un
motif hexadécimal exact, sans conversion flottante dépendante de l’hôte.

### Assembler et exécuter `addi`

Le premier cycle source → encodage → RAM → pas-à-pas dans le guest est :

```text
rvmonitor> assemble 0x80001000 addi x1,x0,1
assembled instruction at 0x0000000080001000 = 0x0000000000100093
rvmonitor> step
step: temporary breakpoint restored
trap: breakpoint pc=0x0000000080001004
rvmonitor> regs
... x1=0x0000000000000001 ...
```

Cette première commande accepte uniquement la forme `addi <registre>,<registre>,<immédiat>`
avec registres `x0` à `x31` et immédiat signé dans `[-2048, 2047]`. L’écriture
est refusée si l’adresse est hors RAM ou occupée par un breakpoint actif. Le
portage de l’assembleur complet réutilisera le crate `luna-isa-core` généré
depuis R2, sans table d’opcodes spécifique au guest.

Pour plusieurs instructions, `assemble-program` ouvre un mode source borné.
Chaque ligne est validée avant toute écriture ; `end` termine la saisie :

```text
rvmonitor> assemble-program 0x80010a30
source mode: enter integer/control, lui/auipc, ld/sd, fadd.s/fadd.d or fmv lines, finish with end
source> _start:
source> addi x1,x0,1
source> beq x1,x1,next
source> addi x1,x0,99
source> next:
source> addi x1,x1,2
source> end
assembled program: 4 instruction(s) at 0x0000000080010a30
rvmonitor> symbols
symbols:
  0x0000000080010a30 _start
  0x0000000080010a3c next
rvmonitor> disasm _start 4
0x0000000080010a30: 0000000000100093 <_start>  addi x1,x0,1
0x0000000080010a34: 0000000000108463  beq x1,x1,next
0x0000000080010a38: 0000000006300093  addi x1,x0,99
0x0000000080010a3c: 0000000000208093 <next>  addi x1,x1,2
rvmonitor> step
trap: breakpoint pc=0x0000000080010a34
rvmonitor> step
trap: breakpoint pc=0x0000000080010a3c
rvmonitor> step
trap: breakpoint pc=0x0000000080010a40
rvmonitor> regs
... x1=0x0000000000000003 ...
```

Une erreur de source multi-ligne expose un code stable et la ligne fautive.
La validation est atomique : le programme déjà présent dans la fenêtre reste
inchangé si une ligne est rejetée.

```text
rvmonitor> assemble-program 0x81000100
source> addi x1,x0,7
source> not-an-instruction x1
source> end
error [GUEST-ASM-008] source line 2: supports integer/control, ld/sd, fadd.s/fadd.d or fmv syntax
```

Le parseur invité accepte `addi`, `lui`, `auipc`, `beq`, `bne`, `jal`, `jalr`, `ld`, `sd`,
`fadd.s` et `fadd.d`. Les branches et `jal` prennent une cible relative numérique ou un
label, éventuellement suivi de `+offset` ou `-offset`; `jalr` utilise la forme
`jalr rd,imm(rs1)`. Les instructions flottantes utilisent
`fadd.[s|d] fd,fs1,fs2` et acceptent éventuellement un mode d’arrondi numérique
`0..7` en quatrième opérande.
Le tampon accepte au maximum 16 lignes de 96 caractères et huit labels. Les
labels ASCII (`a-z`, chiffres, `_`, `.` et `$`) occupent l’adresse courante
sans produire d’instruction. L’adresse de l’exemple doit correspondre à une
zone RAM libre, qui ne recouvre ni le code, ni la pile, ni les données du
moniteur ; le test E2E la calcule à partir de `_target_workspace_start` avec
`nm`.

### Exécuter une instruction

À l’arrêt sur un `ebreak`, `step` exécute l’instruction courante et installe
un breakpoint temporaire à l’adresse de reprise :

```text
rvmonitor> step
step: temporary breakpoint restored
trap: breakpoint pc=0x000000008000....
rvmonitor> regs
pc=0x000000008000.... x1=0x0000000000000002 ...
```

Le pas-à-pas actuel couvre le flux de contrôle et les opérations flottantes
nécessaires aux démonstrations. Une instruction non reconnue provoque un
diagnostic et laisse le moniteur en M-mode.

### Exécuter des flottants avec des motifs exacts

`setf` écrit directement les 64 bits sauvegardés d’un registre flottant. Cette
commande permet de préparer explicitement des opérandes NaN-boxés pour `.s` et
des motifs binary64 pour `.d` :

```text
rvmonitor> setf f1 0xffffffff3f800000
set f1=0xffffffff3f800000
rvmonitor> setf f2 0xffffffff40000000
set f2=0xffffffff40000000
rvmonitor> assemble 0x80001000 fadd.s f3,f1,f2
assembled instruction at 0x0000000080001000 = 0x00000000002081d3
rvmonitor> step
rvmonitor> regs
... f3=0xffffffff40400000 ... fcsr=0x0000000000000000 ...
```

Pour `.d`, écrire par exemple `0x3ff0000000000000` et
`0x4000000000000000` dans `f4` et `f5`; le résultat `f6` de
`fadd.d f6,f4,f5` est `0x4008000000000000`. L’affichage reste le motif brut,
indépendant du format décimal de l’hôte. Un résultat `.s` est NaN-boxé avec
les 32 bits hauts à `0xffffffff`.

### Continuer l’exécution

```text
rvmonitor> continue
trap: breakpoint pc=0x000000008000....
```

`continue` reprend depuis le point d’arrêt courant. Si l’arrêt ne correspond
pas à un état de breakpoint exploitable, la commande est refusée :

```text
error: target is not stopped at a breakpoint
```

Pour exécuter un nombre borné d’instructions, utiliser `run <count>`. Le
budget est décrémenté à chaque instruction retirée. Un breakpoint permanent,
un `ebreak` réel ou un trap interrompt le run avant l’épuisement ; sinon le
moniteur revient au prompt avec `run: budget exhausted`.

```text
rvmonitor> assemble-program 0x81000100
source> addi x1,x0,1
source> addi x1,x1,1
source> end
rvmonitor> run 2
trap: breakpoint pc=0x0000000081000108
run: budget exhausted
rvmonitor> regs
... x1=0x0000000000000002 ...
```

Les budgets `0` et supérieurs à `100000` sont refusés avec
`GUEST-RUN-003`.

### Consulter et corriger le source

Après un `assemble-program` réussi, le guest conserve jusqu’à 16 lignes du
dernier programme et son adresse de chargement. `source` affiche le document,
`source <n>` une ligne, et `source replace <n> "<texte>"` modifie uniquement le
buffer. La mémoire cible n’est réécrite qu’après `assemble-source`.

```text
rvmonitor> source
1 | addi x1,x0,1
2 | addi x1,x1,2
rvmonitor> source replace 2 "addi x1,x1,5"
source line 2 updated; use assemble-source to apply
rvmonitor> assemble-source
assembled source: 2 instruction(s) at 0x00000000810001c0
```

Une ligne invalide ou une correction hors plage est rejetée avec un code
`GUEST-SOURCE-*`, sans modification de la cible.

### Snapshots et projets pendant la session

Le guest ne dispose pas d’un système de fichiers. `snapshot save` conserve donc
un slot volatil en RAM : le `TargetContext`, le workspace de 64 KiB, la région
de données de 1 MiB, le source, les symboles, les breakpoints et les
watchpoints. `snapshot restore` restitue cet état. `project-save` et
`project-load` sont les alias pédagogiques du même slot.

```text
rvmonitor> set x1 0x7
rvmonitor> snapshot save
snapshot saved (workspace=65536 data=1048576)
rvmonitor> set x1 0x99
rvmonitor> snapshot restore
snapshot restored (workspace=65536 data=1048576)
rvmonitor> regs
... x1=0x0000000000000007 ...
```

Le slot est perdu lors d’un reset ou de l’arrêt de QEMU. Il peut toutefois
être inspecté ou corrigé par petits blocs sur l’UART, ce qui permet à un hôte
d’archiver ou de reconstruire progressivement les deux régions capturées :

```text
rvmonitor> snapshot info
snapshot: valid workspace=65536 data=1048576 source-lines=1 chunk-max=4096
rvmonitor> snapshot manifest
snapshot-manifest format=RVSNAP01 workspace-size=65536 data-size=1048576 source-lines=1 workspace-crc32=0x... data-crc32=0x... chunk-max=4096
rvmonitor> snapshot dump data 112 4
snapshot-chunk data offset=112 length=4 hex=44332211
rvmonitor> snapshot patch data 112 aabbccdd
snapshot chunk patched data offset=112 length=4
rvmonitor> snapshot restore
snapshot restored (workspace=65536 data=1048576)
```

`snapshot dump` accepte `workspace` ou `data`, un offset décimal et une
longueur de 1 à 4096 octets. `snapshot patch` accepte les mêmes régions et une
suite hexadécimale de 1 à 32 octets. Le patch ne touche pas la mémoire cible
active : il modifie uniquement le slot ; `snapshot restore` est nécessaire
pour l’appliquer. Cette tranche fournit le transport UART déterministe, pas
encore un fichier persistant, plusieurs slots ou la capture des 64 MiB
complets. `snapshot manifest` décrit le profil `RVSNAP01`, les tailles fixes
et un CRC-32 IEEE indépendant pour chaque région ; l’hôte doit comparer ces
valeurs après avoir transféré tous les blocs.

### Export depuis l’hôte par UART TCP

Pour automatiser le transfert depuis l’hôte, démarrer QEMU avec l’UART guest
sur un port TCP :

```sh
$ qemu-system-riscv64 -M virt -m 64M -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -display none -serial tcp:127.0.0.1:12353,server=on,wait=on
```

Dans un autre terminal :

```sh
$ cargo run -p luna-app -- \
    --guest-uart-port 12353 --snapshot-out session.rvsnap
guest snapshot exported to session.rvsnap (workspace=65536 data=1048576 source-lines=0)
```

L’application attend l’invite UART, exécute `snapshot save`, collecte les
blocs, contrôle le manifeste et écrit un fichier `RVSNAP01` déterministe.
Cette version exporte les régions mémoire et le nombre de lignes source ; les
registres, symboles et texte source ne sont pas encore sérialisés.
Le guest expose désormais le sous-format `RVMETA01` par
`snapshot metadata` puis `snapshot metadata dump <offset> <length>` ; son
intégration dans le fichier hôte reste la prochaine étape de cette famille.

Pour importer une image vérifiée dans le slot guest :

```sh
$ cargo run -p luna-app -- \
    --guest-uart-port 12353 --snapshot-in session.rvsnap
guest snapshot imported from session.rvsnap (workspace=65536 data=1048576 source-lines=0)
```

L’import décode et contrôle le fichier, initialise le slot par `snapshot save`
et vérifie la réponse avant d’envoyer le moindre patch. Il
utilise le canal binaire `snapshot patchbin` avec des blocs jusqu’à 4096 octets,
puis une seule commande `snapshot restore` rend l’ensemble actif ; une erreur
laisse donc le slot partiellement préparé mais ne modifie pas la cible active
avant la restauration. Le canal texte `snapshot patch` de 32 octets reste
disponible pour le dépannage manuel.

### Watchpoints logiciels

Le guest fournit des watchpoints logiciels sur les accès RV64 `ld` et `sd`.
Ils sont évalués avant l’exécution de l’instruction et acceptent une plage de
1 à 8 octets dans la RAM cible. `watch` surveille les écritures, `rwatch` les
lectures et `awatch` les deux directions.

```text
rvmonitor> set x4 0x0000000082000060
rvmonitor> watch 0x82000060 8
watchpoint #1 set at 0x0000000082000060 width=8 mode=write
rvmonitor> run 2
watchpoint #1 hit at pc=0x0000000081000184 address=0x0000000082000060 width=8
rvmonitor> memory 0x82000060 8
0x0000000082000060: 00 00 00 00 00 00 00 00                 |........|
rvmonitor> delete watch 1
watchpoint #1 deleted
```

Les watchpoints ne couvrent pas encore les accès MMIO, atomiques, les autres
largeurs ou des conditions d’expression. Ils sont implémentés par inspection
du décodage guest et ne constituent pas une fonctionnalité matérielle QEMU.

### Poser un breakpoint logiciel

Les breakpoints sont implantés en remplaçant temporairement le mot
d’instruction cible par l’encodage `ebreak` RV64. L’adresse doit être dans la
RAM cible, alignée sur quatre octets et exprimée comme adresse QEMU complète.

Pour obtenir l’adresse de `target_entry` et poser un breakpoint sur une
instruction ultérieure :

```text
$ entry_hex=$(riscv64-linux-gnu-nm -n \
    target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    | awk '$3 == "target_entry" { print $1; exit }')
$ printf 'target_entry=0x%s\n' "$entry_hex"
$ printf 'break-after-first-ebreak=0x%x\n' "$((16#$entry_hex + 12))"
```

Dans la console QEMU, utiliser l’adresse calculée :

```text
rvmonitor> break 0x80000...
breakpoint #1 set at 0x0000000080000...
rvmonitor> info break
breakpoints:
  #1 addr=0x0000000080000... original=0x........
rvmonitor> continue
trap: breakpoint #1 pc=0x0000000080000...
```

Le moniteur conserve au maximum quatre breakpoints permanents. Une adresse
déjà utilisée ou non alignée est refusée.

### Modifier un registre entier

À l’arrêt sur un trap, `set` écrit un motif hexadécimal complet dans `x1` à
`x31`. `x0` reste constant et toute écriture est refusée :

```text
rvmonitor> set x9 0x8000000080000000
set x9=0x8000000080000000
rvmonitor> set x0 0x1
error: x0 is read-only
```

La commande conserve les 64 bits du registre. Elle ne convertit pas la valeur
en pointeur : la convention ILP32 impose toujours au programme cible de
produire lui-même la forme signée attendue lorsqu’un pointeur 32 bits est
étendu dans un registre RV64.

### Supprimer un breakpoint

```text
rvmonitor> delete 1
breakpoint #1 deleted
```

La suppression restaure le mot d’instruction original et exécute une
barrière d’instructions. Un numéro nul, hors plage ou déjà libre produit une
erreur sans modifier la cible.

### Quitter

```text
rvmonitor> quit
bye
```

Dans la version actuelle, `quit` termine la commande mais ne peut pas arrêter
le processus QEMU depuis le guest. Pour arrêter QEMU, utiliser `Ctrl-C` dans
le terminal hôte.

## 4. Session complète reproductible

Le test E2E fourni automatise une session UART et vérifie les sorties :

```text
$ bash scripts/test-guest-monitor.sh
guest monitor QEMU end-to-end test passed
```

Le script :

1. construit l’ELF invité ;
2. calcule une adresse de breakpoint avec `riscv64-linux-gnu-nm` ;
3. démarre QEMU avec `-bios none`, `-kernel` et `-nographic` ;
4. envoie `help`, `regs`, `set`, `memory`, `edit`, `undo`, `break`, `info break`,
   `continue`, `step`, `delete` et `assemble-program` sur l’UART ;
5. vérifie les modifications de `x1`, les motifs exacts de `f3`/`f6`,
   les transferts `fmv.w.x`/`fmv.x.w` et leur NaN-boxing, `fcsr`, les
   encodages flottants et les diagnostics de trap.

Pour observer la même session manuellement, démarrer QEMU dans un terminal,
puis saisir les commandes une par une dans son terminal UART.

## 5. Ce qui n’est pas encore disponible dans ce mode

Les commandes suivantes appartiennent aujourd’hui au moniteur hôte ou au
simulateur interne, pas encore au binaire exécuté dans QEMU :

```text
watch, rwatch, history, project-save, project-load, snapshot, restore
```

Le cycle d’une ligne et le source buffer multi-ligne existent désormais pour
`addi`, les branches, les sauts, les loads/stores 64 bits et
`fadd.s`/`fadd.d` avec résolution de labels, ainsi que le désassemblage des
mots 32 bits. Les expressions générales, les macros, les directives dans le
source assembleur et les snapshots restent à venir, en conservant le moniteur
M-mode et le programme cible U-mode séparés. Les directives `data` documentées
plus haut sont déjà disponibles pour écrire une valeur isolée en mémoire.

## 6. Différence avec les deux autres parcours

| Parcours | Binaire qui exécute le moniteur | Transport | État cible actuel |
|---|---|---|---|
| Simulateur interne | `luna-app` sur l’hôte | aucun | Machine Rust déterministe |
| Console QEMU hôte | `luna-app` sur l’hôte | GDB RSP/TCP | QEMU distant, registres entiers + PC |
| Guest prioritaire | `luna-guest-monitor` dans QEMU M-mode | UART virtuelle | U-mode sur le même hart |

Le tutoriel général [TUTORIAL.md](TUTORIAL.md) couvre les deux premiers
parcours. Le présent document est la référence pour le troisième.
