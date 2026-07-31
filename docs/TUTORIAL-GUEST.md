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
minimal. Il fournit déjà l’inspection des registres, la lecture mémoire et
des commandes `assemble` et `assemble-program` limitées à `addi`. Les vues
les directives et les snapshots restent à porter. Le programme U-mode de
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
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -nographic
```

La sortie initiale ressemble à ceci :

```text
RVMonitor 4B M-mode
target: RV64 ILP32D U-mode, hart=1, C=off
capabilities: I M F D Zicsr Zifencei
target workspace: 0x000000008001.... ..0x000000008001....
target: entering U-mode
trap: breakpoint pc=0x000000008000....
rvmonitor>
```

Le premier `ebreak` du programme U-mode arrête volontairement la cible. Le
PC exact dépend du placement final de l’image ; il faut utiliser l’adresse
affichée par QEMU ou celle calculée avec `nm`, jamais supposer une adresse
fixe dans un tutoriel automatisable.

## 3. Commandes disponibles dans le guest

La commande `help` affiche la grammaire actuellement implémentée :

```text
rvmonitor> help
help/? regs/registers memory <addr> <length> assemble <addr> addi <rd>,<rs1>,<imm> step/s continue/c break <addr> delete <n> info break quit/q
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
doit rester dans la RAM cible `[0x80000000, 0x80020000)`. Les adresses MMIO,
les dépassements et les longueurs nulles sont refusés.

### Assembler et exécuter `addi`

Le premier cycle source → encodage → RAM → pas-à-pas dans le guest est :

```text
rvmonitor> assemble 0x80001000 addi x1,x0,1
assembled addi at 0x0000000080001000 = 0x0000000000100093
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
source mode: enter addi lines, finish with end
source> _start:
source> addi x1,x0,1
source> next:
source> addi x1,x1,2
source> end
assembled program: 2 instruction(s) at 0x0000000080010a30
rvmonitor> symbols
symbols:
  0x0000000080010a30 _start
  0x0000000080010a34 next
rvmonitor> disasm _start 2
0x0000000080010a30: 0000000000100093 <_start>  addi x1,x0,1
0x0000000080010a34: 0000000000208093 <next>  addi x1,x1,2
rvmonitor> step
trap: breakpoint pc=0x0000000080010a34
rvmonitor> step
trap: breakpoint pc=0x0000000080010a38
rvmonitor> regs
... x1=0x0000000000000003 ...
```

Le tampon accepte au maximum 16 lignes de 96 caractères et huit labels. Les
labels ASCII (`a-z`, chiffres, `_`, `.` et `$`) occupent l’adresse courante
sans produire d’instruction. L’adresse de l’exemple doit correspondre à une
zone RAM libre, qui ne recouvre ni le code, ni la pile, ni les données du
moniteur ; le smoke test la calcule à partir de `_target_workspace_start` avec
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

Le pas-à-pas actuel couvre le flux de contrôle nécessaire au programme de
démonstration. Une instruction de contrôle non reconnue provoque un
diagnostic et laisse le moniteur en M-mode.

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

Le smoke test fourni automatise une session UART et vérifie les sorties :

```text
$ bash scripts/test-guest-monitor.sh
guest monitor QEMU smoke test passed
```

Le script :

1. construit l’ELF invité ;
2. calcule une adresse de breakpoint avec `riscv64-linux-gnu-nm` ;
3. démarre QEMU avec `-bios none`, `-kernel` et `-nographic` ;
4. envoie `help`, `regs`, `memory`, `break`, `info break`, `continue`,
   `step`, `delete` et `assemble-program` sur l’UART ;
5. vérifie les modifications de `x1`, l’encodage `addi` et les diagnostics de
   trap.

Pour observer la même session manuellement, démarrer QEMU dans un terminal,
puis saisir les commandes une par une dans son terminal UART.

## 5. Ce qui n’est pas encore disponible dans ce mode

Les commandes suivantes appartiennent aujourd’hui au moniteur hôte ou au
simulateur interne, pas encore au binaire exécuté dans QEMU :

```text
edit, undo, watch, rwatch, history, project-save, project-load, snapshot,
restore
```

Le cycle d’une ligne et le source buffer multi-ligne existent désormais pour
`addi`, les labels et le désassemblage des mots 32 bits. La prochaine étape est
la prise en charge des expressions, directives et des instructions de contrôle
avec résolution de labels, en conservant le moniteur M-mode et le programme
cible U-mode séparés.

## 6. Différence avec les deux autres parcours

| Parcours | Binaire qui exécute le moniteur | Transport | État cible actuel |
|---|---|---|---|
| Simulateur interne | `luna-app` sur l’hôte | aucun | Machine Rust déterministe |
| Console QEMU hôte | `luna-app` sur l’hôte | GDB RSP/TCP | QEMU distant, registres entiers + PC |
| Guest prioritaire | `luna-guest-monitor` dans QEMU M-mode | UART virtuelle | U-mode sur le même hart |

Le tutoriel général [TUTORIAL.md](TUTORIAL.md) couvre les deux premiers
parcours. Le présent document est la référence pour le troisième.
