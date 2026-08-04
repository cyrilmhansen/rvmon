# Pilote UART guest 16550A

Le guest de référence utilise le 16550A de la machine QEMU `virt` à
`0x10000000` et l'IRQ PLIC 10. Le pilote est volontairement limité à ce
périphérique et au hart M-mode unique du profil actuel.

## Initialisation

Le pilote configure 8N1, le diviseur 12 (base QEMU 115200, donc 9600 bauds,
valeur de reset conservée), active et vide les FIFO RX/TX, sélectionne le
seuil RX d'un octet et positionne DTR, RTS et OUT2. OUT2 est nécessaire pour
qu'un 16550 physique transmette son signal d'interruption.

Le FIFO matériel QEMU fait 16 octets. Il ne constitue pas un tampon de
transfert fiable à lui seul : le pilote le vide dans une file logicielle fixe
de 4096 octets. Un octet `0x03` est retiré de la file normale et mémorisé comme
demande Ctrl-C ; les erreurs LSR sont comptées et exposées par `info uart`.

## Interruptions et contexte

Quand le programme cible U-mode s'exécute, IER.RLSI/RDI, `mie.MEIE` et le
contexte M-mode du PLIC sont actifs. Le handler claim l'IRQ 10, vide le FIFO,
complete l'IRQ puis restaure le même `TargetContext` et reprend par `mret`.

Quand le moniteur traite une commande en M-mode, IER et `mie.MEIE` sont
désactivés. Le trap entry actuel sauvegarde le contexte de la cible arrêtée ;
une interruption imbriquée pendant cette phase écraserait ce contexte. Le
fallback de polling est donc exécuté dans `put`, `get` et `try_get` : chaque
émission peut drainer le FIFO sans réentrer le trap.

Cette séparation est une propriété de sûreté, pas une optimisation. Une cible
qui ne demande jamais `ecall` ne peut pas encore transformer seule la demande
Ctrl-C en arrêt de débogueur ; le caractère est capturé mais la décision reste
coopérative au niveau du programme cible.

## ABI console cible

Les services restent ceux de `GUEST_PAYLOAD_ABI.md` : `a7=1` écrit un octet,
`a7=2` lit bloquant, `a7=5` lit sans bloquer et retourne zéro si la file est
vide. Le moniteur hôte ne voit ni analyse BASIC ni résultat numérique cible.

## Débit et mesure

Le diviseur actuel vaut 12 sur une base QEMU de 115200, soit 9600 bauds
nominalement. En 8N1, cela représente environ 960 octets/s sur une liaison
physique qui respecterait le temps de transmission UART. Le diviseur 1
correspondrait à 115200 bauds, soit environ 11520 octets/s utiles.

Le backend `-serial tcp:...` de QEMU n'impose cependant pas ce délai bit par
bit : il remet les octets au chardev aussi vite que le socket les accepte. Le
test `scripts/test-guest-snapshot-export.sh` a observé 27,6 secondes pour deux
exports complets (workspace 65536 octets et data 1048576 octets chacun), avec
compilation incrémentale et handshakes inclus. Cette mesure n'est donc pas un
débit UART physique ; elle correspond approximativement à 80--160 Kio/s selon
que l'on compte les octets mémoire ou leur représentation hexadécimale.

Les commandes `snapshot dump` encodent actuellement chaque octet sur deux
caractères hexadécimaux. Le débit utile est donc inférieur au débit du socket.
Une optimisation future pourrait utiliser un dump binaire symétrique, mais
elle n'est pas nécessaire au contrat V1.

## Vérification

`scripts/test-guest-uart.sh` vérifie l'initialisation et les compteurs sans
entrée perdue au démarrage. `scripts/test-guest-uart-irq.sh` exécute un payload
U-mode sans `ecall` pendant l'arrivée de Ctrl-C et exige un service IRQ PLIC
observable. `scripts/test-minibasic.sh` exerce la réception
pendant une sortie interactive, les entrées `INPUT` et Ctrl-C. Les tests
QEMU utilisent la version locale annoncée par `qemu-system-riscv64 --version`.

Références de conception : [QEMU `serial.c` v11.0.2](https://gitlab.com/qemu-project/qemu/-/raw/v11.0.2/hw/char/serial.c),
[QEMU `virt.h` v11.0.2](https://gitlab.com/qemu-project/qemu/-/raw/v11.0.2/include/hw/riscv/virt.h)
et le chemin de réception Linux 8250 utilisé comme référence comportementale,
non comme code copié.
