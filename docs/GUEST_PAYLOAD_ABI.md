# ABI des payloads U-mode chargés par RVMonitor

Cette interface constitue la première tranche du futur chargement dynamique.
Elle est volontairement plus petite que l’ABI complète d’un système d’exploitation.

## Contrat V1

Le contrat est identifié par `RVMPAY01`. La cible de référence est
`RV64ILP32D-MON-1`, little-endian, mono-hart, avec exécution du payload en
U-mode supervisée par le moniteur M-mode.

Un payload est assemblé dans le workspace cible `[0x81000000,0x81010000)`.
Son point d’entrée doit être aligné sur quatre octets et rester dans cette
région. Le moniteur :

- remet `x0..x31` à zéro ;
- initialise `x2` avec la pile U-mode guest dédiée ;
- initialise les registres flottants avec le motif NaN-box neutre et `fcsr=0` ;
- positionne `pc` et `mepc` sur l’entrée ;
- reprend l’exécution en U-mode sur le hart unique.

La pile U-mode dédiée fait 8192 octets ; son adresse exacte est fournie au
payload par `x2` et ne doit pas être supposée par le code. La pile privée
M-mode du moniteur fait 65536 octets. Les deux piles sont distinctes du
workspace et de la région de données.

La région de données cible est `[0x82000000,0x82100000)` et fait 1 MiB. Les
accès du payload à ces deux régions sont des adresses RV64 ordinaires ; aucune
traduction de pointeur ILP32 ni alias implicite n’est appliquée par ce contrat.

La commande guest `info payload` expose ces valeurs depuis l’image exécutée,
afin que la séance de démonstration ne dépende pas d’une adresse recopiée dans
la documentation.

Le payload ne doit pas supposer que les adresses de l’image Rust sont stables.
Il utilise les services `ecall` documentés dans [TUTORIAL-GUEST.md](TUTORIAL-GUEST.md) :
`a7=1` écrit un octet, `a7=2` lit un octet, `a7=3` termine et `a7=5` poll une
interruption. `a7=4` écrit une plage bornée de RAM cible. Les arguments sont
dans `a0` et `a1` ; les services 1, 2 et 5 renvoient leur résultat dans `a0`.
Le service 3 ne reprend pas le payload et rend le contrôle au moniteur avec le
statut `a0`; une erreur de service produit un diagnostic et n’écrit pas dans
la cible.

## Commande actuelle

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

Une fixture complète et reproductible est fournie dans
[`examples/minibasic-payload-skeleton.rv`](../examples/minibasic-payload-skeleton.rv).
Elle montre la séquence actuelle : écriture de données dans la région cible,
labels `entry`/`payload_exit`, `lui`, `ld`/`sd`, `jal`, appels ecall, listing
des symboles, désassemblage, `run-at` et inspection mémoire. Le test QEMU
correspondant est `bash scripts/test-guest-payload-skeleton.sh`.

Cette fixture n’est pas encore un MiniBASIC : elle constitue le squelette
assembleur utilisateur et son oracle de chargement. Le dialecte guest ne
possède pas encore de directive de section `.text`/`.data` dans une même
session ; la donnée est donc déposée explicitement par `data` avant
l’assemblage du texte. Le futur chargeur `assemble-load` devra réunir ces
sections atomiquement.

`run-at` est un lanceur de payload, pas encore un chargeur de fichier : le
source est toujours saisi par `assemble-program`. La commande valide l’adresse
et réinitialise le contexte avant le saut ; une entrée hors workspace, non
alignée ou non hexadécimale produit `GUEST-RUNAT-001` ou `GUEST-RUNAT-002`.

## Limites et prochaine étape

Le payload doit encore être assemblé par le parseur guest limité. Il n’existe
pas encore de format ELF utilisateur, de relocation, de sections `.text`/
`.data` séparées, de copie atomique depuis un fichier hôte ou de table de
symboles persistante propre au payload. Ces fonctions appartiennent à la
tranche `assemble-load`, après validation de ce contrat minimal.
