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

Ce binaire invité est un moniteur de démarrage et de débogage minimal. Il
fournit déjà l’inspection des registres, les vues mémoire hex/ASCII, les
directives de données exactes, les watchpoints logiciels, les snapshots
volatils et des commandes `assemble` et `assemble-program` limitées à `addi`,
`lui`, `beq`, `bne`, `jal`, `jalr`, `ld`, `sd`, `add`, `sub`, `mul`, `div`,
`fadd.s`, `fadd.d`, `fdiv.d`, `ecall`, `ebreak`, `fmv.w.x` et `fmv.x.w`. La
trace arrière, les conditions de breakpoint et la persistance sur fichiers
restent à porter. Le programme U-mode de démonstration et MiniBASIC-RV sont
liés dans l’image et servent à valider les traps, les breakpoints logiciels et
le pas-à-pas.

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

Au démarrage, le moniteur configure le NS16550 virtuel en 8N1, active ses FIFO
RX/TX et son interruption de réception. Le PLIC QEMU achemine l'IRQ UART 10
vers le contexte M-mode du hart unique ; le handler vide le FIFO dans un
tampon logiciel de 4096 octets. Le polling reste un filet de sécurité aux
frontières des appels de service et dans la boucle de commandes M-mode. Les
caractères reçus sont ainsi tamponnés pendant que la cible ou le moniteur
traite une commande, sans partager le contexte de trap avec du code M-mode.
Le tampon logiciel est utilisé
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

### ABI des services U-mode

Un programme cible peut demander un service au moniteur M-mode avec `ecall`.
Le numéro de service est dans `a7` (`x17`) et les arguments/résultats dans
`a0` (`x10`) et `a1` (`x11`). Le moniteur avance `mepc` de 4 octets et reprend
le programme sans afficher de trap de débogage pour les services valides.

| `a7` | Service | Entrée | Résultat |
|---:|---|---|---|
| 1 | `write_char` | `a0` octet bas | aucun |
| 2 | `read_char` | aucune | `a0` octet reçu, bloquant |
| 3 | `exit` | `a0` code | arrêt au prompt avec le code affiché |
| 4 | `write_buffer` | `a0` adresse RV64, `a1` longueur ≤4096 | aucun |
| 5 | `poll_char` | aucune | `a0=0` si vide, sinon octet reçu |

Les adresses du service 4 doivent rester dans la RAM cible ; l’hôte n’est
jamais lu ou écrit directement. Le caractère ASCII Ctrl-C (`0x03`) est
retourné par `read_char` et constitue le mécanisme d’interruption coopératif
de MiniBASIC. `poll_char` est non bloquant et permet au programme cible de
vérifier périodiquement cette interruption. Un service inconnu produit
`GUEST-IO-002` et rend la main
au prompt.

## 3. Commandes disponibles dans le guest

Commencer chaque séance par `help`. Cette commande reste disponible après une
erreur et décrit les commandes réellement présentes dans l’image :

```text
rvmonitor> help
RVMonitor guest commands
  help/?                         this help
  basic                          load and launch assembly MiniBASIC-RV
  basic-rust                     launch legacy resident Rust MiniBASIC-RV
  regs|registers                 show integer/floating registers
  set <xreg> <hex64>             edit an integer register
  setf <freg> <hex64>            edit raw floating bits
  memory <addr> <length>         show target memory as hex/ASCII
  edit <addr> <hex-bytes>|undo   transactional memory edit/rollback
  data <addr> <directive> <bits> write exact data representation
  assemble <addr> <instruction> assemble one instruction
  assemble-program <addr> ... end  assemble a bounded source buffer
  assemble-source                 reassemble the edited source buffer
  source [line]|replace ...       inspect or edit source buffer
  symbols                         list source symbols
  disasm <addr|label> <count>    disassemble target words
  step|s, stepidp <count>         step with full register/stack trace
  run <count>                     execute with a bounded budget
  continue|c                      resume from a breakpoint
  break <addr|label>              software breakpoint
  watch|rwatch|awatch <addr> <width>  memory watchpoint
  info break|watch                list debugger objects
  delete <n>|delete watch <n>     remove debugger object
  snapshot save|restore|info|manifest  volatile state snapshot
  project-save|project-load      snapshot aliases
  quit|q                          leave the monitor command loop
Errors are non-fatal: read the code/message, correct the command, and retry.
```

Après toute ligne `error` ou `ERROR`, lire le code, revenir à un arrêt connu
avec `regs` ou `info break`, corriger la commande, puis la rejouer. Un message
`target is not stopped at a breakpoint` signifie que `set`, `step`, `continue`
ou un snapshot contextuel attend le prochain trap.

## Pas-à-pas instrumenté

`stepidp N` exécute au plus `N` instructions en suivant les PC réellement
calculés par les branches et appels. Après chaque instruction retraitée, le
moniteur affiche le PC avant/après, le flux (`sequential`, branche prise ou
non prise, appel, saut, retour), les 32 registres entiers, les 32 registres
flottants bruts, `fcsr`, les cinq bits `fflags`, puis une fenêtre de 16 octets
alignée sur `x2`. Lorsqu’une instruction écrit en mémoire, un second bloc de
16 octets centré sur l’écriture est affiché. La commande accepte `1..256` et
exige que la cible soit arrêtée.

Exemple minimal :

```text
rvmonitor> stepidp 5
stepidp[1/5] 0x...: ... addi ... -> pc=0x... flow=sequential
stepidp[3/5] 0x...: ... jal ... -> pc=0x... flow=call->0x...
...
stack memory: sp=0x... block=0x... ...
```

Cette vue est particulièrement utile pour suivre `fdiv.d` dans MiniBASIC et
pour vérifier qu’une routine ne détruit pas `x2`, les cadres de sauvegarde ou
les pointeurs décrits dans `docs/MEMORY_MAP.md`.

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

Le dialecte source utilise `;` comme préfixe de commentaire. Le caractère et
tout ce qui le suit sont ignorés, ce qui permet de commenter une instruction,
un label ou une ligne entière :

```text
source> ; commentaire pédagogique ignoré par l’assembleur guest
source> addi x1,x0,1 ; premier résultat observable
```

```text
rvmonitor> assemble-program 0x80010a30
source mode: enter integer/control, lui/auipc, ld/sd/fld/fsd, f arithmetic or fmv lines, finish with end
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
error [GUEST-ASM-008] source line 2: supports integer/control, ld/sd/fld/fsd, f arithmetic or fmv syntax
```

Le parseur invité accepte `addi`, `lui`, `auipc`, `beq`, `bne`, `jal`, `jalr`,
`lb/lh/lw/ld`, `lbu/lhu/lwu`, `sb/sh/sw/sd`, les opérations entières
`add/sub/sll/slt/sltu/xor/srl/sra/or/and` et les immédiats
`andi/ori/xori/slti/sltiu/slli/srli/srai`, ainsi que `ld`, `sd`,
`fadd.s` et `fadd.d`. Les branches et `jal` prennent une cible relative numérique ou un
label, éventuellement suivi de `+offset` ou `-offset`; `jalr` utilise la forme
`jalr rd,imm(rs1)`. Les instructions flottantes utilisent
`fadd.[s|d] fd,fs1,fs2` et acceptent éventuellement un mode d’arrondi numérique
`0..7` en quatrième opérande.
Le document source persistant accepte 4096 lignes de 128 caractères ; la
commande `assemble-program` accepte 9216 lignes, avec 1024 labels. Cette
capacité permet désormais d’assembler le payload MiniBASIC complet dans le
guest, sans augmenter la capacité du document édité ni consommer la pile M-mode.
Les
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

Pour lancer un programme assemblé dans le workspace, utiliser `run-at` avec
son adresse d’entrée. Cette commande réinitialise le contexte U-mode puis
saute à cette adresse ; elle constitue le premier chemin de payload utilisateur
chargé par le moniteur :

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
A
target exit status=0
```

`run-at` ne charge pas encore un fichier et ne fournit pas encore de
relocations ; il lance le programme déjà assemblé dans le workspace. Pour le
contrat détaillé et ses limites, voir [GUEST_PAYLOAD_ABI.md](GUEST_PAYLOAD_ABI.md).

### Consulter et corriger le source

Après un `assemble-program` réussi, le guest conserve jusqu’à 4096 lignes de
128 caractères du dernier programme et son adresse de chargement. Un payload
plus grand peut être assemblé et exécuté, mais son source n’est pas retenu
dans le buffer éditable. `source` affiche le document,
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

Pour exporter l’image et le metadata dans un projet `RVPROJ01` :

```sh
$ cargo run -p luna-app -- \
    --guest-uart-port 12353 --project-out session.rvproj
guest project exported to session.rvproj (metadata source=... symbols=...)
```

L’application attend l’invite UART, exécute `snapshot save`, collecte les
blocs, contrôle le manifeste et écrit un fichier `RVSNAP01` déterministe.
Cette version exporte les régions mémoire et le nombre de lignes source ; les
registres, symboles et texte source ne sont pas encore sérialisés.
Le guest expose désormais le sous-format `RVMETA01` par
`snapshot metadata` puis `snapshot metadata dump <offset> <length>` ; son
intégration dans `RVPROJ01` est maintenant disponible à l’export. Un projet
peut être réinjecté avec :

```sh
$ cargo run -p luna-app -- \
    --guest-uart-port 12353 --project-in session.rvproj
guest project imported from session.rvproj
```

La copie complète des régions fixes reste lente sous QEMU/16550 ; le contrat
d’import est donc validé par tests de format et de transport borné.

Note de débit : le pilote configure nominalement 9600 bauds (diviseur UART
12), mais le chardev TCP de QEMU n’attend pas le temps de transmission de
chaque bit. Les exports TCP observés sont donc beaucoup plus rapides qu’une
liaison série physique ; ils restent limités par les commandes, les
aller-retours et l’encodage hexadécimal des dumps.

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

## 4. MiniBASIC-RV : progression guidée

`basic` lance MiniBASIC-RV en U-mode. Le moteur, son magasin de lignes, son
parseur, ses variables et ses calculs sont exécutés dans la machine cible ; le
moniteur ne fournit que les services console et l’arrêt coopératif documentés
plus haut. Le parcours ci-dessous introduit une seule idée à la fois.

### 4.0 Ce que fait réellement `basic`

Lors du build, le script `scripts/build-minibasic-asm-payload.sh` assemble
`examples/minibasic-asm/payload-repl.rv` et produit une image de code ainsi
qu’une image de données. La commande `basic` copie ces images dans le workspace
et la région de données de la cible, réinitialise le contexte U-mode, puis
commence à `0x81000100`. Le parseur, le magasin de lignes, les calculs et les
appels UART sont donc exécutés par le payload RV64 dans la machine cible.

Pour rejouer directement le chemin indépendant du moniteur de commande :

```sh
$ bash scripts/test-guest-minibasic-basic-command.sh
```

`basic-rust` conserve temporairement l’ancien moteur lié dans l’ELF afin de
faciliter les comparaisons et les régressions. Il n’est pas utilisé par le
parcours normal.

### 4.0.1 Limites restantes du chargement

Le chargement automatique actuel est spécialisé au payload MiniBASIC et ne
constitue pas encore un chargeur général de fichiers utilisateur. Le dialecte
assembleur guest ne gère encore ni appels symboliques complets, ni relocations.
La conversion
automatique du désassemblage Rust en
source pédagogique ne produirait donc pas encore un programme maintenable.

La trajectoire retenue est :

1. figer l’ABI U-mode de MiniBASIC : `entry`, pile, console `ecall`, zone de
   variables, magasin BASIC, code de sortie et convention de faute ;
2. extraire le runtime minimal en modules indépendants et produire une carte
   des symboles/sections vérifiable avec `nm` et `objdump` ;
3. étendre l’assembleur guest aux labels, appels `jal`/`jalr`, `.text`, `.data`,
   `.bss`, constantes flottantes et au chargement atomique d’un payload ;
4. écrire progressivement le REPL, lexer, évaluateur D et contrôle de flot en
   assembleur accepté par ce dialecte, en utilisant le binaire Rust comme
   oracle différentiel temporaire ;
5. ajouter `load-program` ou `assemble-load` : assemblage dans le workspace,
   validation des bornes, symboles, pile et point d’entrée, puis lancement
   U-mode par adresse chargée ;
6. remplacer `basic` résident par un exemple chargé depuis le moniteur et
   conserver le chemin résident uniquement comme mode de secours/tests.

Le désassemblage du Rust compilé sera utilisé pour comprendre les séquences
RV64D, les appels de services et les conventions de pile, jamais comme
substitut silencieux à une source assembleur. Cette migration est un jalon
distinct : la présente démo prouve le moteur cible, mais ne doit pas être
présentée comme preuve du chargement dynamique.

### 4.1 Entrer en mode direct

```text
rvmonitor> basic

MiniBASIC-RV
READY> PRINT 2+3*4
14.000000
READY> PRINT (2+3)*4
20.000000
READY> PRINT 22/7
3.142857
```

La priorité est celle des mathématiques usuelles : multiplication et division
avant addition et soustraction ; les parenthèses forcent un autre groupement.
Les six décimales sont un format d’affichage V1, pas une conversion exécutée
par l’hôte.

### 4.2 Construire puis lister un petit programme

Une ligne précédée d’un numéro est stockée. La saisie suivante remplace la
ligne 20 ; `LIST` trie les lignes même si elles ont été entrées dans le
désordre.

```text
READY> 30 PRINT "DONE"
READY> 10 A=2
READY> 20 PRINT A*A
READY> LIST
10 A=2
20 PRINT A*A
30 PRINT "DONE"
READY> RUN
4.000000
DONE
```

Un numéro seul supprime la ligne : `READY> 20`. `NEW` efface ensuite le
programme et les variables. Cette convention reprend l’immédiateté des BASIC
à lignes numérotées sans chercher une compatibilité avec un dialecte précis.

### 4.3 Ajouter une boucle et observer TRACE

```text
READY> 10 FOR I=1 TO 3
READY> 20 PRINT I,I*I
READY> 30 NEXT I
READY> LIST
10 FOR I=1 TO 3
20 PRINT I,I*I
30 NEXT I
READY> TRACE ON
READY> RUN
[10]
[20]
1.000000 1.000000
[30]
[20]
2.000000 4.000000
[30]
[20]
3.000000 9.000000
[30]
READY> TRACE OFF
```

`FOR` utilise la pile FOR cible ; huit boucles imbriquées sont garanties en
V1. Un `STEP 0`, une pile pleine ou une expression invalide provoquent un
diagnostic stable et rendent la main à `READY>`.

### 4.4 Faire intervenir INPUT et le contrôle de flot

```text
READY> NEW
READY> 10 INPUT N
READY> 20 IF N<0 THEN 50
READY> 30 PRINT N*N
READY> 40 GOTO 10
READY> 50 END
READY> RUN
? 3
9.000000
? 4
16.000000
? -1
READY>
```

Une boucle `GOTO` peut être interrompue par le caractère Ctrl-C envoyé sur la
console. L'IRQ UART capture le caractère pendant l'exécution U-mode ; le
programme cible le consomme ensuite via `ecall` 5 et le moniteur ne tue donc
pas la machine QEMU. Une cible qui ne demande jamais de service console ne
peut pas encore être interrompue coopérativement par Ctrl-C.

### 4.5 Inspecter le calcul flottant dans le débogueur

Le calcul `I/3` du programme suivant passe par une fonction cible contenant
réellement `fdiv.d` :

```text
READY> NEW
READY> 10 I=1
READY> 20 X=I/3
READY> 30 PRINT X
READY> 40 END
READY> RUN
0.333333
```

Depuis l’hôte, retrouver l’adresse sans la coder en dur :

```sh
$ riscv64-linux-gnu-nm -n \
    target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    | awk '$3 == "minibasic_divide" { print $1; exit }'
```

Poser ensuite `break 0x...`, relancer `basic`, ressaisir le petit programme et
exécuter `RUN`. À l’arrêt :

```text
rvmonitor> disasm 0x... 12
rvmonitor> regs
rvmonitor> step
rvmonitor> regs
```

Le désassemblage doit afficher `fdiv.d`. Les registres `f0`–`f31` sont montrés
comme motifs binaires exacts et `fcsr` expose `frm` et `fflags`. Cette étape
relie une expression BASIC, une adresse symbolique, une instruction RV64D et
le résultat observable dans le contexte cible.

### 4.6 Exemple traditionnel : Sumerian Granary

L’exemple est inspiré de l’expérience de *Hammurabi*, un jeu de gestion textuel
attribué à la tradition de *BASIC Computer Games*. La page de référence le
présente comme une version simplifiée de *The Sumerian Game* et conserve une
structure de rapports annuels, décisions de grain et bilan final
([listing et présentation de Hammurabi](https://basic-code.bearblog.dev/hammurabi/)).
Le code ci-dessous est une adaptation pédagogique originale, volontairement
plus petite et compatible avec MiniBASIC-RV V1 ; il ne reproduit pas le listing
de cette page.

Le roi gère trois années. Une acre coûte dix mesures de grain, une acre plantée
rapporte trois mesures et chaque habitant consomme deux mesures. Les réponses
doivent être saisies dans cet ordre à chaque année : acres achetées, acres
plantées, grain distribué.

```basic
10 PRINT "SUMERIAN GRANARY"
20 A=100
30 G=300
40 P=20
50 Y=0
60 PRINT "YEAR",Y,"ACRES",A,"GRAIN",G
70 Y=Y+1
80 PRINT "BUY ACRES"
90 INPUT Q
100 IF Q<0 THEN 80
110 IF Q*10>G THEN 80
120 A=A+Q
130 G=G-Q*10
140 PRINT "PLANT ACRES"
150 INPUT Q
160 IF Q<0 THEN 140
170 IF Q>A THEN 140
180 IF Q*2>G THEN 140
190 G=G-Q*2
200 G=G+Q*3
210 PRINT "FEED GRAIN"
220 INPUT Q
230 IF Q<0 THEN 210
240 IF Q>G THEN 210
250 G=G-Q
260 IF Q<P*2 THEN 300
270 PRINT "THE PEOPLE ARE FED"
280 IF Y<3 THEN 60
290 GOTO 400
300 PRINT "STARVATION"
310 IF Y<3 THEN 60
320 GOTO 400
400 PRINT "FINAL GRAIN",G
410 END
```

Séquence de démonstration reproductible :

```text
RUN
? 0
? 20
? 40
? 0
? 20
? 40
? 0
? 20
? 40
```

La sortie doit contenir trois rapports annuels, trois fois
`THE PEOPLE ARE FED`, puis `FINAL GRAIN` avec `240.000000`. Les valeurs ne
sont pas préenregistrées : elles résultent des opérations binary64 du guest.
Pour voir le chemin de contrôle, activer `TRACE ON` avant `RUN`; pour relier
le texte aux octets, utiliser `DUMP`, puis revenir au moniteur et examiner la
zone indiquée par `memory <adresse> <longueur>` en hexadécimal/ASCII.

L’exemple illustre aussi les limites assumées : pas de tableaux, de chaînes
variables, de hasard, de sous-programmes, de `INT`, de `OR`, de labels ni de
séparateur `:` en V1. Cette réduction rend chaque décision visible dans le
moniteur et conserve l’esprit rapport annuel / allocation / conséquences du
BASIC historique.

### 4.7 Jeu final : HAMMURABI-RV

Voici la version finale du parcours. Elle reste un programme MiniBASIC V1
réellement saisissable : les noms de variables longs sont volontairement
utilisés pour rendre l’état visible dans le moniteur, sans `:` ni fonctions
cachées, et uniquement avec les instructions documentées plus haut. Elle
reprend les éléments structurants de *Hammurabi* — rapport annuel, population,
terres, grain, décision du joueur, famine et bilan — sans recopier le listing
de référence. La page de référence décrit cette adaptation de *The Sumerian
Game* et explique ses choix de variables et de décisions
([Hammurabi.BAS](https://basic-code.bearblog.dev/hammurabi/)).

Les variables sont volontairement descriptives : `REGNALYEAR` année,
`CITIZENS` population, `HOLDINGS` acres, `CORNSTOCK` grain, `QUANTITY`
quantité saisie, `HARVESTED` récolte, `CITIZENFED` personnes nourries,
`MORTALITY` morts de faim et `OVERALLDEATH` total des morts. Leurs longueurs
varient de 8 à 10 caractères : `LIST`, `TRACE ON`, `DUMP` et les registres
permettent de suivre ces états sans quitter le programme cible.

Dans cette version assembleur, un identifiant long ne doit pas commencer par
le préfixe d’une instruction ou d’un raccourci historique dont le dispatcher
est prioritaire (`G` pour `GOTO`, `E` pour `END`, `F` pour `FOR`, `X`/`Y` pour
les variables courtes, etc.). `CORNSTOCK` et `REGNALYEAR` sont choisis pour
rester descriptifs tout en passant par l’affectation générique.

```basic
10 PRINT "HAMMURABI-RV"
20 CITIZENS=95
30 HOLDINGS=1000
40 CORNSTOCK=2800
50 REGNALYEAR=0
60 OVERALLDEATH=0
70 MORTALITY=0
80 REGNALYEAR=REGNALYEAR+1
90 PRINT "YEAR",REGNALYEAR,"PEOPLE",CITIZENS,"ACRES",HOLDINGS,"GRAIN",CORNSTOCK
100 PRINT "LAND PRICE 10 GRAIN PER ACRE"
110 PRINT "ACRES TO BUY (NEGATIVE TO SELL)"
120 INPUT QUANTITY
130 IF QUANTITY<0 THEN 180
140 IF QUANTITY*10>CORNSTOCK THEN 110
150 HOLDINGS=HOLDINGS+QUANTITY
160 CORNSTOCK=CORNSTOCK-QUANTITY*10
170 GOTO 220
180 QUANTITY=0-QUANTITY
190 IF QUANTITY>HOLDINGS THEN 110
200 HOLDINGS=HOLDINGS-QUANTITY
210 CORNSTOCK=CORNSTOCK+QUANTITY*10
220 PRINT "ACRES TO PLANT"
230 INPUT QUANTITY
240 IF QUANTITY<0 THEN 220
250 IF QUANTITY>HOLDINGS THEN 220
260 IF QUANTITY*2>CORNSTOCK THEN 220
270 CORNSTOCK=CORNSTOCK-QUANTITY*2
280 HARVESTED=QUANTITY*3
290 CORNSTOCK=CORNSTOCK+HARVESTED
300 PRINT "BUSHELS TO FEED"
310 MORTALITY=0
320 INPUT QUANTITY
330 IF QUANTITY<0 THEN 300
340 IF QUANTITY>CORNSTOCK THEN 300
350 CORNSTOCK=CORNSTOCK-QUANTITY
360 CITIZENFED=QUANTITY/2
370 IF CITIZENFED>=CITIZENS THEN 400
380 MORTALITY=CITIZENS-CITIZENFED
390 CITIZENS=CITIZENFED
400 OVERALLDEATH=OVERALLDEATH+MORTALITY
410 PRINT "HARVEST",HARVESTED,"STARVED",MORTALITY
420 IF MORTALITY*2>CITIZENS THEN 500
430 IF REGNALYEAR<5 THEN 80
440 GOTO 600
500 PRINT "REVOLT"
510 GOTO 600
600 PRINT "FINAL STARVED",OVERALLDEATH,"GRAIN",CORNSTOCK
610 END
```

Pour une partie prudente, saisir `0`, `20`, puis `190` à chaque année : ne
pas acheter, planter 20 acres, distribuer 190 mesures. Pour expérimenter,
acheter ou vendre des terres, planter davantage, puis observer l’effet d’une
distribution insuffisante. Une entrée négative ou trop grande ramène à la
question correspondante ; une année où `CITIZENFED<CITIZENS` conserve la famine
dans `MORTALITY` et alimente le bilan `OVERALLDEATH`.

Une séance pédagogique recommandée est :

```text
READY> NEW
READY> 10 PRINT "HAMMURABI-RV"
...
READY> LIST
READY> TRACE ON
READY> RUN
```

Pendant l’exécution, placer un breakpoint sur `minibasic_divide` pour observer
`QUANTITY/2`. Après le retour à l’invite, `DUMP` montre chaque variable longue,
son motif binary64 et sa valeur fixe ; le moniteur permet ainsi de comparer
trois niveaux au même moment : la ligne BASIC (`360 CITIZENFED=QUANTITY/2`), l’instruction `fdiv.d` et les
bits du registre flottant. Après une famine volontaire, utiliser `snapshot
save`, modifier une décision, puis `snapshot restore` pour rejouer la partie.

Le programme est assez compact pour être recopié manuellement, mais assez
riche pour donner envie de modifier les règles : changer le prix du terrain,
le rendement, la durée du règne ou le seuil de révolte constitue une suite
d’exercices naturelle. Les extensions du listing original — noms de variables
longs, `INT`, hasard, invites `INPUT`, labels, `OR` et séparateurs `:` — sont
laissées comme exercices de conception future, pas introduites implicitement
dans le langage V1.

## 5. Session complète reproductible

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

## 6. Ce qui n’est pas encore disponible dans ce mode

Les fonctions suivantes ne sont pas encore disponibles dans le binaire
exécuté dans QEMU :

```text
history, conditions de breakpoint, persistance directe sur système de fichiers
```

`watch`, `rwatch`, `awatch`, `project-save`, `project-load`, `snapshot` et
`restore` sont disponibles dans le guest, mais les snapshots restent
volatils et limités aux régions documentées. Les expressions générales, les
macros, les directives dans le source assembleur et l’import du moteur
MiniBASIC comme source assembleur restent à venir, en conservant le moniteur
M-mode et le programme cible U-mode séparés. Les directives `data` documentées
plus haut sont déjà disponibles pour écrire une valeur isolée en mémoire.

## 7. Différence avec les deux autres parcours

| Parcours | Binaire qui exécute le moniteur | Transport | État cible actuel |
|---|---|---|---|
| Simulateur interne | `luna-app` sur l’hôte | aucun | Machine Rust déterministe |
| Console QEMU hôte | `luna-app` sur l’hôte | GDB RSP/TCP | QEMU distant, registres entiers + PC |
| Guest prioritaire | `luna-guest-monitor` dans QEMU M-mode | UART virtuelle | U-mode sur le même hart |

Le tutoriel général [TUTORIAL.md](TUTORIAL.md) couvre les deux premiers
parcours. Le présent document est la référence pour le troisième.
