# Pointeurs RV64ILP32 dans RVMonitor

Ce document fixe le comportement observable du profil local
`RV64ILP32D-MON-1`. Il distingue l’ISA RV64, l’ABI expérimentale
RV64ILP32D et l’environnement d’exécution du moniteur.

## Règle courte

Une valeur de pointeur ABI occupe 32 bits en mémoire et est représentée par
une valeur sign-étendue à 64 bits lorsqu’elle se trouve dans un registre RV64.
La moitié haute du registre n’est donc pas un champ indépendant conservé par
les indirections : elle est recalculée à partir du bit 31 du pointeur.

La psABI de développement décrit les pointeurs RV64ILP32* comme des scalaires
32 bits sign-étendus à XLEN lorsqu’ils sont passés dans un registre ou sur la
pile, et marque cette famille comme expérimentale [R3, §2.6].

## Table d’indirection

Une table de pointeurs contient une entrée tous les 4 octets :

```text
base +  0 : 00 00 00 00       -> 0x00000000
base +  4 : ff ff ff 7f       -> 0x7fffffff
base +  8 : 00 00 00 80       -> 0x80000000
base + 12 : ff ff ff ff       -> 0xffffffff
```

Le chargement correct est :

```asm
lw t1,  0(base)       # t1 = 0x0000000000000000
lw t2,  4(base)       # t2 = 0x000000007fffffff
lw t3,  8(base)       # t3 = 0xffffffff80000000
lw t4, 12(base)       # t4 = 0xffffffffffffffff
```

Le test `luna-machine::loads_an_ilp32_pointer_table_with_four_byte_stride`
exécute exactement cette forme avec les quatre valeurs frontières.

## La moitié haute est-elle conservée ?

Non. `lw rd, offset(rs1)` écrit les 64 bits de `rd` avec le mot mémoire
32 bits sign-étendu. Si `rd` contenait auparavant
`0xdeadbeef_deadbeef`, cette valeur est entièrement remplacée ; aucun bit de
son ancienne moitié haute ne subsiste.

La sémantique RV64I normative est :

| Instruction | Accès mémoire | Valeur écrite dans `rd` / mémoire | Usage pour une table ILP32 |
|---|---:|---|---|
| `lw` | 4 octets | mot sign-étendu à 64 bits | chargement canonique d’un pointeur |
| `sw` | 4 octets | écrit les 32 bits faibles de `rs2` | stockage canonique d’un pointeur |
| `lwu` | 4 octets | mot zéro-étendu à 64 bits | représentation non canonique pour un pointeur ABI haut |
| `ld` | 8 octets | 64 bits contigus | ne charge pas une entrée ILP32 ; consomme deux entrées |

R1, RV64I §4.1.3, définit précisément `LW` comme sign-extension, `LWU`
comme zero-extension et `LD` comme chargement de 64 bits. La même section
précise que `SW` ne stocke que les bits faibles 32 bits.

Dans l’implémentation actuelle, `lw` et `sw` sont exécutés par
`luna-machine`. `lwu` et `ld` sont identifiés dans cette spécification mais
ne sont pas encore exposés par le décodeur/exécuteur V1 ; ils ne doivent donc
pas être utilisés pour conclure à une compatibilité déjà livrée.

## Indirection effective

Une séquence comme :

```asm
lw t1, 0(t0)
lw t2, 0(t1)
```

utilise bien la valeur RV64 sign-étendue contenue dans `t1` comme adresse du
second chargement. Elle ne transforme pas automatiquement
`0xffffffff80000000` en `0x0000000080000000`.

Il faut donc que l’environnement d’exécution mappe réellement l’adresse RV64
sign-étendue. Le moniteur ne suppose aucun alias implicite entre les fenêtres
basse et haute. Dans l’émulateur hôte actuel, `Memory` est un tableau plat
borné par sa taille ; une adresse haute sign-étendue est donc unmapped sauf si
un futur backend ajoute explicitement une traduction logique → physique.

Cette limite est intentionnelle : elle empêche de confondre la représentation
ABI d’un pointeur avec une politique de mapping mémoire.

## Arithmétique et canonicalisation

Une opération RV64 sur 64 bits ne recanonicalise pas automatiquement un
pointeur ILP32. Par exemple, partir de
`0x000000007fffffff` et exécuter `addi rd, rs1, 1` produit
`0x0000000080000000`, qui n’est pas la représentation sign-étendue canonique
de `0x80000000`.

Pour une opération arithmétique dans le domaine 32 bits suivie d’une valeur
de pointeur ABI, il faut une opération `W` qui signe-étend son résultat, par
exemple `addiw` dans le profil ISA complet. `luna-machine` ne modélise pas
encore `addiw`; cette limitation est distincte de la règle de chargement
`lw`.

## Conclusions de conception

1. Les entrées d’une table de pointeurs sont espacées de 4 octets, jamais de
   8 octets.
2. Un chargement de pointeur utilise `lw` et remplace le registre entier.
3. Un stockage de pointeur utilise `sw`; les 32 bits hauts du registre source
   sont ignorés.
4. `lwu`, `ld` et l’arithmétique 64 bits ne doivent pas être introduits comme
   raccourcis silencieux dans le code ABI.
5. La validité d’une adresse sign-étendue dépend du mapping de l’environnement,
   pas d’une conservation supposée de la moitié haute.

Références :

- [R1 — RV64I, §4.1.3 Load and Store Instructions](https://docs.riscv.org/reference/isa/unpriv/rv64.html#load-and-store-instructions)
- [R3 — RV64ILP32* Calling Convention, §2.6](https://riscv-non-isa.github.io/riscv-elf-psabi-doc/#rv64ilp32-calling-convention)
- [R3 — RV64ILP32* Named ABIs, §2.7](https://riscv-non-isa.github.io/riscv-elf-psabi-doc/#named-abis)
