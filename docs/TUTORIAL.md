# Tutoriel rapide RVMonitor

Ce tutoriel présente les deux chemins disponibles : le moniteur interne avec
sa machine RV64 déterministe, puis la console hôte connectée à
`qemu-system-riscv64` par GDB Remote Protocol.

## 1. Premier pas dans le moniteur interne

Depuis la racine du dépôt :

```text
$ cargo run -p luna-app -- --interactive
RVMonitor interactive; type 'help' for commands
rvmonitor> assemble addi x1,x0,1
loaded 4 bytes at 0x0000000000000000
rvmonitor> step
0x0000000000000000: 00100093  addi x1,x0,1 -> pc=0x0000000000000004
rvmonitor> regs
```

La même démonstration peut être rejouée sans prompt avec le fichier fourni :

```text
$ cargo run -p luna-app -- --script examples/internal-first-step.rv
```

La commande `regs` affiche les registres entiers, les registres flottants et
`fcsr`. La mémoire cible est isolée de la mémoire du processus hôte.

## 2. Charger un petit programme et le suivre

Un label peut être utilisé même dans une session d’une ligne :

```text
rvmonitor> assemble-program _start: addi x1,x0,7
loaded 4 bytes at 0x0000000000000000; 1 symbol(s)
rvmonitor> symbols
symbols:
  0x0000000000000000 _start
rvmonitor> disasm _start 1
0x0000000000000000: 00700093  addi x1,x0,7
rvmonitor> break _start
breakpoint #1 set at 0x0000000000000000
rvmonitor> run 4
stopped: breakpoint #1 at pc=0x0000000000000000
rvmonitor> continue 1
rvmonitor> history
```

`break` est logique et s’arrête avant l’instruction. `continue` franchit le
breakpoint courant une fois. Pour un programme multi-ligne, fournir le texte
avec des retours à la ligne à l’API `BackendConsole::execute` ou utiliser
`--script` avec un fichier de commandes.

## 3. Mémoire, watchpoint et annulation

```text
rvmonitor> reset
rvmonitor> edit 0x20 44 33 22 11
edited 4 byte(s) at 0x0000000000000020
rvmonitor> memory 0x20 4
0x0000000000000020: 44 33 22 11                                     |D3".|
rvmonitor> undo
undid 4 byte(s) at 0x0000000000000020
```

Un watchpoint est déclenché lorsque le backend fournit un `MemoryAccess` :

```text
rvmonitor> assemble lw x1,32(x0)
rvmonitor> rwatch 0x20 4
watchpoint #1 set (read) at 0x0000000000000020 width=4
rvmonitor> run 1
stopped: watchpoint #1 at pc=0x0000000000000004; read addr=0x0000000000000020 width=4
```

`history` conserve un nombre borné d’étapes et affiche le texte désassemblé,
l’adresse avant/après et l’accès mémoire lorsqu’il existe.

## 4. Sauvegarder une session

Le moniteur interne historique sauvegarde un projet complet avec
`project-save`. La console backend-générique sauvegarde une session
portable : source, symboles, vue, breakpoints et watchpoints.

```text
rvmonitor-qemu> project-save /tmp/demo.rvs
session saved (...; target state unchanged)
rvmonitor-qemu> project-load /tmp/demo.rvs
session loaded (...; target registers and memory unchanged)
```

La session backend-générique ne promet donc pas de restaurer les registres ou
la mémoire d’une cible distante. Les backends qui exposent explicitement le
contrat d’instantané peuvent en revanche utiliser `snapshot` et `restore` :
le backend Machine restaure l’état complet de façon déterministe, tandis que
le backend QEMU indique actuellement que cette capacité n’est pas disponible.

```text
rvmonitor> snapshot /tmp/machine.rvt
snapshot saved (...)
rvmonitor> restore /tmp/machine.rvt
snapshot restored
```

## 5. Connecter QEMU

Construire l’image bare-metal puis lancer QEMU arrêté sur son premier PC :

```text
$ cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf
$ qemu-system-riscv64 -M virt -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor \
    -nographic -S -gdb tcp::12351
```

Dans un second terminal :

```text
$ cargo run -p luna-app -- --qemu-port 12351
RVMonitor QEMU backend on 127.0.0.1:12351; type 'help' for commands
rvmonitor-qemu> regs
rvmonitor-qemu> memory 0x80000000 16
rvmonitor-qemu> assemble-program _start: addi x1,x0,1
rvmonitor-qemu> symbols
rvmonitor-qemu> disasm _start 1
rvmonitor-qemu> break _start
rvmonitor-qemu> run 1
```

Le backend QEMU actuellement observé expose les registres entiers et le PC,
mais pas les événements d’accès mémoire ni les registres flottants dans son
paquet `g`. `regs` affiche donc ces capacités explicitement et `rwatch` est
refusé avec un diagnostic au lieu de simuler un arrêt inexistant.

Pour reproduire automatiquement cette session :

```text
$ cargo run -p luna-app -- --qemu-port 12351 --script examples/qemu-session.rv
```

Le script d’intégration complet, qui lance aussi QEMU et vérifie les sorties,
est `bash scripts/test-qemu-gdb-backend.sh`.

## 6. Diagnostic rapide

- `Connection refused` : vérifier que QEMU est lancé avec `-gdb tcp::12351`
  et que le port choisi est identique à `--qemu-port`.
- `backend does not expose memory access events` : limitation attendue du
  layout RSP QEMU actuel ; utiliser le moniteur interne pour tester les
  watchpoints.
- `target operation failed` : le backend a rejeté l’accès mémoire ou la
  commande RSP ; consulter le message et l’adresse affichée.
