# ABI des payloads U-mode chargés par RVMonitor

Cette interface constitue la première tranche du futur chargement dynamique.
Elle est volontairement plus petite que l’ABI complète d’un système d’exploitation.

## Contrat V1

Un payload est assemblé dans le workspace cible `[0x81000000,0x81010000)`.
Son point d’entrée doit être aligné sur quatre octets et rester dans cette
région. Le moniteur :

- remet `x0..x31` à zéro ;
- initialise `x2` avec la pile U-mode guest dédiée ;
- initialise les registres flottants avec le motif NaN-box neutre et `fcsr=0` ;
- positionne `pc` et `mepc` sur l’entrée ;
- reprend l’exécution en U-mode sur le hart unique.

Le payload ne doit pas supposer que les adresses de l’image Rust sont stables.
Il utilise les services `ecall` documentés dans [TUTORIAL-GUEST.md](TUTORIAL-GUEST.md) :
`a7=1` écrit un octet, `a7=2` lit un octet, `a7=3` termine et `a7=5` poll une
interruption. Les arguments sont dans `a0` et `a1`.

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
