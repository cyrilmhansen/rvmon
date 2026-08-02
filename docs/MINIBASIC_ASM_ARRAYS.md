# MiniBASIC-RV : tableaux dans le payload assembleur

Cette note décrit la tranche exécutée dans `examples/minibasic-asm/payload-repl.rv`.
Les opérations et les données appartiennent à la machine RV64 cible ; le
moniteur hôte ne connaît que les octets transmis par la console.

## Syntaxe disponible

Les tableaux numériques et de chaînes utilisent un nom d'une lettre (`A` à
`Z`) et une dimension fixe :

```basic
DIM B(3)
B(1)=7
PRINT B(1)

DIM A$(2)
I=1
A$(I)="DIRECT"
PRINT A$(I)
```

Un indice peut être un entier décimal ou une variable numérique simple. Les
indices sont zéro-based et la dimension `N` réserve `0..N`, soit `N+1`
cellules. Les expressions d'indice composées et les tableaux multidimensionnels
ne sont pas encore exposés par cette tranche.

Les mêmes formes sont acceptées dans les lignes numérotées et pendant `RUN`.
Une dimension supérieure à 31, un indice hors limites, un nom mal formé ou une
affectation sans chaîne terminée par `"` produit `ERR` et rend la main à
l'invite sans écrire la cellule.

## Layout mémoire cible

Les offsets sont relatifs à `data` (`x8` dans le payload) :

| Région | Offset | Format |
| --- | ---: | --- |
| dimensions numériques | `+584` | 26 mots de 64 bits, `N+1` |
| dimensions chaînes | `+800` | 26 mots de 64 bits, `N+1` |
| variables scalaires | `+768` | 26 `binary64` |
| variables chaînes | `+4096` | 26 cellules de 128 octets |
| cellules numériques | `+8192` | 26 × 32 cellules de 8 octets |
| cellules chaînes | `+16384` | 26 × 32 cellules de 128 octets |

Une cellule chaîne contient la longueur `u64` à son offset zéro, suivie de
120 octets ASCII à l'offset huit. La fonction de résolution renvoie un pointeur
sur les caractères et la longueur dans `x11`, conformément à l'appel
`write_buffer` du payload. L'assignation vérifie la capacité avant toute
mutation.

Les tableaux numériques restent isolés de la table des scalaires. Les tableaux
de chaînes utilisent une région différente des variables chaînes, ce qui évite
qu'un `DIM` ou une écriture indexée ne modifie un nom scalaire homonyme.

## Chemins d'exécution

- `DIM` choisit le parseur numérique ou chaîne selon la présence de `$` après
  le nom, puis initialise les longueurs de cellules à zéro dans la cible.
- L'affectation directe et l'affectation dans une ligne numérotée passent par
  le même résolveur de cellule.
- `PRINT A$(I)` est distingué de `PRINT A$` avant l'appel au résolveur de
  variable simple.
- La conversion d'un indice variable passe par `fcvt.l.d` dans la cible ; un
  indice négatif ou non représentable échoue ensuite au contrôle de borne.

## Preuve reproductible

```sh
bash scripts/test-guest-runtime-asm-repl-string-array.sh
bash scripts/test-guest-runtime-asm-repl-string-array-error.sh
bash scripts/test-guest-runtime-asm-repl-print-mixed.sh
bash scripts/test-guest-runtime-asm-repl-array-table.sh
bash scripts/test-guest-runtime-asm-repl-array-index-variable.sh
```

Le premier test vérifie une affectation/lecture directe, une utilisation dans
un programme et une lecture par variable. Le second isole le diagnostic de
borne hors tableau ; le troisième vérifie `PRINT` avec mélange de littéraux,
variables chaînes, tableaux de chaînes et expressions numériques. Les deux
derniers protègent respectivement les tableaux numériques multi-noms et les
indices variables numériques.
